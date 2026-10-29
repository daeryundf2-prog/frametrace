use super::*;
use crate::audit;
use crate::carve::{self, CarveOptions};
use crate::case_db;
use crate::checkpoint::ResumeMode;
use crate::e01::{self, E01Options};
use crate::lazy_extract::{
    allocated_video_selectors, legacy_extracted_file, legacy_extracted_relative_path, path_key,
    plan_extract_targets,
};
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

pub fn verify_e01(case_dir: &Path, e01_file: &Path, options: E01Options) -> Result<(), String> {
    ensure_case(case_dir)?;
    let log = e01::verify_e01(case_dir, e01_file, &options)
        .map_err(|err| format!("E01 hash verification failed: {err}"))?;
    println!("E01 hash verification: verified (stored hash matches the recomputed media)");
    println!("verify log: {}", log.display());
    Ok(())
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
    // fls has no total up front: the cap (`max_entries`) is not one — the
    // workstation showed it as "0/5000000" progress.
    let job = case_db::start_job(
        case_dir,
        "inspect-image",
        image_file,
        None,
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
    ensure_case(case_dir)?;
    let source = case_db::register_evidence_source(
        case_dir,
        &case_db::EvidenceSourceInput {
            kind: "forensic-image".to_string(),
            path: image_file.to_path_buf(),
            source_id: None,
            write_protect: Some("source image read-only; inode output is derived".to_string()),
            acquisition_tool: Some("Sleuth Kit icat".to_string()),
            evidence_hash: None,
            notes: Some("Auto-registered by recover-inode".to_string()),
            metadata_json: Some(tsk_recover_options_json(&options)),
        },
    )?;
    let job = case_db::start_job(
        case_dir,
        "recover-inode",
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
    case_db::complete_job(case_dir, &job.job_id, 1, "recover-inode completed")?;
    println!("inode recovered");
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
    Ok(())
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

/// Indexes the live videos of the latest filesystem inspection where their
/// extraction would put them, without reading any of them: the review opens
/// as soon as the card is listed, and each clip is read out of the image
/// when it is first played or exported ([`crate::lazy_extract`]).
pub fn index_image_files(case_dir: &Path) -> Result<(), String> {
    ensure_case(case_dir)?;
    let listing = crate::lazy_extract::allocated_listing(case_dir)?.ok_or_else(|| {
        "no filesystem inspection found; run inspect-image or inspect-e01 --filesystem first"
            .to_string()
    })?;
    let case_root = crate::util::canonicalize_display(case_dir)
        .map_err(|err| format!("failed to canonicalize case directory: {err}"))?;
    let root = crate::lazy_extract::extracted_root(&case_root);
    let without_size = listing
        .files
        .iter()
        .filter(|file| file.size_bytes.is_none())
        .count();
    let listed: Vec<crate::scan::ListedVideo> = listing
        .files
        .iter()
        .map(|file| crate::scan::ListedVideo {
            path: root.join(&file.relative),
            relative_path: file.relative.to_string_lossy().to_string(),
            size_bytes: file.size_bytes.unwrap_or(0),
        })
        .collect();
    let result = crate::scan::index_listed_videos(case_dir, &root, &listed)?;
    println!(
        "indexed {} live video(s) from the image listing; each is read out of the image when first played or exported",
        result.video_count
    );
    println!("image: {}", listing.image_path.display());
    if without_size > 0 {
        println!(
            "warning: {without_size} listed file(s) have no size (inspection made without fls -l); re-run the analysis to list sizes"
        );
    }
    Ok(())
}

/// Finds the deleted videos of the latest inspected image without reading
/// them out ([`crate::recovery_scan`]); the review lists them graded, and
/// each is read when first played or exported.
pub fn scan_deleted(case_dir: &Path) -> Result<(), String> {
    ensure_case(case_dir)?;
    let job = case_db::start_job(case_dir, "scan-deleted", case_dir, None, "{}")?;
    let job_id = job.job_id.clone();
    let last = std::sync::Mutex::new(std::time::Instant::now());
    let progress = move |done: u64, total: u64| {
        let mut last = last.lock().unwrap_or_else(|e| e.into_inner());
        if done >= total || last.elapsed() >= std::time::Duration::from_secs(1) {
            *last = std::time::Instant::now();
            let _ = case_db::report_job_progress(case_dir, &job_id, Some(total), done);
        }
    };
    let summary = match crate::recovery_scan::scan_deleted(case_dir, Some(&progress)) {
        Ok(summary) => summary,
        Err(err) => {
            let _ = case_db::fail_job(case_dir, &job.job_id, &err);
            return Err(err);
        }
    };
    let found = summary.deleted_entries + summary.carved + summary.frame_runs;
    case_db::complete_job(
        case_dir,
        &job.job_id,
        found as u64,
        "scan-deleted completed",
    )?;
    println!("deleted video scan complete (nothing read out yet)");
    println!(
        "deleted entries: {} (intact {}, partly overwritten {}, overwritten {}, unknown {})",
        summary.deleted_entries,
        summary.intact,
        summary.partial,
        summary.overwritten,
        summary.unknown
    );
    println!(
        "unallocated space scanned: {} bytes, carved candidates: {}",
        summary.unallocated_bytes, summary.carved
    );
    println!("headerless frame runs: {}", summary.frame_runs);
    println!(
        "allocated tails scanned: {} file(s), {} bytes, candidates found there: {}",
        summary.tail_files, summary.tail_bytes, summary.tail_candidates
    );
    println!("candidates: {}", summary.candidates_path.display());
    for warning in &summary.warnings {
        println!("warning: {warning}");
    }
    Ok(())
}

/// Repairs a broken/recovered video into `artifacts/Recovery/Repaired/`
/// ([`crate::video_repair`]); the source is never modified.
pub fn repair_video(case_dir: &Path, video: &Path) -> Result<(), String> {
    ensure_case(case_dir)?;
    crate::lazy_extract::ensure_materialized(case_dir, video)?;
    let source = crate::util::canonicalize_display(video)
        .map_err(|err| format!("failed to open {}: {err}", video.display()))?;
    let entry = crate::video_repair::repair_video(case_dir, &source)?;
    let field = |key: &str| entry.get(key).and_then(|v| v.as_str()).unwrap_or("-");
    let reused = entry.get("reused").and_then(|v| v.as_bool()) == Some(true);
    println!(
        "{} ({}): {}",
        if reused {
            "already repaired"
        } else {
            "repaired"
        },
        field("method"),
        field("output_path")
    );
    println!("sha256: {}", field("sha256"));
    println!("validation: {}", field("validation_status"));
    for note in entry
        .get("notes")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str())
    {
        println!("note: {note}");
    }
    Ok(())
}

/// One icat recovery of a batch; `slot` is its position in the outcomes.
struct RecoverTask {
    slot: usize,
    selector: String,
    action: &'static str,
    detail_folder: &'static str,
    options: TskRecoverOptions,
}

/// How many icat recoveries run at once. Each one decompresses the E01 on
/// its own core, so a solid-state source scales with cores; a spinning or
/// unidentified disk (USB bridges often do not say) gets two streams, which
/// overlaps decompression with I/O without thrashing the heads.
/// `FRAMETRACE_EXTRACT_JOBS` overrides.
fn recover_workers(image: &Path, tasks: usize) -> usize {
    let configured = std::env::var("FRAMETRACE_EXTRACT_JOBS")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok());
    let cores = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(4);
    workers_for(configured, crate::util::path_on_ssd(image), cores, tasks)
}

/// Pure part of [`recover_workers`].
fn workers_for(configured: Option<usize>, ssd: Option<bool>, cores: usize, tasks: usize) -> usize {
    let workers = match configured.filter(|jobs| *jobs > 0) {
        Some(jobs) => jobs,
        None if ssd == Some(true) => cores.clamp(2, 8),
        None => 2,
    };
    workers.clamp(1, 32).min(tasks.max(1))
}

/// Runs `tasks` on a worker pool and fills their `slots`. The image, icat
/// and the source registration are resolved once for the batch (they cost
/// a process launch and several fsynced log/DB writes per item before), and
/// the batch job reports done/total so the workstation shows real progress.
fn run_recover_tasks(
    case_dir: &Path,
    image_file: &Path,
    job_id: &str,
    tasks: &[RecoverTask],
    slots: &mut [Option<BatchOutcome>],
) {
    if tasks.is_empty() {
        return;
    }
    let failed = |task: &RecoverTask, detail: String| BatchOutcome {
        selector: task.selector.clone(),
        action: task.action,
        status: "failed",
        detail,
    };
    let setup = tsk::RecoverContext::new(image_file, "icat").and_then(|context| {
        case_db::register_evidence_source(
            case_dir,
            &case_db::EvidenceSourceInput {
                kind: "forensic-image".to_string(),
                path: image_file.to_path_buf(),
                source_id: None,
                write_protect: Some("source image read-only; inode output is derived".to_string()),
                acquisition_tool: Some("Sleuth Kit icat".to_string()),
                evidence_hash: None,
                notes: Some("Auto-registered by recover-batch".to_string()),
                metadata_json: None,
            },
        )?;
        Ok(context)
    });
    let context = match setup {
        Ok(context) => context,
        Err(err) => {
            for task in tasks {
                slots[task.slot] = Some(failed(task, err.clone()));
            }
            return;
        }
    };
    let mut folders = std::collections::BTreeSet::new();
    for task in tasks {
        if let Some(parent) = task.options.output_path.as_deref().and_then(Path::parent) {
            folders.insert(parent.to_path_buf());
        }
    }
    for folder in &folders {
        tsk::sweep_dead_parts(folder);
    }

    let total = tasks.len() as u64;
    let workers = recover_workers(context.image_path(), tasks.len());
    println!("recovering {total} item(s) with {workers} worker(s)");
    let next = std::sync::atomic::AtomicUsize::new(0);
    let done = std::sync::atomic::AtomicU64::new(0);
    let last_report = std::sync::Mutex::new(std::time::Instant::now());
    let results = std::sync::Mutex::new(Vec::with_capacity(tasks.len()));
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let Some(task) = tasks.get(index) else {
                        break;
                    };
                    let outcome =
                        match tsk::recover_inode_in(case_dir, &context, &task.options, false) {
                            Ok(result) => {
                                for warning in &result.warnings {
                                    eprintln!("warning: {}: {warning}", task.selector);
                                }
                                BatchOutcome {
                                    selector: task.selector.clone(),
                                    action: task.action,
                                    status: "ok",
                                    detail: format!("saved into {}", task.detail_folder),
                                }
                            }
                            Err(error) => failed(task, error),
                        };
                    results
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .push((task.slot, outcome));
                    let finished = done.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    // Throttled: one progress write a second, plus the last.
                    let mut last = last_report.lock().unwrap_or_else(|e| e.into_inner());
                    if finished == total || last.elapsed() >= std::time::Duration::from_secs(1) {
                        *last = std::time::Instant::now();
                        let _ =
                            case_db::report_job_progress(case_dir, job_id, Some(total), finished);
                    }
                }
            });
        }
    });
    for (slot, outcome) in results.into_inner().unwrap_or_else(|e| e.into_inner()) {
        slots[slot] = Some(outcome);
    }
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
    let extracted_root = crate::lazy_extract::extracted_root(case_dir);
    let extract_targets = plan_extract_targets(&selection.items, is_extract);
    let mut legacy_counts = std::collections::HashMap::new();
    for (item, target) in selection.items.iter().zip(&extract_targets) {
        if let (Some(_), Some(note)) = (target, item.notes.as_deref()) {
            *legacy_counts
                .entry(path_key(&legacy_extracted_relative_path(note)))
                .or_insert(0usize) += 1;
        }
    }
    // Plan every item first (skips are decided here), then run the icat
    // recoveries on a worker pool.
    let mut slots: Vec<Option<BatchOutcome>> = Vec::with_capacity(selection.items.len());
    let mut tasks: Vec<RecoverTask> = Vec::new();
    for (item, extract_target) in selection.items.iter().zip(extract_targets) {
        let inode = item
            .selector
            .trim()
            .strip_prefix("fls:")
            .unwrap_or(item.selector.trim())
            .to_string();

        let item_is_extract = extract_target.is_some();
        let (output_path, recover_deleted, detail_folder) = if let Some(rel) = extract_target {
            let target = extracted_root.join(rel);
            if !target.is_file()
                && let Some(legacy) =
                    legacy_extracted_file(&extracted_root, item.notes.as_deref(), &legacy_counts)
            {
                slots.push(Some(BatchOutcome {
                    selector: item.selector.clone(),
                    action: "extract",
                    status: "ok",
                    detail: format!("already exists (legacy name) at {}", legacy.display()),
                }));
                continue;
            }
            (target, false, "artifacts/extracted")
        } else {
            let filename = item
                .notes
                .as_deref()
                .and_then(|p| Path::new(p).file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("");
            let clean_name = filename.trim_end_matches("(deleted)").trim();
            let leaf = if clean_name.is_empty() {
                format!("inode_{}.bin", crate::util::sanitize_filename(&inode))
            } else {
                format!(
                    "{}_{}",
                    crate::util::sanitize_filename(&inode),
                    crate::util::sanitize_filename(clean_name)
                )
            };
            let target = case_dir.join("artifacts/Recovery/Filesystem").join(leaf);
            (target, true, "artifacts/Recovery/Filesystem")
        };

        if output_path.is_file()
            && std::fs::metadata(&output_path)
                .map(|m| m.len())
                .unwrap_or(0)
                > 0
        {
            slots.push(Some(BatchOutcome {
                selector: item.selector.clone(),
                action: if item_is_extract {
                    "extract"
                } else {
                    "recover"
                },
                status: "ok",
                detail: format!("already exists at {}", output_path.display()),
            }));
            continue;
        }

        tasks.push(RecoverTask {
            slot: slots.len(),
            selector: item.selector.clone(),
            action: if item_is_extract {
                "extract"
            } else {
                "recover"
            },
            detail_folder,
            options: TskRecoverOptions {
                partition_offset,
                inode,
                output_path: Some(output_path),
                recover_deleted,
                include_slack: false,
                skip_sparse_holes: true,
                icat_bin: "icat".to_string(),
                timeout_secs,
            },
        });
        slots.push(None);
    }
    run_recover_tasks(case_dir, image_file, &job.job_id, &tasks, &mut slots);
    let outcomes: Vec<BatchOutcome> = slots.into_iter().flatten().collect();

    let ok = outcomes.iter().filter(|o| o.status == "ok").count();
    let failed = outcomes.iter().filter(|o| o.status == "failed").count();
    let line = format!(
        "{{\"schema_version\":1,\"event\":\"recover-batch\",\"selection_path\":\"{}\",\"requested\":{},\"ok\":{},\"failed\":{},\"results\":{}}}",
        json_escape(&selection_label),
        outcomes.len(),
        ok,
        failed,
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
    for outcome in &outcomes {
        println!(
            "  [{}] {}: {}",
            outcome.status, outcome.selector, outcome.detail
        );
    }
    if failed > 0 {
        // Some items are missing from the result: the workstation must show
        // that as a warning, never as a clean pass (exit code 5).
        exit_partial(&format!(
            "{failed} of {} item(s) failed — see artifacts/logs/batch-log.jsonl",
            outcomes.len()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod extract_tests {
    use super::*;
    use crate::selection::SelectionItem;

    fn item(inode: &str, path: &str) -> SelectionItem {
        SelectionItem {
            selector: inode.to_string(),
            kind: Some("filesystem".to_string()),
            action: Some("extract".to_string()),
            format: None,
            time_seconds: None,
            notes: Some(path.to_string()),
        }
    }

    fn rel(path: &Option<PathBuf>) -> String {
        path.as_ref().unwrap().to_string_lossy().replace('\\', "/")
    }

    #[test]
    fn extracted_paths_keep_card_names_and_extensions() {
        let targets = plan_extract_targets(
            &[
                item("12550151", "INF/20261017_170617_INF_1.avi"),
                item("4103", "/주차/20260913_153310_PARK.MP4"),
                item("77", "EVENT/clip:1.mp4 (deleted)"),
            ],
            true,
        );
        assert_eq!(rel(&targets[0]), "INF/20261017_170617_INF_1.avi");
        assert_eq!(rel(&targets[1]), "주차/20260913_153310_PARK.MP4");
        assert_eq!(rel(&targets[2]), "EVENT/clip_1.mp4");
    }

    #[test]
    fn colliding_names_never_share_an_output() {
        // Old mapping: both became `___/a_avi` and the second was skipped as
        // "already exists" — a live video silently missing from the review.
        let targets = plan_extract_targets(
            &[
                item("10", "상시/a.avi"),
                item("11", "상시/A.AVI"),
                item("12", "x/b?.avi"),
                item("13", "x/b*.avi"),
                item("14", "x/c.avi"),
            ],
            true,
        );
        let names: Vec<String> = targets.iter().map(rel).collect();
        assert_eq!(names[0], "상시/a~10.avi");
        assert_eq!(names[1], "상시/A~11.AVI");
        assert_eq!(names[2], "x/b_~12.avi");
        assert_eq!(names[3], "x/b_~13.avi");
        assert_eq!(names[4], "x/c.avi");
        let unique: std::collections::HashSet<String> =
            names.iter().map(|name| name.to_lowercase()).collect();
        assert_eq!(unique.len(), names.len());
    }

    #[test]
    fn recover_items_get_no_extract_target() {
        let mut recover = item("5", "EVENT/x.avi");
        recover.action = Some("recover".to_string());
        let targets = plan_extract_targets(&[recover, item("6", "EVENT/y.avi")], false);
        assert!(targets[0].is_none());
        assert_eq!(rel(&targets[1]), "EVENT/y.avi");
    }

    #[test]
    fn worker_count_follows_storage_and_override() {
        // SSD: scale with cores, capped at 8.
        assert_eq!(workers_for(None, Some(true), 32, 1000), 8);
        assert_eq!(workers_for(None, Some(true), 4, 1000), 4);
        // Spinning or unknown disk: two streams.
        assert_eq!(workers_for(None, Some(false), 32, 1000), 2);
        assert_eq!(workers_for(None, None, 32, 1000), 2);
        // Override wins; 0 is ignored; never more workers than items.
        assert_eq!(workers_for(Some(5), None, 32, 1000), 5);
        assert_eq!(workers_for(Some(0), None, 32, 1000), 2);
        assert_eq!(workers_for(Some(99), None, 32, 1000), 32);
        assert_eq!(workers_for(None, Some(true), 32, 3), 3);
        assert_eq!(workers_for(None, None, 32, 0), 1);
    }

    #[test]
    fn legacy_extraction_is_reused_only_when_unambiguous() {
        let root = std::env::temp_dir().join(format!("ft-legacy-extract-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("EVENT")).unwrap();
        std::fs::write(root.join("EVENT/clip_avi"), b"RIFF....AVI ").unwrap();
        let key = path_key(&legacy_extracted_relative_path("EVENT/clip.avi"));
        let mut counts = std::collections::HashMap::new();
        counts.insert(key.clone(), 1usize);
        assert_eq!(
            legacy_extracted_file(&root, Some("EVENT/clip.avi"), &counts),
            Some(root.join("EVENT/clip_avi"))
        );
        counts.insert(key, 2);
        assert_eq!(
            legacy_extracted_file(&root, Some("EVENT/clip.avi"), &counts),
            None
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
