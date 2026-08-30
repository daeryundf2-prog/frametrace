use crate::artifacts::{self, ProxyOptions, ThumbnailOptions};
use crate::audit;
use crate::carve::{self, CarveOptions};
use crate::case_db;
use crate::e01::{self, E01Options};
use crate::html_report;
use crate::model::{CaseManifest, ScanOptions};
use crate::package;
use crate::report;
use crate::scan;
use crate::tool_policy::require_case_output_path;
use crate::tsk::{self, TskInspectOptions, TskRecoverOptions};
use crate::util::{
    create_case_layout, json_escape, now_unix, read_to_string, write_text, write_text_atomic,
};
use crate::validation::{self, ValidationOptions};
use crate::video_export::{self, ExportOptions};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct InitCaseOptions {
    pub title: Option<String>,
    pub operator: Option<String>,
    pub device_id: Option<String>,
    pub device_serial: Option<String>,
    pub write_protect: Option<String>,
    pub acquisition_tool: Option<String>,
    pub evidence_hash: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RegisterSourceOptions {
    pub kind: String,
    pub source_id: Option<String>,
    pub write_protect: Option<String>,
    pub acquisition_tool: Option<String>,
    pub evidence_hash: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PackageOptions {
    pub output_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct BenchmarkOptions {
    pub rows: usize,
}

pub fn init_case(case_dir: &Path, options: &InitCaseOptions) -> Result<(), String> {
    create_case_layout(case_dir).map_err(|err| format!("failed to create case layout: {err}"))?;

    let created_unix = now_unix()?;
    let manifest = CaseManifest {
        schema_version: 1,
        case_id: format!("FT-{created_unix}"),
        title: options
            .title
            .clone()
            .unwrap_or_else(|| "Untitled FrameTrace case".to_string()),
        created_unix,
        tool_name: env!("CARGO_PKG_NAME").to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        platform: env::consts::OS.to_string(),
        operator: options.operator.clone().or_else(default_operator),
        host: default_host(),
        device_id: options.device_id.clone(),
        device_serial: options.device_serial.clone(),
        write_protect: options.write_protect.clone(),
        acquisition_tool: options.acquisition_tool.clone(),
        evidence_hash: options.evidence_hash.clone(),
        notes: options.notes.clone(),
    };

    write_text_atomic(&case_dir.join("case.json"), &manifest.to_json())
        .map_err(|err| format!("failed to write case manifest: {err}"))?;

    println!("case created: {}", case_dir.display());
    println!("case id: {}", manifest.case_id);
    Ok(())
}

pub fn scan_folder(case_dir: &Path, source_dir: &Path, options: ScanOptions) -> Result<(), String> {
    ensure_case(case_dir)?;
    if !source_dir.is_dir() {
        return Err(format!(
            "source is not a directory: {}",
            source_dir.display()
        ));
    }

    let source = case_db::register_evidence_source(
        case_dir,
        &case_db::EvidenceSourceInput {
            kind: "folder".to_string(),
            path: source_dir.to_path_buf(),
            source_id: None,
            write_protect: None,
            acquisition_tool: None,
            evidence_hash: None,
            notes: Some("Auto-registered by scan-folder".to_string()),
            metadata_json: Some(scan_options_json(&options)),
        },
    )?;
    let job = case_db::start_job(
        case_dir,
        "scan-folder",
        source_dir,
        None,
        &scan_options_json(&options),
    )?;
    let result = match scan::scan_folder(case_dir, source_dir, &options) {
        Ok(result) => result,
        Err(err) => {
            let _ = case_db::fail_job(case_dir, &job.job_id, &err);
            return Err(err);
        }
    };
    case_db::complete_job(
        case_dir,
        &job.job_id,
        result.video_count as u64,
        "scan-folder completed",
    )?;
    println!("scan complete");
    println!("source registered: {} ({})", source.source_id, source.kind);
    println!("job: {} ({})", job.job_id, job.job_type);
    println!("videos indexed: {}", result.video_count);
    println!("bytes indexed: {}", result.total_bytes);
    println!("index: {}", case_dir.join("db/video_index.json").display());
    println!("sqlite: {}", case_db::case_db_path(case_dir).display());
    Ok(())
}

pub fn register_source(
    case_dir: &Path,
    source_path: &Path,
    options: RegisterSourceOptions,
) -> Result<(), String> {
    ensure_case(case_dir)?;
    let row = case_db::register_evidence_source(
        case_dir,
        &case_db::EvidenceSourceInput {
            kind: options.kind,
            path: source_path.to_path_buf(),
            source_id: options.source_id,
            write_protect: options.write_protect,
            acquisition_tool: options.acquisition_tool,
            evidence_hash: options.evidence_hash,
            notes: options.notes,
            metadata_json: None,
        },
    )?;
    println!("source registered: {}", row.source_id);
    println!("kind: {}", row.kind);
    println!("path: {}", row.path);
    println!("sqlite: {}", case_db::case_db_path(case_dir).display());
    Ok(())
}

pub fn inspect_e01(case_dir: &Path, e01_file: &Path, options: E01Options) -> Result<(), String> {
    ensure_case(case_dir)?;
    e01::inspect_e01(case_dir, e01_file, &options)?;
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
    println!("source registered: {} ({})", row.source_id, row.kind);
    println!("source: {}", e01_file.display());
    println!(
        "audit log: {}",
        case_dir.join("evidence/logs/e01-audit.jsonl").display()
    );
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
    case_db::register_evidence_source(
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
    )?;
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
    println!(
        "next: carve-file {} {}",
        case_dir.display(),
        result.raw_output_path.display()
    );
    Ok(())
}

/// Remuxes a Dahua DAV container to MP4 (no re-encode), recording the
/// derived artifact in artifacts/clips/export-log.jsonl so validate-batch and
/// the viewer can consume the output by path. DAV parsing is pending
/// real-sample validation; outputs stay candidate until examiner review.
pub fn export_dav(
    case_dir: &Path,
    dav_file: &Path,
    output: Option<PathBuf>,
    timeout_secs: Option<u64>,
) -> Result<(), String> {
    ensure_case(case_dir)?;
    let job = case_db::start_job(case_dir, "export-dav", dav_file, None, "{}")?;

    let stem = dav_file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("dav");
    let requested_raw = output.unwrap_or_else(|| {
        crate::util::unique_path(&case_dir.join("artifacts/clips").join(format!("{stem}.mp4")))
    });
    require_case_output_path(case_dir, &requested_raw, "DAV export")?;
    if requested_raw.exists() {
        return Err(format!(
            "output already exists: {} (choose a new --output path)",
            requested_raw.display()
        ));
    }

    let es_path =
        crate::util::unique_path(&case_dir.join("artifacts/carved").join(format!("{stem}.es")));
    let (written, frames, channel) = crate::dav::extract_video_es(dav_file, &es_path)?;
    if let Err(error) = crate::dav::remux_es_to_mp4(&es_path, &requested_raw, timeout_secs) {
        let _ = std::fs::remove_file(&requested_raw);
        return Err(error);
    }
    let output_sha256 = audit::digest_file(&requested_raw)?;

    let line = format!(
        "{{\"schema_version\":1,\"event\":\"export-dav\",\"selector\":\"{}\",\"source_path\":\"{}\",\"format\":\"mp4\",\"output_path\":\"{}\",\"output_sha256\":\"{}\",\"es_path\":\"{}\",\"es_bytes\":{},\"video_frames\":{},\"channel\":{},\"container_validation\":\"pending-real-sample-validation\"}}",
        json_escape(stem),
        json_escape(&dav_file.to_string_lossy()),
        json_escape(&requested_raw.to_string_lossy()),
        json_escape(&output_sha256),
        json_escape(&es_path.to_string_lossy()),
        written,
        frames,
        channel
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string()),
    );
    audit::append_chained_jsonl(&case_dir.join("artifacts/clips/export-log.jsonl"), &line)?;
    case_db::complete_job(case_dir, &job.job_id, 1, "export-dav completed")?;

    println!("dav remux complete");
    println!(
        "video frames: {frames} · channel: {} · ES bytes: {written}",
        channel
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    println!("output: {}", requested_raw.display());
    println!("output sha256: {output_sha256}");
    println!("note: DAV parsing is pending real-sample validation; keep the original DAV.");
    Ok(())
}

pub fn make_review(case_dir: &Path) -> Result<(), String> {
    ensure_case(case_dir)?;
    let index_path = case_dir.join("db/video_index.json");
    let index_json = read_to_string(&index_path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            format!(
                "no case index yet at {} — run scan-folder (or import-e01) before make-review",
                index_path.display()
            )
        } else {
            format!("failed to read {}: {err}", index_path.display())
        }
    })?;
    let manifest_path = case_dir.join("case.json");
    let manifest_json = read_to_string(&manifest_path)
        .map_err(|err| format!("failed to read {}: {err}", manifest_path.display()))?;
    let html = html_report::render_review_html(&manifest_json, &index_json);
    let review_path = case_dir.join("review/index.html");
    write_text(&review_path, &html).map_err(|err| format!("failed to write review html: {err}"))?;
    let carve_log =
        read_to_string(&case_dir.join("artifacts/carved/carve-log.jsonl")).unwrap_or_default();
    let filesystem_log =
        read_to_string(&case_dir.join("evidence/logs/tsk-audit.jsonl")).unwrap_or_default();
    let validation_log =
        read_to_string(&case_dir.join("evidence/logs/validation-log.jsonl")).unwrap_or_default();
    let fls_entries = latest_fls_entries_jsonl(case_dir);
    let videos = collect_index_videos(&index_json);
    let (thumbs_json, thumb_stats) = generate_review_thumbnails(case_dir, &videos)?;
    let evidence_viewer = html_report::render_evidence_viewer_html(
        &manifest_json,
        &index_json,
        &carve_log,
        &filesystem_log,
        &validation_log,
        &fls_entries,
        &thumbs_json,
    );
    let evidence_viewer_path = case_dir.join("review/evidence-viewer.html");
    write_text(&evidence_viewer_path, &evidence_viewer)
        .map_err(|err| format!("failed to write evidence viewer html: {err}"))?;
    println!("review written: {}", review_path.display());
    println!(
        "evidence viewer written: {}",
        evidence_viewer_path.display()
    );
    println!(
        "thumbnails: {} created, {} cached, {} unavailable{}",
        thumb_stats.created,
        thumb_stats.cached,
        thumb_stats.skipped,
        if thumb_stats.ffmpeg_missing {
            " (ffmpeg not found; rerun with ffmpeg in PATH)"
        } else {
            ""
        }
    );
    Ok(())
}

pub fn make_report(case_dir: &Path) -> Result<(), String> {
    ensure_case(case_dir)?;
    let index_path = case_dir.join("db/video_index.json");
    let index_json = read_to_string(&index_path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            format!(
                "no case index yet at {} — run scan-folder (or import-e01) before make-review",
                index_path.display()
            )
        } else {
            format!("failed to read {}: {err}", index_path.display())
        }
    })?;
    let manifest_path = case_dir.join("case.json");
    let manifest_json = read_to_string(&manifest_path)
        .map_err(|err| format!("failed to read {}: {err}", manifest_path.display()))?;
    let export_log =
        read_to_string(&case_dir.join("artifacts/clips/export-log.jsonl")).unwrap_or_default();
    let proxy_log =
        read_to_string(&case_dir.join("artifacts/proxies/proxy-log.jsonl")).unwrap_or_default();
    let thumbnail_log = read_to_string(&case_dir.join("artifacts/thumbnails/thumbnail-log.jsonl"))
        .unwrap_or_default();
    let carve_log =
        read_to_string(&case_dir.join("artifacts/carved/carve-log.jsonl")).unwrap_or_default();
    let filesystem_log =
        read_to_string(&case_dir.join("evidence/logs/tsk-audit.jsonl")).unwrap_or_default();
    let validation_log =
        read_to_string(&case_dir.join("evidence/logs/validation-log.jsonl")).unwrap_or_default();
    let batch_log =
        read_to_string(&case_dir.join("artifacts/logs/batch-log.jsonl")).unwrap_or_default();
    let scan_runs_json = read_scan_runs_json(case_dir);
    let marks_json = match crate::case_db::load_review_marks(case_dir) {
        Ok(rows) => {
            let entries = rows
                .iter()
                .map(|mark| {
                    format!(
                        "{{\"id\":\"{}\",\"status\":\"{}\",\"marked_unix\":{}}}",
                        crate::util::json_escape(&mark.record_id),
                        crate::util::json_escape(&mark.status),
                        mark.marked_unix
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("[{entries}]")
        }
        Err(_) => "[]".to_string(),
    };
    let html = report::render_case_report(&report::ReportInputs {
        manifest_json: &manifest_json,
        index_json: &index_json,
        export_log_jsonl: &export_log,
        proxy_log_jsonl: &proxy_log,
        thumbnail_log_jsonl: &thumbnail_log,
        carve_log_jsonl: &carve_log,
        filesystem_log_jsonl: &filesystem_log,
        validation_log_jsonl: &validation_log,
        batch_log_jsonl: &batch_log,
        scan_runs_json: &scan_runs_json,
        marks_json: &marks_json,
    });
    let report_path = case_dir.join("reports/case-report.html");
    write_text(&report_path, &html).map_err(|err| format!("failed to write report html: {err}"))?;
    println!("report written: {}", report_path.display());
    Ok(())
}

/// Reads every db/scan_runs/*.json snapshot and returns them joined into a
/// JSON array literal for the report script.
fn read_scan_runs_json(case_dir: &Path) -> String {
    let runs_dir = case_dir.join("db/scan_runs");
    let Ok(entries) = std::fs::read_dir(&runs_dir) else {
        return "[]".to_string();
    };
    let mut runs: Vec<(String, String)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Ok(content) = read_to_string(&path) else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().to_string();
        runs.push((name, content));
    }
    runs.sort_by_key(|(name, _)| name.clone());
    let joined = runs
        .iter()
        .map(|(_, content)| content.trim())
        .collect::<Vec<_>>()
        .join(",");
    format!("[{joined}]")
}

pub fn package_case(case_dir: &Path, options: PackageOptions) -> Result<(), String> {
    ensure_case(case_dir)?;
    let result = package::package_case(case_dir, options.output_dir.as_deref())?;
    println!("case package written");
    println!("output: {}", result.output_dir.display());
    println!("files: {}", result.file_count);
    println!("manifest: {}", result.manifest_path.display());
    Ok(())
}

pub fn export_video(case_dir: &Path, selector: &str, options: ExportOptions) -> Result<(), String> {
    ensure_case(case_dir)?;
    let result = video_export::export_video(case_dir, selector, &options)?;
    println!("video exported");
    println!("source: {}", result.source_path.display());
    println!("output: {}", result.output_path.display());
    println!("format: {}", result.format.extension());
    Ok(())
}

pub fn make_proxy(case_dir: &Path, selector: &str, options: ProxyOptions) -> Result<(), String> {
    ensure_case(case_dir)?;
    let result = artifacts::generate_proxy(case_dir, selector, &options)?;
    println!("proxy generated");
    println!("source: {}", result.source_path.display());
    println!("output: {}", result.output_path.display());
    Ok(())
}

pub fn make_thumbnail(
    case_dir: &Path,
    selector: &str,
    options: ThumbnailOptions,
) -> Result<(), String> {
    ensure_case(case_dir)?;
    let result = artifacts::generate_thumbnail(case_dir, selector, &options)?;
    println!("thumbnail generated");
    println!("source: {}", result.source_path.display());
    println!("output: {}", result.output_path.display());
    Ok(())
}

pub fn carve_file(
    case_dir: &Path,
    source_file: &Path,
    options: CarveOptions,
) -> Result<(), String> {
    ensure_case(case_dir)?;
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
    let result = match carve::carve_file(case_dir, source_file, &options) {
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
    let result = match tsk::inspect_image(case_dir, image_file, &options) {
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
        "inspect-image completed",
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
    }
    Ok(())
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
    println!(
        "next: scan-folder {} {} --no-ffprobe",
        case_dir.display(),
        result
            .output_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .display()
    );
    Ok(())
}

pub fn validate_artifact(
    case_dir: &Path,
    selector: &str,
    options: ValidationOptions,
) -> Result<(), String> {
    ensure_case(case_dir)?;
    let job = case_db::start_job(
        case_dir,
        "validate-artifact",
        Path::new(selector),
        Some(1),
        &validation_options_json(&options),
    )?;
    let result = match validation::validate_artifact(case_dir, selector, &options) {
        Ok(result) => result,
        Err(err) => {
            let _ = case_db::fail_job(case_dir, &job.job_id, &err);
            return Err(err);
        }
    };
    case_db::complete_job(case_dir, &job.job_id, 1, "validate-artifact completed")?;
    println!("artifact validated");
    println!("job: {} ({})", job.job_id, job.job_type);
    println!("selector: {}", result.selector);
    println!("target: {}", result.target_path.display());
    println!("sha256: {}", result.target_sha256);
    println!("status: {}", result.validation_status);
    println!("note: {}", result.validation_note);
    if let Some(codec) = result.probe.video_codec {
        println!("video codec: {codec}");
    }
    if let Some(duration) = result.probe.duration_seconds {
        println!("duration seconds: {duration:.3}");
    }
    Ok(())
}

pub fn benchmark_db(output_dir: &Path, options: BenchmarkOptions) -> Result<(), String> {
    let result = case_db::benchmark_case_db(output_dir, options.rows)?;
    println!("SQLite benchmark complete");
    println!("rows: {}", result.rows);
    println!("elapsed_ms: {}", result.elapsed_ms);
    println!("db: {}", result.path.display());
    Ok(())
}

pub fn verify_audit(log_path: &Path) -> Result<(), String> {
    let result = audit::verify_chained_jsonl(log_path)?;
    println!("audit verified: {}", log_path.display());
    println!("entries: {}", result.entries);
    println!("last entry sha256: {}", result.last_entry_sha256);
    Ok(())
}

#[derive(Debug, Clone)]
struct BatchOutcome {
    selector: String,
    action: &'static str,
    status: &'static str,
    detail: String,
}

/// Selection items may reference indexed videos (vid_*), carved candidates
/// (carve_*), inode recoveries (inode:*), or direct paths. Indexed lookup
/// comes first; artifact-log resolution covers the rest.
fn resolve_batch_selector(case_dir: &Path, selector: &str) -> Result<PathBuf, String> {
    crate::video_export::resolve_video_source(case_dir, selector)
        .or_else(|_| crate::validation::resolve_artifact_path(case_dir, selector))
}

/// Names batch outputs after the selection item (not the resolved source
/// path, which would produce path-mangled filenames).
fn batch_output_path(
    case_dir: &Path,
    relative_dir: &str,
    selector: &str,
    extension: &str,
) -> Result<PathBuf, String> {
    let unix = now_unix()?;
    Ok(crate::util::unique_path(&case_dir.join(relative_dir).join(
        format!(
            "{}_{}.{}",
            crate::video_export::sanitize_filename(selector),
            unix,
            extension
        ),
    )))
}

pub fn export_batch(case_dir: &Path, selection_path: &Path, dry_run: bool) -> Result<(), String> {
    ensure_case(case_dir)?;
    let selection = crate::selection::parse_selection_file(selection_path)?;
    let job = case_db::start_job(
        case_dir,
        "export-batch",
        selection_path,
        Some(selection.items.len() as u64),
        &format!(
            "{{\"dry_run\":{dry_run},\"items\":{}}}",
            selection.items.len()
        ),
    )?;

    let mut outcomes = Vec::new();
    for item in &selection.items {
        let action = crate::selection::effective_action(item);
        let outcome = (|| -> Result<BatchOutcome, String> {
            match action {
                "export" => {
                    let format = crate::selection::effective_format(item)?;
                    let resolved = resolve_batch_selector(case_dir, &item.selector)?;
                    if dry_run {
                        Ok(BatchOutcome {
                            selector: item.selector.clone(),
                            action,
                            status: "dry-run-ok",
                            detail: format!("would export {} from {}", format, resolved.display()),
                        })
                    } else {
                        let options = crate::video_export::ExportOptions {
                            format: crate::video_export::ExportFormat::parse(format)?,
                            start_seconds: None,
                            duration_seconds: None,
                            output_path: Some(batch_output_path(
                                case_dir,
                                "artifacts/clips",
                                &item.selector,
                                format,
                            )?),
                            timeout_secs: None,
                        };
                        let selector = resolved.display().to_string();
                        let result = video_export::export_video(case_dir, &selector, &options)?;
                        Ok(BatchOutcome {
                            selector: item.selector.clone(),
                            action,
                            status: "ok",
                            detail: result.output_path.display().to_string(),
                        })
                    }
                }
                "proxy" => {
                    let resolved = resolve_batch_selector(case_dir, &item.selector)?;
                    if dry_run {
                        Ok(BatchOutcome {
                            selector: item.selector.clone(),
                            action,
                            status: "dry-run-ok",
                            detail: format!("would generate proxy from {}", resolved.display()),
                        })
                    } else {
                        let options = crate::artifacts::ProxyOptions {
                            output_path: Some(batch_output_path(
                                case_dir,
                                "artifacts/proxies",
                                &item.selector,
                                "mp4",
                            )?),
                            ..crate::artifacts::ProxyOptions::default()
                        };
                        let result = artifacts::generate_proxy(
                            case_dir,
                            &resolved.display().to_string(),
                            &options,
                        )?;
                        Ok(BatchOutcome {
                            selector: item.selector.clone(),
                            action,
                            status: "ok",
                            detail: result.output_path.display().to_string(),
                        })
                    }
                }
                "thumbnail" => {
                    let resolved = resolve_batch_selector(case_dir, &item.selector)?;
                    if dry_run {
                        Ok(BatchOutcome {
                            selector: item.selector.clone(),
                            action,
                            status: "dry-run-ok",
                            detail: format!("would generate thumbnail from {}", resolved.display()),
                        })
                    } else {
                        let options = crate::artifacts::ThumbnailOptions {
                            time_seconds: item.time_seconds.unwrap_or(0.0),
                            output_path: Some(batch_output_path(
                                case_dir,
                                "artifacts/thumbnails",
                                &item.selector,
                                "jpg",
                            )?),
                        };
                        let result = artifacts::generate_thumbnail(
                            case_dir,
                            &resolved.display().to_string(),
                            &options,
                        )?;
                        Ok(BatchOutcome {
                            selector: item.selector.clone(),
                            action,
                            status: "ok",
                            detail: result.output_path.display().to_string(),
                        })
                    }
                }
                _ => Ok(BatchOutcome {
                    selector: item.selector.clone(),
                    action,
                    status: "skipped",
                    detail: format!(
                        "action '{}' is not part of export-batch; use validate-batch",
                        action
                    ),
                }),
            }
        })();
        outcomes.push(match outcome {
            Ok(outcome) => outcome,
            Err(error) => BatchOutcome {
                selector: item.selector.clone(),
                action,
                status: "failed",
                detail: error,
            },
        });
    }

    let ok = outcomes
        .iter()
        .filter(|o| o.status == "ok" || o.status == "dry-run-ok")
        .count();
    let failed = outcomes.iter().filter(|o| o.status == "failed").count();
    let skipped = outcomes.iter().filter(|o| o.status == "skipped").count();

    if !dry_run {
        let line = format!(
            "{{\"schema_version\":1,\"event\":\"export-batch\",\"selection_path\":\"{}\",\"requested\":{},\"ok\":{},\"failed\":{},\"skipped\":{},\"results\":{}}}",
            json_escape(&selection_path.display().to_string()),
            outcomes.len(),
            ok,
            failed,
            skipped,
            outcomes_json(&outcomes),
        );
        audit::append_chained_jsonl(&case_dir.join("artifacts/logs/batch-log.jsonl"), &line)?;
    }

    case_db::complete_job(
        case_dir,
        &job.job_id,
        outcomes.len() as u64,
        if dry_run {
            "export-batch dry run"
        } else {
            "export-batch completed"
        },
    )?;

    println!(
        "export batch {}",
        if dry_run { "dry run" } else { "complete" }
    );
    println!("requested: {}", outcomes.len());
    println!("ok: {ok}");
    println!("failed: {failed}");
    println!("skipped: {skipped}");
    for outcome in &outcomes {
        println!(
            "  [{}] {} ({}): {}",
            outcome.status, outcome.selector, outcome.action, outcome.detail
        );
    }
    Ok(())
}

pub fn validate_batch(case_dir: &Path, selection_path: &Path) -> Result<(), String> {
    ensure_case(case_dir)?;
    let selection = crate::selection::parse_selection_file(selection_path)?;
    let job = case_db::start_job(
        case_dir,
        "validate-batch",
        selection_path,
        Some(selection.items.len() as u64),
        &format!("{{\"items\":{}}}", selection.items.len()),
    )?;

    // Compute phase runs in parallel (per-file SHA-256 + ffprobe dominate the
    // runtime); validation log appends then happen sequentially so the hash
    // chain stays ordered and verifiable.
    let options = ValidationOptions::default();
    let items = &selection.items;
    let slots: std::sync::Mutex<Vec<Option<Result<crate::validation::ValidationResult, String>>>> =
        std::sync::Mutex::new((0..items.len()).map(|_| None).collect());
    let next_index = std::sync::atomic::AtomicUsize::new(0);
    let items = &items;
    let slots = &slots;
    let next_index = &next_index;
    let workers = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(4)
        .clamp(1, 8);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let index = next_index.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let Some(item) = items.get(index) else { break };
                    let outcome =
                        crate::validation::compute_validation(case_dir, &item.selector, &options);
                    slots.lock().unwrap()[index] = Some(outcome);
                }
            });
        }
    });

    let mut outcomes = Vec::new();
    let results = slots.lock().unwrap().drain(..).collect::<Vec<_>>();
    for (item, slot) in selection.items.iter().zip(results) {
        let computed = slot.unwrap_or_else(|| Err("validation worker lost its result".to_string()));
        outcomes.push(match computed {
            Ok(result) => match validation::append_validation_log(case_dir, &result, &options) {
                Ok(()) => BatchOutcome {
                    selector: item.selector.clone(),
                    action: "validate",
                    status: if result.validation_status == "validation-failed" {
                        "failed"
                    } else {
                        "ok"
                    },
                    detail: format!("{} ({})", result.validation_status, result.target_sha256),
                },
                Err(error) => BatchOutcome {
                    selector: item.selector.clone(),
                    action: "validate",
                    status: "failed",
                    detail: error,
                },
            },
            Err(error) => BatchOutcome {
                selector: item.selector.clone(),
                action: "validate",
                status: "failed",
                detail: error,
            },
        });
    }

    let ok = outcomes.iter().filter(|o| o.status == "ok").count();
    let failed = outcomes.iter().filter(|o| o.status == "failed").count();
    let line = format!(
        "{{\"schema_version\":1,\"event\":\"validate-batch\",\"selection_path\":\"{}\",\"requested\":{},\"ok\":{},\"failed\":{},\"results\":{}}}",
        json_escape(&selection_path.display().to_string()),
        outcomes.len(),
        ok,
        failed,
        outcomes_json(&outcomes),
    );
    audit::append_chained_jsonl(&case_dir.join("artifacts/logs/batch-log.jsonl"), &line)?;
    case_db::complete_job(
        case_dir,
        &job.job_id,
        outcomes.len() as u64,
        "validate-batch completed",
    )?;

    println!("validate batch complete");
    println!("requested: {}", outcomes.len());
    println!("ok: {ok}");
    println!("failed: {failed}");
    for outcome in &outcomes {
        println!(
            "  [{}] {}: {}",
            outcome.status, outcome.selector, outcome.detail
        );
    }
    Ok(())
}

/// Batch-recovers viewer-selected deleted inodes (kind "candidate") from a
/// raw image. Each recovery appends its own tsk-audit entry; the batch outcome
/// is chained into artifacts/logs/batch-log.jsonl.
pub fn recover_batch(
    case_dir: &Path,
    image_file: &Path,
    selection_path: &Path,
    partition_offset: u64,
    timeout_secs: Option<u64>,
) -> Result<(), String> {
    ensure_case(case_dir)?;
    let selection = crate::selection::parse_selection_file(selection_path)?;
    let job = case_db::start_job(
        case_dir,
        "recover-batch",
        selection_path,
        Some(selection.items.len() as u64),
        &format!("{{\"items\":{}}}", selection.items.len()),
    )?;

    let mut outcomes = Vec::new();
    for item in &selection.items {
        let inode = item
            .selector
            .trim()
            .strip_prefix("fls:")
            .unwrap_or(item.selector.trim())
            .to_string();
        let options = TskRecoverOptions {
            partition_offset,
            inode,
            output_path: None,
            recover_deleted: true,
            include_slack: false,
            skip_sparse_holes: true,
            icat_bin: "icat".to_string(),
            timeout_secs,
        };
        let outcome = recover_inode(case_dir, image_file, options);
        outcomes.push(match outcome {
            Ok(()) => BatchOutcome {
                selector: item.selector.clone(),
                action: "recover",
                status: "ok",
                detail: "recovered into artifacts/recovered/filesystem".to_string(),
            },
            Err(error) => BatchOutcome {
                selector: item.selector.clone(),
                action: "recover",
                status: "failed",
                detail: error,
            },
        });
    }

    let ok = outcomes.iter().filter(|o| o.status == "ok").count();
    let failed = outcomes.iter().filter(|o| o.status == "failed").count();
    let line = format!(
        "{{\"schema_version\":1,\"event\":\"recover-batch\",\"selection_path\":\"{}\",\"requested\":{},\"ok\":{},\"failed\":{},\"results\":{}}}",
        json_escape(&selection_path.display().to_string()),
        outcomes.len(),
        ok,
        failed,
        outcomes_json(&outcomes),
    );
    audit::append_chained_jsonl(&case_dir.join("artifacts/logs/batch-log.jsonl"), &line)?;
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
    Ok(())
}

pub fn import_marks(case_dir: &Path, marks_path: &Path) -> Result<(), String> {
    ensure_case(case_dir)?;
    let marks_file = crate::selection::parse_marks_file(marks_path)?;
    let rows = marks_file
        .marks
        .iter()
        .map(|entry| case_db::ReviewMarkRow {
            record_id: entry.id.clone(),
            status: entry.status.clone(),
            marked_unix: entry.marked_unix.unwrap_or_else(|| now_unix().unwrap_or(0)),
            record_path: None,
            examiner: None,
        })
        .collect::<Vec<_>>();
    let stored = case_db::upsert_review_marks(case_dir, &rows)?;
    println!("marks imported");
    println!("source: {}", marks_path.display());
    println!("marks stored: {stored}");
    println!("sqlite: {}", case_db::case_db_path(case_dir).display());
    Ok(())
}

pub fn export_marks(case_dir: &Path, output: Option<&Path>) -> Result<(), String> {
    ensure_case(case_dir)?;
    let marks = case_db::load_review_marks(case_dir)?;
    let entries = marks
        .iter()
        .map(|mark| {
            format!(
                "{{\"id\":\"{}\",\"status\":\"{}\",\"marked_unix\":{}}}",
                crate::util::json_escape(&mark.record_id),
                crate::util::json_escape(&mark.status),
                mark.marked_unix
            )
        })
        .collect::<Vec<_>>()
        .join(",\n    ");
    let text = format!(
        "{{\n  \"schema_version\": 1,\n  \"case_id\": null,\n  \"exported_unix\": {},\n  \"marks\": [\n    {}\n  ]\n}}\n",
        now_unix()?,
        entries
    );
    let output_path = output
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| case_dir.join("db/review-marks.json"));
    write_text(&output_path, &text)
        .map_err(|err| format!("failed to write marks export: {err}"))?;
    println!("marks exported");
    println!("marks: {}", marks.len());
    println!("output: {}", output_path.display());
    Ok(())
}

fn outcomes_json(outcomes: &[BatchOutcome]) -> String {
    let items = outcomes
        .iter()
        .map(|outcome| {
            format!(
                "{{\"selector\":\"{}\",\"action\":\"{}\",\"status\":\"{}\",\"detail\":\"{}\"}}",
                crate::util::json_escape(&outcome.selector),
                crate::util::json_escape(outcome.action),
                crate::util::json_escape(outcome.status),
                crate::util::json_escape(&outcome.detail),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

pub fn inspect(case_dir: &Path) -> Result<(), String> {
    ensure_case(case_dir)?;
    let manifest_path = case_dir.join("case.json");
    let index_path = case_dir.join("db/video_index.json");
    println!("case: {}", case_dir.display());
    println!("manifest: {}", manifest_path.display());
    if index_path.exists() {
        let text = read_to_string(&index_path)
            .map_err(|err| format!("failed to read {}: {err}", index_path.display()))?;
        println!("index: {}", index_path.display());
        println!(
            "videos indexed: {}",
            extract_json_number(&text, "video_count").unwrap_or_else(|| "unknown".to_string())
        );
        println!(
            "bytes indexed: {}",
            extract_json_number(&text, "total_bytes").unwrap_or_else(|| "unknown".to_string())
        );
    } else {
        println!("index: not created yet");
    }
    match case_db::summarize_case_db(case_dir)? {
        Some(summary) => {
            println!("sqlite: {}", summary.path.display());
            println!("sqlite videos: {}", summary.video_count);
            println!("sqlite scan runs: {}", summary.scan_run_count);
            println!("sqlite evidence sources: {}", summary.evidence_source_count);
            println!("sqlite jobs: {}", summary.job_count);
            println!("sqlite active jobs: {}", summary.active_job_count);
        }
        None => println!("sqlite: not created yet"),
    }
    Ok(())
}

/// (id, source path) pairs for every video in the case index.
fn collect_index_videos(index_json: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(items) = crate::selection::json_array_field(index_json, "videos") {
        for object in crate::selection::json_objects_in_array(&items) {
            if let (Some(id), Some(source)) = (
                crate::selection::json_string_field(&object, "id"),
                crate::selection::json_string_field(&object, "source_path"),
            ) {
                out.push((id, source));
            }
        }
    }
    out
}

#[derive(Debug, Default)]
struct ThumbnailStats {
    created: usize,
    cached: usize,
    skipped: usize,
    ffmpeg_missing: bool,
}

/// Generates a representative frame per video into review/thumbs/<id>.jpg for
/// the viewer's thumbnail grid. ffmpeg is optional: without it the viewer
/// shows placeholders. Existing thumbs newer than their source are reused.
fn generate_review_thumbnails(
    case_dir: &Path,
    videos: &[(String, String)],
) -> Result<(String, ThumbnailStats), String> {
    let mut stats = ThumbnailStats::default();
    let thumbs_dir = case_dir.join("review/thumbs");
    std::fs::create_dir_all(&thumbs_dir)
        .map_err(|err| format!("failed to create thumbnail directory: {err}"))?;

    let ffmpeg = match crate::tool_policy::resolve_tool_binary("ffmpeg", &["ffmpeg"]) {
        Ok(binary) => binary,
        Err(_) => {
            stats.ffmpeg_missing = true;
            stats.skipped = videos.len();
            return Ok(("{}".to_string(), stats));
        }
    };

    let mut map = std::collections::BTreeMap::new();
    // Fresh thumbnails are resolved up front; the ffmpeg extraction runs are
    // then fanned out across a small worker pool (extraction dominates the
    // runtime on multi-thousand-record cases, and each invocation is an
    // independent subprocess so parallelism is safe here — audit log appends
    // are not part of this loop).
    let mut cached_ids = Vec::new();
    let mut pending: Vec<(&String, &String)> = Vec::new();
    for (id, source) in videos {
        let output = thumbs_dir.join(format!("{id}.jpg"));
        if thumbnail_is_fresh(Path::new(source), &output) {
            cached_ids.push(id.clone());
        } else {
            pending.push((id, source));
        }
    }
    stats.cached = cached_ids.len();

    let created_ids: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    let skipped = std::sync::atomic::AtomicUsize::new(0);
    let next_index = std::sync::atomic::AtomicUsize::new(0);
    let workers = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(4)
        .clamp(1, 8);
    let pending = &pending;
    let created_ids = &created_ids;
    let skipped = &skipped;
    let next_index = &next_index;
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let index = next_index.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let Some((id, source)) = pending.get(index) else {
                        break;
                    };
                    let output = thumbs_dir.join(format!("{id}.jpg"));
                    let mut created = false;
                    for seek in ["5", "0"] {
                        let result = Command::new(&ffmpeg)
                            .args([
                                "-y",
                                "-loglevel",
                                "error",
                                "-ss",
                                seek,
                                "-i",
                                source,
                                "-frames:v",
                                "1",
                                "-vf",
                                "scale=288:-2",
                                "-q:v",
                                "5",
                            ])
                            .arg(&output)
                            .output();
                        match result {
                            Ok(result) if result.status.success() => {
                                created = true;
                                break;
                            }
                            _ => {
                                let _ = std::fs::remove_file(&output);
                            }
                        }
                    }
                    if created {
                        created_ids.lock().unwrap().push((*id).clone());
                    } else {
                        skipped.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                }
            });
        }
    });
    stats.created = created_ids.lock().unwrap().len();
    stats.skipped = skipped.load(std::sync::atomic::Ordering::SeqCst);
    map.extend(
        cached_ids
            .into_iter()
            .map(|id| (id.clone(), format!("thumbs/{id}.jpg"))),
    );
    map.extend(
        created_ids
            .lock()
            .unwrap()
            .iter()
            .map(|id| (id.clone(), format!("thumbs/{id}.jpg"))),
    );

    let entries = map
        .iter()
        .map(|(id, path)| {
            format!(
                "\"{}\":\"{}\"",
                crate::util::json_escape(id),
                crate::util::json_escape(path)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    Ok((format!("{{{entries}}}"), stats))
}

fn thumbnail_is_fresh(source: &Path, thumb: &Path) -> bool {
    let (Ok(source_meta), Ok(thumb_meta)) = (std::fs::metadata(source), std::fs::metadata(thumb))
    else {
        return false;
    };
    let (Ok(source_time), Ok(thumb_time)) = (source_meta.modified(), thumb_meta.modified()) else {
        return false;
    };
    thumb_time >= source_time
}

/// Recovers-inode outputs are named inode_*.bin, but the Sleuth Kit listing
/// keeps each inode's original path (with recorder timestamps and channels).
/// Pass the newest listing to the viewer so recovered rows can show original
/// names and recording times.
fn latest_fls_entries_jsonl(case_dir: &Path) -> String {
    let entries_dir = case_dir.join("db/filesystem");
    let Ok(entries) = std::fs::read_dir(&entries_dir) else {
        return String::new();
    };
    let best = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("tsk-files-") && name.ends_with(".jsonl"))
        })
        .max();
    let Some(path) = best else {
        return String::new();
    };
    read_to_string(&path).unwrap_or_default()
}

fn ensure_case(case_dir: &Path) -> Result<(), String> {
    if !case_dir.join("case.json").is_file() {
        return Err(format!(
            "not a case directory: {} (run init-case first)",
            case_dir.display()
        ));
    }
    Ok(())
}

fn extract_json_number(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":");
    let start = text.find(&needle)? + needle.len();
    let rest = text[start..].trim_start();
    let end = rest
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(rest.len());
    if end == 0 {
        None
    } else {
        Some(rest[..end].to_string())
    }
}

fn e01_options_json(options: &E01Options) -> String {
    format!(
        "{{\"output_path\":{},\"max_bytes\":{},\"skip_verify\":{},\"hash_e01\":{}}}",
        options
            .output_path
            .as_ref()
            .map(|path| format!("\"{}\"", crate::util::json_escape(&path.to_string_lossy())))
            .unwrap_or_else(|| "null".to_string()),
        options
            .max_bytes
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string()),
        options.skip_verify,
        options.hash_e01
    )
}

fn carve_options_json(options: &CarveOptions) -> String {
    format!(
        "{{\"max_bytes\":{},\"max_candidates\":{}}}",
        options.max_bytes, options.max_candidates
    )
}

fn tsk_inspect_options_json(options: &TskInspectOptions) -> String {
    format!(
        "{{\"partition_offset\":{},\"max_entries\":{},\"mmls_bin\":\"{}\",\"fls_bin\":\"{}\"}}",
        options
            .partition_offset
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string()),
        options.max_entries,
        crate::util::json_escape(&options.mmls_bin),
        crate::util::json_escape(&options.fls_bin)
    )
}

fn tsk_recover_options_json(options: &TskRecoverOptions) -> String {
    format!(
        "{{\"partition_offset\":{},\"inode\":\"{}\",\"output_path\":{},\"recover_deleted\":{},\"include_slack\":{},\"skip_sparse_holes\":{},\"icat_bin\":\"{}\"}}",
        options.partition_offset,
        crate::util::json_escape(&options.inode),
        options
            .output_path
            .as_ref()
            .map(|path| format!("\"{}\"", crate::util::json_escape(&path.to_string_lossy())))
            .unwrap_or_else(|| "null".to_string()),
        options.recover_deleted,
        options.include_slack,
        options.skip_sparse_holes,
        crate::util::json_escape(&options.icat_bin)
    )
}

fn validation_options_json(options: &ValidationOptions) -> String {
    format!(
        "{{\"ffprobe_bin\":\"{}\"}}",
        crate::util::json_escape(&options.ffprobe_bin)
    )
}

fn scan_options_json(options: &ScanOptions) -> String {
    format!(
        "{{\"hash_files\":{},\"use_ffprobe\":{},\"max_depth\":{}}}",
        options.hash_files,
        options.use_ffprobe,
        options
            .max_depth
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string())
    )
}

fn default_operator() -> Option<String> {
    env::var("USERNAME")
        .ok()
        .or_else(|| env::var("USER").ok())
        .filter(|value| !value.trim().is_empty())
}

fn default_host() -> Option<String> {
    env::var("COMPUTERNAME")
        .ok()
        .or_else(|| env::var("HOSTNAME").ok())
        .filter(|value| !value.trim().is_empty())
}
