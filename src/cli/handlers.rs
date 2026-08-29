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
use crate::tsk::{self, TskInspectOptions, TskRecoverOptions};
use crate::util::{create_case_layout, now_unix, read_to_string, write_text};
use crate::validation::{self, ValidationOptions};
use crate::video_export::{self, ExportOptions};
use std::env;
use std::path::{Path, PathBuf};

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

    write_text(&case_dir.join("case.json"), &manifest.to_json())
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

pub fn make_review(case_dir: &Path) -> Result<(), String> {
    ensure_case(case_dir)?;
    let index_path = case_dir.join("db/video_index.json");
    let index_json = read_to_string(&index_path)
        .map_err(|err| format!("failed to read {}: {err}", index_path.display()))?;
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
    let evidence_viewer = html_report::render_evidence_viewer_html(
        &manifest_json,
        &index_json,
        &carve_log,
        &filesystem_log,
        &validation_log,
    );
    let evidence_viewer_path = case_dir.join("review/evidence-viewer.html");
    write_text(&evidence_viewer_path, &evidence_viewer)
        .map_err(|err| format!("failed to write evidence viewer html: {err}"))?;
    println!("review written: {}", review_path.display());
    println!(
        "evidence viewer written: {}",
        evidence_viewer_path.display()
    );
    Ok(())
}

pub fn make_report(case_dir: &Path) -> Result<(), String> {
    ensure_case(case_dir)?;
    let index_path = case_dir.join("db/video_index.json");
    let index_json = read_to_string(&index_path)
        .map_err(|err| format!("failed to read {}: {err}", index_path.display()))?;
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
    let html = report::render_case_report(&report::ReportInputs {
        manifest_json: &manifest_json,
        index_json: &index_json,
        export_log_jsonl: &export_log,
        proxy_log_jsonl: &proxy_log,
        thumbnail_log_jsonl: &thumbnail_log,
        carve_log_jsonl: &carve_log,
        filesystem_log_jsonl: &filesystem_log,
        validation_log_jsonl: &validation_log,
    });
    let report_path = case_dir.join("reports/case-report.html");
    write_text(&report_path, &html).map_err(|err| format!("failed to write report html: {err}"))?;
    println!("report written: {}", report_path.display());
    Ok(())
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
