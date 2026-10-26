use super::*;
use crate::audit;
use crate::carve::{self, CarveOptions};
use crate::case_db;
use crate::checkpoint::ResumeMode;
use crate::e01::{self, E01Options};
use crate::tsk::{self, TskInspectOptions, TskRecoverOptions};
use crate::util::json_escape;
use std::path::Path;

pub fn inspect_e01(
    case_dir: &Path,
    e01_file: &Path,
    options: E01Options,
    filesystem: bool,
) -> Result<(), String> {
    ensure_case(case_dir)?;
    let is_corrupted = e01::inspect_e01(case_dir, e01_file, &options)?;
    let row = case_db::register_evidence_source(
        case_dir,
        &case_db::EvidenceSourceInput {
            kind: "e01".to_string(),
            path: e01_file.to_path_buf(),
            source_id: None,
            write_protect: None,
            acquisition_tool: Some("libewf ewfinfo".to_string()),
            evidence_hash: None,
            notes: Some("Auto-registered by inspect-e01".to_string()),
            metadata_json: None,
        },
    )?;
    println!("E01 inspected");
    if is_corrupted {
        println!(
            "warning: ewfinfo flags this segment set as corrupted/incomplete; see the info log"
        );
    }
    println!("source registered: {} ({})", row.source_id, row.kind);
    println!("source: {}", e01_file.display());
    println!(
        "audit log: {}",
        case_dir.join("evidence/logs/e01-audit.jsonl").display()
    );
    if filesystem {
        // TSK reads the E01 through libewf, so filesystem triage needs no
        // raw export; decompression makes any probe-class timeout
        // meaningless, so run unbounded.
        println!("filesystem triage: mmls/fls read the E01 via libewf (no export)");
        let tsk_options = crate::tsk::TskInspectOptions {
            timeout_secs: None,
            ..Default::default()
        };
        // C7 / B16: a partial listing (direct triage) exits 5 — completed,
        // but the server must report it as partial, never as a clean pass.
        if let Some(reason) = inspect_image_run(case_dir, e01_file, tsk_options)? {
            exit_partial(&reason);
        }
    }
    Ok(())
}

/// C7 / B16 contract: the step completed and recorded everything, but its
/// result is PARTIAL. The last stderr line starts with
/// [`crate::tsk::PARTIAL_LINE_PREFIX`].
fn exit_partial(reason: &str) -> ! {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    eprintln!("{}{reason}", crate::tsk::PARTIAL_LINE_PREFIX);
    let _ = std::io::stderr().flush();
    std::process::exit(crate::tsk::EXIT_PARTIAL);
}

pub fn import_e01(case_dir: &Path, e01_file: &Path, options: E01Options) -> Result<(), String> {
    ensure_case(case_dir)?;
    let source = case_db::register_evidence_source(
        case_dir,
        &case_db::EvidenceSourceInput {
            kind: "e01".to_string(),
            path: e01_file.to_path_buf(),
            source_id: None,
            write_protect: None,
            acquisition_tool: Some("libewf".to_string()),
            evidence_hash: None,
            notes: Some("Auto-registered by import-e01".to_string()),
            metadata_json: Some(e01_options_json(&options)),
        },
    )?;
    let job = case_db::start_job(
        case_dir,
        "import-e01",
        e01_file,
        None,
        &e01_options_json(&options),
    )?;
    let result = match e01::import_e01(case_dir, e01_file, &options) {
        Ok(result) => result,
        Err(err) => {
            let _ = case_db::fail_job(case_dir, &job.job_id, &err);
            return Err(err);
        }
    };
    // FrameTrace digested the exported raw image itself, so its hash is
    // recorded as computed, not operator-supplied.
    let raw_source = case_db::register_evidence_source_with_origin(
        case_dir,
        &case_db::EvidenceSourceInput {
            kind: "raw-image".to_string(),
            path: result.raw_output_path.clone(),
            source_id: None,
            write_protect: Some("derived read-only image; preserve original E01".to_string()),
            acquisition_tool: Some("libewf ewfexport".to_string()),
            evidence_hash: Some(result.raw_sha256.clone()),
            notes: Some(format!("Exported from source {}", source.source_id)),
            metadata_json: None,
        },
        case_db::HashOrigin::Computed,
    )
    .inspect_err(|err| {
        let _ = case_db::fail_job(case_dir, &job.job_id, err);
    })?;
    super::case::print_source_changes(case_dir, &raw_source);
    case_db::complete_job(case_dir, &job.job_id, 1, "import-e01 completed")?;
    println!("E01 imported");
    println!("source registered: {} ({})", source.source_id, source.kind);
    println!("job: {} ({})", job.job_id, job.job_type);
    println!("source: {}", result.e01_path.display());
    println!("raw output: {}", result.raw_output_path.display());
    println!("raw sha256: {}", result.raw_sha256);
    if let Some(e01_sha256) = result.e01_sha256 {
        println!("E01 sha256: {e01_sha256}");
    }
    println!("info log: {}", result.ewfinfo_log_path.display());
    if let Some(path) = result.ewfverify_log_path {
        println!("verify log: {}", path.display());
    }
    println!("export log: {}", result.ewfexport_log_path.display());
    // R19-S2 (P29/P32): the paths are quoted, or named by a placeholder.
    println!(
        "{}",
        next_carve_file_hint(case_dir, &result.raw_output_path)
    );
    Ok(())
}

pub fn carve_file(
    case_dir: &Path,
    source_file: &Path,
    options: CarveOptions,
    resume: ResumeMode,
) -> Result<(), String> {
    ensure_case(case_dir)?;
    // C1: an imported raw image is carved only when its completion marker
    // proves the export finished (a killed import's torn image is refused).
    e01::require_complete_case_raw(case_dir, source_file)?;
    let source = case_db::register_evidence_source(
        case_dir,
        &case_db::EvidenceSourceInput {
            kind: "raw-image".to_string(),
            path: source_file.to_path_buf(),
            source_id: None,
            write_protect: None,
            acquisition_tool: None,
            evidence_hash: None,
            notes: Some("Auto-registered by carve-file".to_string()),
            metadata_json: Some(carve_options_json(&options)),
        },
    )?;
    let job = case_db::start_job(
        case_dir,
        "carve-file",
        source_file,
        Some(options.max_candidates as u64),
        &carve_options_json(&options),
    )?;
    let progress_job_id = job.job_id.clone();
    let progress = move |done: u64, total: u64| {
        let _ = case_db::report_job_progress(case_dir, &progress_job_id, Some(total), done);
    };
    let result = match carve::carve_file(case_dir, source_file, &options, resume, Some(&progress)) {
        Ok(result) => result,
        Err(err) => {
            let _ = case_db::fail_job(case_dir, &job.job_id, &err);
            return Err(err);
        }
    };
    case_db::complete_job(
        case_dir,
        &job.job_id,
        result.artifacts.len() as u64,
        "carve-file completed",
    )?;
    println!("carve complete");
    println!("source registered: {} ({})", source.source_id, source.kind);
    println!("job: {} ({})", job.job_id, job.job_type);
    println!("source: {}", result.source_path.display());
    println!("artifacts carved: {}", result.artifacts.len());
    if result.resumed_artifacts > 0 || result.resumed_scan_offset > 0 {
        println!(
            "resumed from checkpoint: {} artifact(s) reused, scan offset {}",
            result.resumed_artifacts, result.resumed_scan_offset
        );
    }
    println!(
        "results: {}",
        case_dir.join("db/carve_results.json").display()
    );
    Ok(())
}

pub fn inspect_image(
    case_dir: &Path,
    image_file: &Path,
    options: TskInspectOptions,
) -> Result<(), String> {
    // C7 / B16: exit code 5 when the listing is partial.
    if let Some(reason) = inspect_image_run(case_dir, image_file, options)? {
        exit_partial(&reason);
    }
    Ok(())
}

/// Runs the inspection; `Ok(Some(reason))` when the result is partial
/// (fls timed out, or the listing was truncated to `--max-entries`).
fn inspect_image_run(
    case_dir: &Path,
    image_file: &Path,
    options: TskInspectOptions,
) -> Result<Option<String>, String> {
    ensure_case(case_dir)?;
    let source = case_db::register_evidence_source(
        case_dir,
        &case_db::EvidenceSourceInput {
            kind: "forensic-image".to_string(),
            path: image_file.to_path_buf(),
            source_id: None,
            write_protect: Some(
                "treat image as read-only; recover into derived artifacts".to_string(),
            ),
            acquisition_tool: Some("Sleuth Kit mmls/fls".to_string()),
            evidence_hash: None,
            notes: Some("Auto-registered by inspect-image".to_string()),
            metadata_json: Some(tsk_inspect_options_json(&options)),
        },
    )?;
    let job = case_db::start_job(
        case_dir,
        "inspect-image",
        image_file,
        Some(options.max_entries as u64),
        &tsk_inspect_options_json(&options),
    )?;
    let progress_job_id = job.job_id.clone();
    let fls_progress = move |entries_seen: u64| {
        // fls reports no total up front — only the running entry count is
        // honest progress.
        let _ = case_db::report_job_progress(case_dir, &progress_job_id, None, entries_seen);
    };
    let result = match tsk::inspect_image(case_dir, image_file, &options, Some(&fls_progress)) {
        Ok(result) => result,
        Err(err) => {
            let _ = case_db::fail_job(case_dir, &job.job_id, &err);
            return Err(err);
        }
    };
    case_db::complete_job(
        case_dir,
        &job.job_id,
        result.entries.len() as u64,
        if result.fls_completed {
            "inspect-image completed"
        } else {
            "inspect-image completed with a PARTIAL fls listing (timeout)"
        },
    )?;
    println!("filesystem image inspected");
    println!("source registered: {} ({})", source.source_id, source.kind);
    println!("job: {} ({})", job.job_id, job.job_type);
    println!("image: {}", result.image_path.display());
    println!("partition offset: {}", result.partition_offset);
    println!("partitions: {}", result.partitions.len());
    println!("entries: {}", result.entries.len());
    println!(
        "deleted entries: {}",
        result.entries.iter().filter(|entry| entry.deleted).count()
    );
    println!(
        "video candidates: {}",
        result
            .entries
            .iter()
            .filter(|entry| entry.video_candidate)
            .count()
    );
    println!("summary: {}", result.summary_path.display());
    println!("entries jsonl: {}", result.entries_jsonl_path.display());
    println!("mmls log: {}", result.mmls_log_path.display());
    println!("fls log: {}", result.fls_log_path.display());
    if !result.warnings.is_empty() {
        println!("warnings: {}", result.warnings.len());
        for warning in &result.warnings {
            println!("warning: {warning}");
        }
    }
    Ok(crate::tsk::partial_reason(&result))
}

pub fn recover_inode(
    case_dir: &Path,
    image_file: &Path,
    options: TskRecoverOptions,
) -> Result<(), String> {
    run_recover_inode(case_dir, image_file, options).map(|_| ())
}

/// [`recover_inode`] returning the recovery result (recover-batch keeps
/// the new output's audit identity for the rest of the batch, M1).
fn run_recover_inode(
    case_dir: &Path,
    image_file: &Path,
    options: TskRecoverOptions,
) -> Result<tsk::TskRecoverResult, String> {
    ensure_case(case_dir)?;
    // Merge review M2: a live-file copy is an extraction, not a recovery.
    let event = options.audit_event();
    let source = case_db::register_evidence_source(
        case_dir,
        &case_db::EvidenceSourceInput {
            kind: "forensic-image".to_string(),
            path: image_file.to_path_buf(),
            source_id: None,
            write_protect: Some("source image read-only; inode output is derived".to_string()),
            acquisition_tool: Some("Sleuth Kit icat".to_string()),
            evidence_hash: None,
            notes: Some(format!("Auto-registered by {event}")),
            metadata_json: Some(tsk_recover_options_json(&options)),
        },
    )?;
    let job = case_db::start_job(
        case_dir,
        event,
        image_file,
        Some(1),
        &tsk_recover_options_json(&options),
    )?;
    let result = match tsk::recover_inode(case_dir, image_file, &options) {
        Ok(result) => result,
        Err(err) => {
            let _ = case_db::fail_job(case_dir, &job.job_id, &err);
            return Err(err);
        }
    };
    case_db::complete_job(case_dir, &job.job_id, 1, &format!("{event} completed"))?;
    println!(
        "{}",
        if options.extract {
            "inode extracted"
        } else {
            "inode recovered"
        }
    );
    println!("source registered: {} ({})", source.source_id, source.kind);
    println!("job: {} ({})", job.job_id, job.job_type);
    println!("image: {}", result.image_path.display());
    println!("partition offset: {}", result.partition_offset);
    println!("inode: {}", result.inode);
    println!("output: {}", result.output_path.display());
    println!("size bytes: {}", result.size_bytes);
    println!("sha256: {}", result.sha256);
    println!("validation: {}", result.validation_status);
    for warning in &result.warnings {
        println!("warning: {warning}");
    }
    println!(
        "{}",
        next_scan_folder_hint(
            case_dir,
            result
                .output_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
        )
    );
    Ok(result)
}

/// R19-S2: `next: carve-file "<case>" "<raw>"` — each path quoted when
/// paste-safe, else `<케이스 폴더>` / `<파일>` and the reason
/// ([`crate::paste_path::hint_command`]).
pub(crate) fn next_carve_file_hint(case_dir: &Path, raw: &Path) -> String {
    use crate::paste_path::{Arg, hint_command};
    hint_command(&[
        Arg::Word("next: carve-file"),
        Arg::Path(case_dir, "<케이스 폴더>"),
        Arg::Path(raw, "<파일>"),
    ])
}

/// R19-S2: `next: scan-folder "<case>" "<folder>" --no-ffprobe`.
pub(crate) fn next_scan_folder_hint(case_dir: &Path, folder: &Path) -> String {
    use crate::paste_path::{Arg, hint_command};
    hint_command(&[
        Arg::Word("next: scan-folder"),
        Arg::Path(case_dir, "<케이스 폴더>"),
        Arg::Path(folder, "<폴더>"),
        Arg::Word("--no-ffprobe"),
    ])
}

/// The image-internal path `raw` (an `fls` path, `/` or `\` separated) as
/// a relative path under `artifacts/extracted` that keeps the folder tree
/// and the names (Korean folder names, extensions) but can never leave it:
/// empty, `.` and `..` components are dropped, roots / drive letters /
/// ADS colons cannot survive ([`clean_path_component`]).
pub(crate) fn clean_relative_path(raw: &str) -> std::path::PathBuf {
    let raw = raw.trim().trim_end_matches("(deleted)").trim();
    let mut path = std::path::PathBuf::new();
    for part in raw.split(['/', '\\']) {
        if let Some(clean) = clean_path_component(part) {
            path.push(clean);
        }
    }
    if path.as_os_str().is_empty() {
        std::path::PathBuf::from("unnamed.bin")
    } else {
        path
    }
}

/// One path component safe on Windows and Unix, or `None` for one that
/// names no folder (`""`, `.`, `..`). Characters Windows forbids in names
/// (`<>:"/\|?*`, control characters) become `_`, trailing dots / spaces
/// are trimmed (Windows drops them silently), and a reserved device name
/// (`CON`, `nul.txt`, `COM1`, …) gets a `_` prefix. Everything else —
/// Unicode letters, spaces, the extension — is kept.
pub(crate) fn clean_path_component(part: &str) -> Option<String> {
    let mapped: String = part
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
            {
                '_'
            } else {
                ch
            }
        })
        .collect();
    let trimmed = mapped.trim().trim_end_matches(['.', ' ']);
    if trimmed.is_empty() {
        return None;
    }
    let stem = trimmed.split('.').next().unwrap_or_default().trim_end();
    Some(if is_reserved_device_name(stem) {
        format!("_{trimmed}")
    } else {
        trimmed.to_string()
    })
}

/// A Windows reserved device name (the part before the first `.`):
/// `CON`, `PRN`, `AUX`, `NUL`, `CONIN$`, `CONOUT$`, `COM1`-`COM9`,
/// `LPT1`-`LPT9` and the superscript forms `COM¹`-`COM³` / `LPT¹`-`LPT³`
/// that Windows also maps to devices — ASCII case-insensitive.
pub(crate) fn is_reserved_device_name(stem: &str) -> bool {
    let upper = stem.to_ascii_uppercase();
    if matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) {
        return true;
    }
    let chars: Vec<char> = upper.chars().collect();
    chars.len() == 4
        && matches!(chars[..3], ['C', 'O', 'M'] | ['L', 'P', 'T'])
        && matches!(chars[3], '1'..='9' | '\u{b9}' | '\u{b2}' | '\u{b3}')
}

/// Merge review M1: who wrote an existing icat output — the image, partition
/// offset and inode of a `recover-inode` / `extract-inode` record in the
/// case's tsk audit lane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InodeOutputRecord {
    pub(crate) output_path: std::path::PathBuf,
    pub(crate) image_path: String,
    pub(crate) partition_offset: u64,
    pub(crate) inode: String,
    pub(crate) size_bytes: u64,
}

/// Every `recover-inode` / `extract-inode` output the case's
/// `evidence/logs/tsk-audit.jsonl` records (usable entries only — an
/// unreadable or forged lane proves nothing, so the batch then writes a
/// new name instead of skipping).
pub(crate) fn inode_output_records(case_dir: &Path) -> Vec<InodeOutputRecord> {
    let rows = audit::read_lane_rows(&case_dir.join("evidence/logs/tsk-audit.jsonl"))
        .map(|lane| lane.usable_values())
        .unwrap_or_default();
    rows.iter()
        .filter(|row| {
            matches!(
                row.get("event").and_then(|v| v.as_str()),
                Some(tsk::RECOVER_EVENT | tsk::EXTRACT_EVENT)
            )
        })
        .filter_map(|row| {
            Some(InodeOutputRecord {
                output_path: std::path::PathBuf::from(row.get("output_path")?.as_str()?),
                image_path: row.get("image_path")?.as_str()?.to_string(),
                partition_offset: row.get("partition_offset")?.as_u64()?,
                inode: row.get("inode")?.as_str()?.to_string(),
                size_bytes: row.get("size_bytes")?.as_u64()?,
            })
        })
        .collect()
}

/// What recover-batch does with one item's output name (M1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OutputChoice {
    /// This exact source (image, partition, inode) already wrote this file
    /// (the audit record names it, same size): kept, reported `skipped`.
    Same(std::path::PathBuf),
    /// A free name to write to.
    Fresh(std::path::PathBuf),
}

/// `base` with `suffix` before its extension (`clip.mp4` + `__ino7` →
/// `clip__ino7.mp4`).
fn with_name_suffix(base: &Path, suffix: &str) -> std::path::PathBuf {
    let stem = base
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let name = match base.extension() {
        Some(ext) => format!("{stem}{suffix}.{}", ext.to_string_lossy()),
        None => format!("{stem}{suffix}"),
    };
    base.with_file_name(name)
}

/// Merge review M1: picks the output of one recover-batch item. An existing
/// file is skipped ONLY when the tsk audit proves it is this item's own
/// earlier output (same canonical output path, image, partition offset,
/// inode and size). Otherwise it belongs to a DIFFERENT source that maps
/// to the same name (case-insensitive host: `Clip.MP4` vs `clip.mp4`;
/// names equal after cleaning: `a:b.mp4` vs `a_b.mp4`, `clip.mp4.` vs
/// `clip.mp4`; another image / partition in a reused case), and this item
/// is written next to it as `<stem><alt_suffix>.<ext>` (then
/// `<stem><alt_suffix>_001.<ext>` …). Probing the same sequence on every
/// run keeps reruns idempotent — never a new copy per run, a 0-byte
/// source included (its record matches like any other).
pub(crate) fn choose_output_path(
    base: &Path,
    alt_suffix: &str,
    image_path: &str,
    partition_offset: u64,
    inode: &str,
    records: &[InodeOutputRecord],
) -> OutputChoice {
    let alt = with_name_suffix(base, alt_suffix);
    let candidates = [base.to_path_buf(), alt.clone()]
        .into_iter()
        .chain((1..1000).map(|n| with_name_suffix(&alt, &format!("_{n:03}"))));
    for candidate in candidates {
        let meta = match std::fs::symlink_metadata(&candidate) {
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return OutputChoice::Fresh(candidate);
            }
            Err(_) => continue,
            Ok(meta) => meta,
        };
        if !meta.is_file() {
            continue;
        }
        let Ok(canonical) = crate::util::canonicalize_display(&candidate) else {
            continue;
        };
        let same = records.iter().any(|record| {
            record.output_path == canonical
                && record.image_path == image_path
                && record.partition_offset == partition_offset
                && record.inode == inode
                && record.size_bytes == meta.len()
        });
        if same {
            return OutputChoice::Same(candidate);
        }
    }
    OutputChoice::Fresh(crate::util::unique_available_path(&alt))
}

/// Free space kept on the case volume while extracting (hunch: refuse
/// before filling the disk). The inspection's `fls -r -p` listing carries
/// no file sizes, so the batch cannot total them up front; before each
/// item it needs this floor plus the largest file written so far.
pub(crate) const EXTRACT_FREE_FLOOR_BYTES: u64 = 1024 * 1024 * 1024;

/// `Err` (a clear message) when `free` bytes cannot take the next item:
/// below [`EXTRACT_FREE_FLOOR_BYTES`] plus the largest output so far.
pub(crate) fn extraction_space_check(free: u64, largest_so_far: u64) -> Result<(), String> {
    let needed = EXTRACT_FREE_FLOOR_BYTES.saturating_add(largest_so_far);
    if free < needed {
        return Err(format!(
            "insufficient disk space for further extraction: {} free on the case volume, need at least {} (1 GB reserve + the largest file so far) — free space and rerun; finished files are kept and skipped",
            crate::diskspace::human_bytes(free),
            crate::diskspace::human_bytes(needed)
        ));
    }
    Ok(())
}

/// The last `recover batch result:` line of a recover-batch run's output
/// (merge review L2): `(ok, skipped)`, or `None` when the run printed none
/// (nothing selected, an error before the summary).
pub(crate) fn parse_batch_result(output: &str) -> Option<(u64, u64)> {
    let line = output
        .lines()
        .rev()
        .find_map(|line| line.trim().strip_prefix("recover batch result:"))?;
    let field = |key: &str| {
        line.split_whitespace()
            .find_map(|part| part.strip_prefix(key))
            .and_then(|value| value.parse::<u64>().ok())
    };
    Some((field("ok=")?, field("skipped=")?))
}

pub fn recover_batch(
    case_dir: &Path,
    image_file: &Path,
    selection_path: Option<&Path>,
    partition_offset: u64,
    deleted_videos: bool,
    allocated_videos: bool,
    timeout_secs: Option<u64>,
) -> Result<(), String> {
    ensure_case(case_dir)?;
    let (selection, selection_label) = if allocated_videos {
        let entries_jsonl = latest_entries_jsonl(case_dir).ok_or_else(|| {
            "no filesystem inspection entries found; run inspect-image or inspect-e01 --filesystem first"
                .to_string()
        })?;
        let items = allocated_video_selectors(&entries_jsonl)?;
        if items.is_empty() {
            println!(
                "latest inspection {} lists no allocated video candidates",
                entries_jsonl.display()
            );
            return Ok(());
        }
        println!("auto-selection (allocated): {}", entries_jsonl.display());
        (
            crate::selection::SelectionFile {
                case_id: None,
                items,
            },
            entries_jsonl.display().to_string(),
        )
    } else if deleted_videos {
        let entries_jsonl = latest_entries_jsonl(case_dir).ok_or_else(|| {
            "no filesystem inspection entries found; run inspect-image or inspect-e01 --filesystem first"
                .to_string()
        })?;
        let items = deleted_video_selectors(&entries_jsonl)?;
        if items.is_empty() {
            return Err(format!(
                "latest inspection {} lists no deleted video candidates",
                entries_jsonl.display()
            ));
        }
        println!("auto-selection: {}", entries_jsonl.display());
        (
            crate::selection::SelectionFile {
                case_id: None,
                items,
            },
            entries_jsonl.display().to_string(),
        )
    } else {
        let selection_path = selection_path.ok_or_else(|| {
            "recover-batch needs a selection file or --deleted-videos / --allocated-videos"
                .to_string()
        })?;
        let parsed = crate::selection::parse_selection_file(selection_path)?;
        crate::selection::enforce_case_binding(
            case_dir,
            parsed.case_id.as_deref(),
            selection_path,
            crate::selection::ImportKind::Selection,
        )?;
        (parsed, selection_path.display().to_string())
    };
    let job = case_db::start_job(
        case_dir,
        "recover-batch",
        image_file,
        Some(selection.items.len() as u64),
        &format!(
            "{{\"items\":{},\"deleted_videos\":{deleted_videos},\"allocated_videos\":{allocated_videos}}}",
            selection.items.len()
        ),
    )?;

    let is_extract = allocated_videos;
    // M1: the identity of every earlier output (and, below, of this
    // batch's own outputs) decides whether an existing file is this item's.
    let mut records = inode_output_records(case_dir);
    let image_identity = crate::util::canonicalize_display(image_file)
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default();
    // L1: `.part` files a killed extraction left anywhere in the tree
    // (same pid / age rules as every other stale-part sweep).
    if selection
        .items
        .iter()
        .any(|item| is_extract || item.action.as_deref() == Some("extract"))
    {
        for (path, result) in
            crate::join::sweep_stale_parts_recursive(&case_dir.join("artifacts/extracted"))
        {
            if let Err(err) = result {
                eprintln!("warning: {err} ({})", path.display());
            }
        }
    }
    let mut largest_so_far = 0u64;
    let mut out_of_space: Option<String> = None;
    let mut outcomes = Vec::new();
    for item in &selection.items {
        let inode = item
            .selector
            .trim()
            .strip_prefix("fls:")
            .unwrap_or(item.selector.trim())
            .to_string();

        let item_is_extract = is_extract || item.action.as_deref() == Some("extract");
        let action = if item_is_extract {
            "extract"
        } else {
            "recover"
        };
        let (base_path, recover_deleted, detail_folder, alt_suffix) = if item_is_extract {
            let note = item.notes.as_deref().unwrap_or(&inode);
            let clean_rel = clean_relative_path(note);
            let target = case_dir.join("artifacts/extracted").join(clean_rel);
            (
                target,
                false,
                "artifacts/extracted",
                format!("__ino{}", crate::util::sanitize_filename(&inode)),
            )
        } else {
            let filename = item
                .notes
                .as_deref()
                .and_then(|p| Path::new(p).file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("");
            let clean_name = filename.trim_end_matches("(deleted)").trim();
            // The original name (extension kept) after the inode, so two
            // deleted files with one name never collide.
            let leaf = match clean_path_component(clean_name) {
                None => format!("inode_{}.bin", crate::util::sanitize_filename(&inode)),
                Some(name) => format!("{}_{name}", crate::util::sanitize_filename(&inode)),
            };
            // PR #6 merge: the documented recovery folder
            // (docs/FILESYSTEM_RECOVERY.md) — packaged with the case
            // (package.rs) and swept for stale `.part` files (media.rs).
            let target = case_dir.join("artifacts/recovered/filesystem").join(leaf);
            (
                target,
                true,
                "artifacts/recovered/filesystem",
                // M1: the same inode number on another partition.
                format!("__off{partition_offset}"),
            )
        };

        if let Some(reason) = &out_of_space {
            outcomes.push(BatchOutcome {
                selector: item.selector.clone(),
                action,
                status: "failed",
                detail: reason.clone(),
            });
            continue;
        }

        // Never overwrite (recover_inode refuses an existing output): an
        // existing file is kept and this item reported `skipped` only when
        // the audit proves it is this item's own earlier output; a file of
        // another source under the same (cleaned / case-folded) name makes
        // this item take a distinct name (M1).
        let output_path = match choose_output_path(
            &base_path,
            &alt_suffix,
            &image_identity,
            partition_offset,
            &inode,
            &records,
        ) {
            OutputChoice::Same(existing) => {
                outcomes.push(BatchOutcome {
                    selector: item.selector.clone(),
                    action,
                    status: "skipped",
                    detail: format!(
                        "already {} from this image, partition and inode at {} (kept, not overwritten)",
                        if item_is_extract {
                            "extracted"
                        } else {
                            "recovered"
                        },
                        existing.display()
                    ),
                });
                continue;
            }
            OutputChoice::Fresh(path) => path,
        };

        // Hunch: never fill the case volume (the listing has no sizes).
        if item_is_extract {
            let free = crate::diskspace::available_bytes(&output_path).unwrap_or(u64::MAX);
            if let Err(reason) = extraction_space_check(free, largest_so_far) {
                eprintln!("{reason}");
                outcomes.push(BatchOutcome {
                    selector: item.selector.clone(),
                    action,
                    status: "failed",
                    detail: reason.clone(),
                });
                out_of_space = Some(reason);
                continue;
            }
        }

        let options = TskRecoverOptions {
            partition_offset,
            inode,
            output_path: Some(output_path),
            recover_deleted,
            include_slack: false,
            skip_sparse_holes: true,
            icat_bin: "icat".to_string(),
            timeout_secs,
            extract: item_is_extract,
        };
        let outcome = run_recover_inode(case_dir, image_file, options);
        outcomes.push(match outcome {
            Ok(result) => {
                largest_so_far = largest_so_far.max(result.size_bytes);
                records.push(InodeOutputRecord {
                    output_path: result.output_path.clone(),
                    image_path: result.image_path.to_string_lossy().to_string(),
                    partition_offset: result.partition_offset,
                    inode: result.inode.clone(),
                    size_bytes: result.size_bytes,
                });
                BatchOutcome {
                    selector: item.selector.clone(),
                    action,
                    status: "ok",
                    detail: format!(
                        "saved into {detail_folder} as {}",
                        result.output_path.display()
                    ),
                }
            }
            Err(error) => BatchOutcome {
                selector: item.selector.clone(),
                action,
                status: "failed",
                detail: error,
            },
        });
    }

    let ok = outcomes.iter().filter(|o| o.status == "ok").count();
    let failed = outcomes.iter().filter(|o| o.status == "failed").count();
    let skipped = outcomes.iter().filter(|o| o.status == "skipped").count();
    let line = format!(
        "{{\"schema_version\":1,\"event\":\"recover-batch\",\"selection_path\":\"{}\",\"requested\":{},\"ok\":{},\"failed\":{},\"skipped\":{},\"results\":{}}}",
        json_escape(&selection_label),
        outcomes.len(),
        ok,
        failed,
        skipped,
        outcomes_json(&outcomes),
    );
    audit::append_chained_jsonl(&case_dir.join("artifacts/logs/batch-log.jsonl"), &line)?;
    // All-items-failed batches must fail the job row, not record success.
    if ok == 0 && failed > 0 && failed == outcomes.len() {
        let _ = case_db::fail_job(
            case_dir,
            &job.job_id,
            &format!("all {failed} item(s) failed"),
        );
        return Err(format!(
            "recover batch failed: {failed} of {} item(s) failed",
            outcomes.len()
        ));
    }
    case_db::complete_job(
        case_dir,
        &job.job_id,
        outcomes.len() as u64,
        "recover-batch completed",
    )?;

    println!("recover batch complete");
    println!("requested: {}", outcomes.len());
    println!("ok: {ok}");
    println!("failed: {failed}");
    println!("skipped: {skipped}");
    for outcome in &outcomes {
        println!(
            "  [{}] {}: {}",
            outcome.status, outcome.selector, outcome.detail
        );
    }
    // L2: the last line, read by the workstation's E01 pipeline
    // ([`parse_batch_result`]) — a long run's head may be cut from the
    // output the server retains.
    println!(
        "recover batch result: requested={} ok={ok} failed={failed} skipped={skipped}",
        outcomes.len()
    );
    Ok(())
}
