use crate::audit;
use crate::checkpoint::{self, ResumeMode, RunCheckpoint};
use crate::util::{json_escape, now_unix, unique_path, write_text_atomic};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const CHUNK_SIZE: usize = 1024 * 1024;
const OVERLAP_SIZE: usize = 32;
const DEFAULT_MAX_BYTES: u64 = 512 * 1024 * 1024;
const DEFAULT_MAX_CANDIDATES: usize = 64;
const MIN_CARVE_BYTES: u64 = 16;
/// Signature-scan progress is checkpointed every this many 1 MiB chunks,
/// so a resume repeats at most ~64 MiB of scanning on huge images.
const CHECKPOINT_CHUNK_INTERVAL: usize = 64;

#[derive(Debug, Clone)]
pub struct CarveOptions {
    pub max_bytes: u64,
    pub max_candidates: usize,
}

impl Default for CarveOptions {
    fn default() -> Self {
        Self {
            max_bytes: DEFAULT_MAX_BYTES,
            max_candidates: DEFAULT_MAX_CANDIDATES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CarveHit {
    pub offset: u64,
    pub signature: String,
    pub extension: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CarvedArtifact {
    pub id: String,
    pub source_path: PathBuf,
    pub output_path: PathBuf,
    pub offset: u64,
    pub size_bytes: u64,
    pub signature: String,
    pub extension: String,
    pub sha256: String,
    pub validation_status: String,
    pub validation_note: String,
    pub duplicate_of: Option<String>,
}

impl CarvedArtifact {
    fn to_json(&self) -> String {
        format!(
            "{{\"schema_version\":3,\"event\":\"carve-file\",\"id\":\"{}\",\"artifact_type\":\"carved-candidate\",\"validation_status\":\"{}\",\"validation_note\":\"{}\",\"duplicate_of\":{},\"source_path\":\"{}\",\"output_path\":\"{}\",\"offset\":{},\"size_bytes\":{},\"signature\":\"{}\",\"extension\":\"{}\",\"sha256\":\"{}\"}}",
            json_escape(&self.id),
            json_escape(&self.validation_status),
            json_escape(&self.validation_note),
            audit::optional_string(self.duplicate_of.as_deref()),
            json_escape(&self.source_path.to_string_lossy()),
            json_escape(&self.output_path.to_string_lossy()),
            self.offset,
            self.size_bytes,
            json_escape(&self.signature),
            json_escape(&self.extension),
            json_escape(&self.sha256)
        )
    }
}

#[derive(Debug, Clone)]
pub struct CarveResult {
    pub source_path: PathBuf,
    pub carved_unix: u64,
    pub source_size_bytes: u64,
    pub artifacts: Vec<CarvedArtifact>,
    /// Artifacts replayed from an interrupted run's checkpoint (their
    /// output files already exist) instead of being re-carved.
    pub resumed_artifacts: usize,
    /// Byte offset the signature scan resumed from (0 = full scan ran).
    pub resumed_scan_offset: u64,
    pub warnings: Vec<String>,
    pub options: CarveOptions,
}

impl CarveResult {
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str("  \"schema_version\": 1,\n");
        out.push_str(&format!(
            "  \"source_path\": \"{}\",\n",
            json_escape(&self.source_path.to_string_lossy())
        ));
        out.push_str(&format!("  \"carved_unix\": {},\n", self.carved_unix));
        out.push_str(&format!(
            "  \"source_size_bytes\": {},\n",
            self.source_size_bytes
        ));
        out.push_str(&format!(
            "  \"artifact_count\": {},\n",
            self.artifacts.len()
        ));
        out.push_str(&format!(
            "  \"resumed_artifacts\": {},\n",
            self.resumed_artifacts
        ));
        out.push_str("  \"options\": {\n");
        out.push_str(&format!("    \"max_bytes\": {},\n", self.options.max_bytes));
        out.push_str(&format!(
            "    \"max_candidates\": {}\n",
            self.options.max_candidates
        ));
        out.push_str("  },\n");
        out.push_str("  \"warnings\": [\n");
        for (index, warning) in self.warnings.iter().enumerate() {
            out.push_str(&format!("    \"{}\"", json_escape(warning)));
            if index + 1 != self.warnings.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ],\n");
        out.push_str("  \"artifacts\": [\n");
        for (index, artifact) in self.artifacts.iter().enumerate() {
            out.push_str("    ");
            out.push_str(&artifact.to_json());
            if index + 1 != self.artifacts.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ]\n");
        out.push_str("}\n");
        out
    }
}

pub fn carve_file(
    case_dir: &Path,
    source_path: &Path,
    options: &CarveOptions,
    resume: ResumeMode,
) -> Result<CarveResult, String> {
    let source_path = source_path
        .canonicalize()
        .map_err(|err| format!("failed to canonicalize carve source: {err}"))?;
    if !source_path.is_file() {
        return Err(format!(
            "carve source is not a file: {}",
            source_path.display()
        ));
    }
    if options.max_bytes < MIN_CARVE_BYTES {
        return Err(format!("--max-bytes must be at least {MIN_CARVE_BYTES}"));
    }
    if options.max_candidates == 0 {
        return Err("--max-candidates must be greater than 0".to_string());
    }

    let metadata = std::fs::metadata(&source_path)
        .map_err(|err| format!("failed to read source metadata: {err}"))?;
    let source_size = metadata.len();
    // The fingerprint covers source identity AND the option set, so a
    // checkpoint from a different image or different --max-* values never
    // silently skips work.
    let fingerprint = carve_fingerprint(&source_path, &metadata, options);
    let mut checkpoint = RunCheckpoint::begin(
        &case_dir.join("db/carve-progress.jsonl"),
        "carve-file",
        &fingerprint,
        resume,
    )?;
    let prior = load_carve_checkpoint(&checkpoint)?;
    let resume_offset = prior.scan_offset;
    let carved_map = prior.carved;

    let hits = scan_signatures(
        &source_path,
        resume_offset,
        prior.hits,
        options.max_candidates,
        &mut checkpoint,
    )?;
    let mut warnings = Vec::new();
    if checkpoint.reset_stale() {
        warnings.push(
            "previous carve checkpoint was recorded for different inputs; started fresh"
                .to_string(),
        );
    }
    // `>` not `>=`: hits.len() == max means the scan stopped exactly at the
    // limit WITHOUT dropping a further candidate (dedup may shrink the
    // list below it), which is not a truncation warning.
    if hits.len() > options.max_candidates {
        warnings.push(format!(
            "candidate limit reached at {}; rerun with --max-candidates if needed",
            options.max_candidates
        ));
    }

    let mut artifacts = Vec::new();
    let mut resumed_artifacts = 0usize;
    let mut first_by_hash = HashMap::<String, String>::new();
    for hit in &hits {
        let next_offset = hits
            .iter()
            .map(|candidate| candidate.offset)
            .filter(|offset| offset > &hit.offset)
            .min()
            .unwrap_or(source_size);
        let available = next_offset.saturating_sub(hit.offset);
        let size_bytes = available.min(options.max_bytes);
        if size_bytes < MIN_CARVE_BYTES {
            warnings.push(format!("skipped tiny candidate at offset {}", hit.offset));
            continue;
        }

        // Replay an already-carved artifact verbatim when its output file
        // still exists; a deleted/never-finished output is re-carved.
        if let Some(artifact) = carved_map.get(&hit.offset)
            && artifact.output_path.is_file()
        {
            first_by_hash
                .entry(artifact.sha256.clone())
                .or_insert_with(|| artifact.id.clone());
            resumed_artifacts += 1;
            artifacts.push(artifact.clone());
            continue;
        }

        let id = format!("carve_{:06}", artifacts.len() + 1);
        let output_path = unique_path(
            &case_dir
                .join("artifacts/carved")
                .join(format!("{}_{:012x}.{}", id, hit.offset, hit.extension)),
        );
        copy_range(&source_path, hit.offset, size_bytes, &output_path)
            .map_err(|err| format!("failed to carve {}: {err}", output_path.display()))?;
        let sha256 = audit::digest_file(&output_path)?;
        let duplicate_of = first_by_hash.get(&sha256).cloned();
        if duplicate_of.is_none() {
            first_by_hash.insert(sha256.clone(), id.clone());
        }
        let validation_status = if duplicate_of.is_some() {
            "duplicate-candidate"
        } else {
            "candidate-unvalidated"
        }
        .to_string();
        let artifact = CarvedArtifact {
            id,
            source_path: source_path.clone(),
            output_path,
            offset: hit.offset,
            size_bytes,
            signature: hit.signature.clone(),
            extension: hit.extension.clone(),
            sha256,
            validation_status,
            validation_note: validation_note_for_signature(&hit.signature).to_string(),
            duplicate_of,
        };
        // Checkpoint the artifact right after it lands on disk: a crash
        // before this line re-carves the range, a crash after it replays.
        checkpoint.append_line(&format!("{{\"carved\":{}}}", artifact.to_json()))?;
        artifacts.push(artifact);
    }

    let result = CarveResult {
        source_path,
        carved_unix: now_unix()?,
        source_size_bytes: source_size,
        artifacts,
        resumed_artifacts,
        resumed_scan_offset: resume_offset,
        warnings,
        options: options.clone(),
    };
    write_carve_outputs(case_dir, &result)?;
    if resumed_artifacts > 0 || resume_offset > 0 {
        let line = format!(
            "{{\"schema_version\":1,\"event\":\"carve-resume\",\"run_id\":\"{}\",\"resumed_artifacts\":{},\"resumed_scan_offset\":{}}}",
            json_escape(checkpoint.run_id()),
            resumed_artifacts,
            resume_offset
        );
        audit::append_chained_jsonl(&case_dir.join("artifacts/carved/carve-log.jsonl"), &line)?;
    }
    checkpoint.finish()?;
    Ok(result)
}

/// Fingerprint of everything that decides carve output: canonical source
/// path, size+mtime, and the option set.
fn carve_fingerprint(
    source_path: &Path,
    metadata: &std::fs::Metadata,
    options: &CarveOptions,
) -> String {
    let modified_unix = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs());
    checkpoint::fingerprint(&[
        "carve-file",
        &source_path.to_string_lossy(),
        &metadata.len().to_string(),
        &modified_unix
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        &options.max_bytes.to_string(),
        &options.max_candidates.to_string(),
    ])
}

/// Decoded prior-run carve progress: the latest signature-scan position
/// plus every completed artifact keyed by hit offset.
#[derive(Debug, Default)]
struct CarveCheckpointState {
    /// Byte offset the signature scan had reached (0 = not started).
    scan_offset: u64,
    /// Cumulative hits at that offset (each `scan` line carries the full
    /// set found so far; the last line wins).
    hits: Vec<CarveHit>,
    /// Artifacts already carved, keyed by their hit's offset.
    carved: HashMap<u64, CarvedArtifact>,
}

/// Decodes a prior run's carve checkpoint lines into `CarveCheckpointState`.
fn load_carve_checkpoint(checkpoint: &RunCheckpoint) -> Result<CarveCheckpointState, String> {
    let corrupt = |detail: String| {
        format!(
            "checkpoint {} is corrupt ({detail}); delete it or rerun with --no-resume",
            checkpoint.path().display()
        )
    };
    let mut state = CarveCheckpointState::default();
    for line in checkpoint.data_lines() {
        let value: serde_json::Value = serde_json::from_str(line)
            .map_err(|err| corrupt(format!("record line does not parse: {err}")))?;
        if let Some(scan) = value.get("scan") {
            state.scan_offset = scan
                .get("offset")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| corrupt("scan record is missing offset".to_string()))?;
            state.hits = serde_json::from_value(
                scan.get("hits").cloned().unwrap_or(serde_json::Value::Null),
            )
            .map_err(|err| corrupt(format!("scan record hits do not parse: {err}")))?;
        } else if let Some(carved_value) = value.get("carved") {
            let artifact: CarvedArtifact = serde_json::from_value(carved_value.clone())
                .map_err(|err| corrupt(format!("carved record does not parse: {err}")))?;
            state.carved.insert(artifact.offset, artifact);
        }
    }
    Ok(state)
}

/// Serializes cumulative signature-scan progress for the checkpoint.
fn scan_progress_line(offset: u64, hits: &[CarveHit]) -> String {
    let hits_json = hits
        .iter()
        .map(|hit| {
            format!(
                "{{\"offset\":{},\"signature\":\"{}\",\"extension\":\"{}\"}}",
                hit.offset,
                json_escape(&hit.signature),
                json_escape(&hit.extension)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"scan\":{{\"offset\":{offset},\"hits\":[{hits_json}]}}}}")
}

/// Signature scan over the source file that can resume mid-file: it
/// re-reads the OVERLAP_SIZE window before `resume_offset` (so signatures
/// straddling the boundary are still found — duplicates dedup at the end)
/// and checkpoints cumulative progress every CHECKPOINT_CHUNK_INTERVAL
/// chunks plus once at the end.
fn scan_signatures(
    source_path: &Path,
    resume_offset: u64,
    prior_hits: Vec<CarveHit>,
    max_candidates: usize,
    checkpoint: &mut RunCheckpoint,
) -> Result<Vec<CarveHit>, String> {
    let mut hits = prior_hits;
    if hits.len() >= max_candidates {
        // The interrupted run had already stopped scanning at the cap.
        hits.truncate(max_candidates);
        return Ok(hits);
    }
    let mut file =
        File::open(source_path).map_err(|err| format!("failed to open carve source: {err}"))?;
    let overlap_start = resume_offset.saturating_sub(OVERLAP_SIZE as u64);
    let mut overlap = vec![0u8; (resume_offset - overlap_start) as usize];
    file.seek(SeekFrom::Start(overlap_start))
        .and_then(|_| file.read_exact(&mut overlap))
        .and_then(|_| file.seek(SeekFrom::Start(resume_offset)))
        .map_err(|err| format!("failed to seek carve source to resume offset: {err}"))?;
    let mut offset = resume_offset;
    let mut since_checkpoint = 0usize;

    loop {
        let mut chunk = vec![0u8; CHUNK_SIZE];
        let read = file
            .read(&mut chunk)
            .map_err(|err| format!("failed to read carve source: {err}"))?;
        if read == 0 {
            break;
        }
        chunk.truncate(read);

        let mut scan = Vec::with_capacity(overlap.len() + chunk.len());
        scan.extend_from_slice(&overlap);
        scan.extend_from_slice(&chunk);
        let scan_start = offset.saturating_sub(overlap.len() as u64);
        scan_buffer(&scan, scan_start, offset, &mut hits);
        if hits.len() >= max_candidates {
            hits.truncate(max_candidates);
            break;
        }

        overlap.clear();
        let keep = OVERLAP_SIZE.min(scan.len());
        overlap.extend_from_slice(&scan[scan.len() - keep..]);
        offset = offset.saturating_add(read as u64);
        since_checkpoint += 1;
        if since_checkpoint >= CHECKPOINT_CHUNK_INTERVAL {
            checkpoint.append_line(&scan_progress_line(offset, &hits))?;
            since_checkpoint = 0;
        }
    }
    checkpoint.append_line(&scan_progress_line(offset, &hits))?;
    hits.sort_by_key(|hit| hit.offset);
    hits.dedup_by_key(|hit| hit.offset);
    Ok(hits)
}

fn validation_note_for_signature(signature: &str) -> &'static str {
    match signature {
        "mp4-ftyp" => {
            "MP4 ftyp signature found and carved contiguously; verify moov/mdat structure and playback before reporting as recovered."
        }
        "riff-avi" => {
            "RIFF AVI signature found and carved contiguously; verify AVI index/playback before reporting as recovered."
        }
        "dahua-dhav" => {
            "Dahua DHAV signature found; treat as proprietary candidate and validate with FFmpeg or vendor player."
        }
        "hikvision-imkh" => {
            "Hikvision IMKH signature found; strip the 40-byte header with export-hik and validate playback before reporting."
        }
        _ => {
            "Signature-based contiguous carve only; verify playback/container integrity before reporting as recovered video."
        }
    }
}

/// Chunked signature scan over any byte source, identical to the on-disk
/// carve walk (1 MiB chunks, 32-byte overlap). `#[doc(hidden)]`: exposed so
/// the fuzz harness can drive the scanner without touching the filesystem;
/// not part of the supported API.
#[doc(hidden)]
pub fn find_video_signatures_in(
    reader: &mut impl Read,
    max_candidates: usize,
) -> Result<Vec<CarveHit>, String> {
    let mut hits = Vec::new();
    let mut offset = 0u64;
    let mut overlap = Vec::<u8>::new();

    loop {
        let mut chunk = vec![0u8; CHUNK_SIZE];
        let read = reader
            .read(&mut chunk)
            .map_err(|err| format!("failed to read carve source: {err}"))?;
        if read == 0 {
            break;
        }
        chunk.truncate(read);

        let mut scan = Vec::with_capacity(overlap.len() + chunk.len());
        scan.extend_from_slice(&overlap);
        scan.extend_from_slice(&chunk);
        let scan_start = offset.saturating_sub(overlap.len() as u64);
        scan_buffer(&scan, scan_start, offset, &mut hits);
        if hits.len() >= max_candidates {
            hits.truncate(max_candidates);
            break;
        }

        overlap.clear();
        let keep = OVERLAP_SIZE.min(scan.len());
        overlap.extend_from_slice(&scan[scan.len() - keep..]);
        offset = offset.saturating_add(read as u64);
    }

    hits.sort_by_key(|hit| hit.offset);
    hits.dedup_by_key(|hit| hit.offset);
    Ok(hits)
}

fn scan_buffer(scan: &[u8], scan_start: u64, current_chunk_start: u64, hits: &mut Vec<CarveHit>) {
    for index in 0..scan.len() {
        let absolute = scan_start + index as u64;
        if absolute < current_chunk_start.saturating_sub(OVERLAP_SIZE as u64) {
            continue;
        }

        if index >= 4 && scan.get(index..index + 4) == Some(b"ftyp") {
            hits.push(CarveHit {
                offset: absolute - 4,
                signature: "mp4-ftyp".to_string(),
                extension: "mp4".to_string(),
            });
        }
        if scan.get(index..index + 4) == Some(b"RIFF")
            && scan.get(index + 8..index + 11) == Some(b"AVI")
        {
            hits.push(CarveHit {
                offset: absolute,
                signature: "riff-avi".to_string(),
                extension: "avi".to_string(),
            });
        }
        if scan.get(index..index + 4) == Some(b"DHAV") {
            hits.push(CarveHit {
                offset: absolute,
                signature: "dahua-dhav".to_string(),
                extension: "dav".to_string(),
            });
        }
        if scan.get(index..index + 4) == Some(b"IMKH") {
            hits.push(CarveHit {
                offset: absolute,
                signature: "hikvision-imkh".to_string(),
                extension: "mpg".to_string(),
            });
        }
    }
}

fn copy_range(source: &Path, offset: u64, size_bytes: u64, output: &Path) -> io::Result<()> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut input = File::open(source)?;
    input.seek(SeekFrom::Start(offset))?;
    let mut reader = input.take(size_bytes);
    let output_file = File::create(output)?;
    let mut writer = BufWriter::new(output_file);
    io::copy(&mut reader, &mut writer)?;
    writer.flush()
}

fn write_carve_outputs(case_dir: &Path, result: &CarveResult) -> Result<(), String> {
    write_text_atomic(&case_dir.join("db/carve_results.json"), &result.to_json())
        .map_err(|err| format!("failed to write carve results: {err}"))?;

    let log_path = case_dir.join("artifacts/carved/carve-log.jsonl");
    for artifact in &result.artifacts {
        audit::append_chained_jsonl(&log_path, &artifact.to_json())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CarveHit, scan_buffer, validation_note_for_signature};

    #[test]
    fn finds_mp4_ftyp_start_offset() {
        let mut hits = Vec::new();
        scan_buffer(b"\0\0\0\x18ftypmp42payload", 100, 100, &mut hits);
        assert_eq!(
            hits[0],
            CarveHit {
                offset: 100,
                signature: "mp4-ftyp".to_string(),
                extension: "mp4".to_string()
            }
        );
    }

    #[test]
    fn finds_avi_and_dhav_signatures() {
        let mut hits = Vec::new();
        scan_buffer(b"RIFFxxxxAVI data DHAVmore", 0, 0, &mut hits);
        assert!(hits.iter().any(|hit| hit.signature == "riff-avi"));
        assert!(hits.iter().any(|hit| hit.signature == "dahua-dhav"));
    }

    #[test]
    fn finds_imkh_signature() {
        let mut hits = Vec::new();
        scan_buffer(b"padIMKHpayloaddatahere", 0, 0, &mut hits);
        assert!(hits.iter().any(|hit| hit.signature == "hikvision-imkh"));
        assert!(validation_note_for_signature("hikvision-imkh").contains("export-hik"));
    }

    #[test]
    fn explains_validation_boundary_by_signature() {
        assert!(validation_note_for_signature("mp4-ftyp").contains("moov/mdat"));
        assert!(validation_note_for_signature("dahua-dhav").contains("proprietary"));
    }

    use super::{ResumeMode, RunCheckpoint, carve_file, carve_fingerprint, scan_progress_line};
    use std::fs;
    use std::path::PathBuf;

    fn resume_fixture(name: &str) -> (PathBuf, PathBuf, Vec<u8>) {
        let root = std::env::temp_dir().join(format!(
            "frametrace-carve-resume-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let case_dir = root.join("case");
        fs::create_dir_all(case_dir.join("artifacts/carved")).unwrap();
        fs::create_dir_all(case_dir.join("db")).unwrap();
        let source = root.join("image.raw");
        // Two MP4 ftyp signatures at offsets 0 and 128 inside 256 bytes.
        let mut content = vec![0u8; 256];
        content[..8].copy_from_slice(b"\0\0\0\x18ftyp");
        content[128..136].copy_from_slice(b"\0\0\0\x18ftyp");
        fs::write(&source, &content).unwrap();
        (case_dir, source, content)
    }

    /// An interrupted carve resumes the signature scan AND replays already
    /// carved artifacts instead of re-copying their ranges.
    #[test]
    fn resume_replays_scan_and_completed_artifacts() {
        let (case_dir, source, content) = resume_fixture("replay");
        let options = super::CarveOptions::default();
        let first = carve_file(&case_dir, &source, &options, ResumeMode::Auto).unwrap();
        assert_eq!(first.artifacts.len(), 2);
        let first_output = first.artifacts[0].output_path.clone();

        // Simulate a crash after artifact 1 landed: drop the results index,
        // the audit log, and the second artifact's output file.
        let _ = fs::remove_file(case_dir.join("db/carve_results.json"));
        let _ = fs::remove_file(case_dir.join("artifacts/carved/carve-log.jsonl"));
        fs::remove_file(&first.artifacts[1].output_path).unwrap();

        // Rebuild the checkpoint the crashed run would have left: full scan
        // progress + the first artifact's carved record.
        let canonical = source.canonicalize().unwrap();
        let metadata = fs::metadata(&source).unwrap();
        let fingerprint = carve_fingerprint(&canonical, &metadata, &options);
        {
            let mut checkpoint = RunCheckpoint::begin(
                &case_dir.join("db/carve-progress.jsonl"),
                "carve-file",
                &fingerprint,
                ResumeMode::Auto,
            )
            .unwrap();
            let hits = vec![
                super::CarveHit {
                    offset: 0,
                    signature: "mp4-ftyp".to_string(),
                    extension: "mp4".to_string(),
                },
                super::CarveHit {
                    offset: 128,
                    signature: "mp4-ftyp".to_string(),
                    extension: "mp4".to_string(),
                },
            ];
            checkpoint
                .append_line(&scan_progress_line(content.len() as u64, &hits))
                .unwrap();
            checkpoint
                .append_line(&format!("{{\"carved\":{}}}", first.artifacts[0].to_json()))
                .unwrap();
        }

        let second = carve_file(&case_dir, &source, &options, ResumeMode::Auto).unwrap();
        assert_eq!(second.resumed_artifacts, 1);
        assert_eq!(second.resumed_scan_offset, content.len() as u64);
        assert_eq!(second.artifacts.len(), 2);
        // The resumed artifact keeps its original output path; the
        // re-carved one lands wherever the run puts it.
        assert_eq!(second.artifacts[0].output_path, first_output);
        assert_eq!(second.artifacts[0].sha256, first.artifacts[0].sha256);
        assert!(
            !case_dir.join("db/carve-progress.jsonl").exists(),
            "checkpoint deleted on full success"
        );
        // The resume is audit-logged into the carve log.
        let log = fs::read_to_string(case_dir.join("artifacts/carved/carve-log.jsonl")).unwrap();
        assert!(log.contains("\"event\":\"carve-resume\""), "{log}");
        let _ = fs::remove_dir_all(case_dir.parent().unwrap());
    }

    /// A checkpoint recorded for a different source/options fingerprint is
    /// discarded — the run re-scans and re-carves rather than trusting it.
    #[test]
    fn resume_rejects_stale_fingerprint() {
        let (case_dir, source, content) = resume_fixture("stale");
        let options = super::CarveOptions::default();
        {
            let mut checkpoint = RunCheckpoint::begin(
                &case_dir.join("db/carve-progress.jsonl"),
                "carve-file",
                "bogus",
                ResumeMode::Auto,
            )
            .unwrap();
            checkpoint
                .append_line(&scan_progress_line(content.len() as u64, &[]))
                .unwrap();
        }
        let result = carve_file(&case_dir, &source, &options, ResumeMode::Auto).unwrap();
        assert_eq!(result.resumed_artifacts, 0);
        assert_eq!(result.resumed_scan_offset, 0);
        assert_eq!(result.artifacts.len(), 2);
        let _ = fs::remove_dir_all(case_dir.parent().unwrap());
    }
}
