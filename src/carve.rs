use crate::audit;
use crate::util::{json_escape, now_unix, unique_path, write_text};
use std::fs::File;
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const CHUNK_SIZE: usize = 1024 * 1024;
const OVERLAP_SIZE: usize = 32;
const DEFAULT_MAX_BYTES: u64 = 512 * 1024 * 1024;
const DEFAULT_MAX_CANDIDATES: usize = 64;
const MIN_CARVE_BYTES: u64 = 16;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarveHit {
    pub offset: u64,
    pub signature: String,
    pub extension: String,
}

#[derive(Debug, Clone)]
pub struct CarvedArtifact {
    pub id: String,
    pub source_path: PathBuf,
    pub output_path: PathBuf,
    pub offset: u64,
    pub size_bytes: u64,
    pub signature: String,
    pub extension: String,
    pub sha256: String,
}

impl CarvedArtifact {
    fn to_json(&self) -> String {
        format!(
            "{{\"schema_version\":2,\"event\":\"carve-file\",\"id\":\"{}\",\"artifact_type\":\"carved-candidate\",\"validation_status\":\"candidate-unvalidated\",\"validation_note\":\"Signature-based contiguous carve only; verify playback/container integrity before reporting as recovered video.\",\"source_path\":\"{}\",\"output_path\":\"{}\",\"offset\":{},\"size_bytes\":{},\"signature\":\"{}\",\"extension\":\"{}\",\"sha256\":\"{}\"}}",
            json_escape(&self.id),
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

    let source_size = std::fs::metadata(&source_path)
        .map_err(|err| format!("failed to read source metadata: {err}"))?
        .len();
    let hits = find_video_signatures(&source_path, options.max_candidates)?;
    let mut warnings = Vec::new();
    if hits.len() >= options.max_candidates {
        warnings.push(format!(
            "candidate limit reached at {}; rerun with --max-candidates if needed",
            options.max_candidates
        ));
    }

    let mut artifacts = Vec::new();
    for (index, hit) in hits.iter().enumerate() {
        let next_offset = hits
            .get(index + 1)
            .map(|next| next.offset)
            .unwrap_or(source_size);
        if next_offset <= hit.offset {
            warnings.push(format!(
                "skipped overlapping candidate at offset {}",
                hit.offset
            ));
            continue;
        }
        let available = next_offset.saturating_sub(hit.offset);
        let size_bytes = available.min(options.max_bytes);
        if size_bytes < MIN_CARVE_BYTES {
            warnings.push(format!("skipped tiny candidate at offset {}", hit.offset));
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
        artifacts.push(CarvedArtifact {
            id,
            source_path: source_path.clone(),
            output_path,
            offset: hit.offset,
            size_bytes,
            signature: hit.signature.clone(),
            extension: hit.extension.clone(),
            sha256,
        });
    }

    let result = CarveResult {
        source_path,
        carved_unix: now_unix()?,
        source_size_bytes: source_size,
        artifacts,
        warnings,
        options: options.clone(),
    };
    write_carve_outputs(case_dir, &result)?;
    Ok(result)
}

fn find_video_signatures(path: &Path, max_candidates: usize) -> Result<Vec<CarveHit>, String> {
    let mut file = File::open(path).map_err(|err| format!("failed to open carve source: {err}"))?;
    let mut hits = Vec::new();
    let mut offset = 0u64;
    let mut overlap = Vec::<u8>::new();

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
    write_text(&case_dir.join("db/carve_results.json"), &result.to_json())
        .map_err(|err| format!("failed to write carve results: {err}"))?;

    let log_path = case_dir.join("artifacts/carved/carve-log.jsonl");
    for artifact in &result.artifacts {
        audit::append_chained_jsonl(&log_path, &artifact.to_json())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CarveHit, scan_buffer};

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
}
