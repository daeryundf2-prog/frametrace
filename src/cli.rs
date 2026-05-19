use crate::artifacts::{self, ProxyOptions, ThumbnailOptions};
use crate::carve::{self, CarveOptions};
use crate::html_report;
use crate::model::{CaseManifest, ScanOptions};
use crate::report;
use crate::scan;
use crate::util::{create_case_layout, now_unix, read_to_string, write_text};
use crate::video_export::{self, ExportFormat, ExportOptions};
use std::env;
use std::path::{Path, PathBuf};

const INIT_CASE_USAGE: &str = "init-case <case-dir> [--title <title>] [--operator <name>] [--device-id <id>] [--device-serial <serial>] [--write-protect <state>] [--acquisition-tool <tool>] [--evidence-hash <sha256>] [--notes <text>]";

pub fn run(args: Vec<String>) -> Result<(), String> {
    let mut args = args.into_iter();
    let _program = args.next();
    let Some(command) = args.next() else {
        print_help();
        return Ok(());
    };

    match command.as_str() {
        "init-case" => {
            let case_dir = args.next().ok_or_else(|| INIT_CASE_USAGE.to_string())?;
            let options = parse_init_case_options(args.collect())?;
            init_case(Path::new(&case_dir), &options)
        }
        "scan-folder" => {
            let case_dir = args.next().ok_or_else(|| {
                "usage: scan-folder <case-dir> <source-dir> [--hash] [--no-ffprobe] [--max-depth <n>]"
                    .to_string()
            })?;
            let source_dir = args.next().ok_or_else(|| {
                "usage: scan-folder <case-dir> <source-dir> [--hash] [--no-ffprobe] [--max-depth <n>]"
                    .to_string()
            })?;
            let options = parse_scan_options(args.collect())?;
            scan_folder(Path::new(&case_dir), Path::new(&source_dir), options)
        }
        "make-review" => {
            let case_dir = args
                .next()
                .ok_or_else(|| "usage: make-review <case-dir>".to_string())?;
            make_review(Path::new(&case_dir))
        }
        "make-report" => {
            let case_dir = args
                .next()
                .ok_or_else(|| "usage: make-report <case-dir>".to_string())?;
            make_report(Path::new(&case_dir))
        }
        "export-video" => {
            let case_dir = args.next().ok_or_else(|| {
                "usage: export-video <case-dir> <video-id|path> --format <mp4|avi> [--start <seconds>] [--duration <seconds>] [--output <path>]".to_string()
            })?;
            let selector = args.next().ok_or_else(|| {
                "usage: export-video <case-dir> <video-id|path> --format <mp4|avi> [--start <seconds>] [--duration <seconds>] [--output <path>]".to_string()
            })?;
            let options = parse_export_options(args.collect())?;
            export_video(Path::new(&case_dir), &selector, options)
        }
        "make-proxy" => {
            let case_dir = args.next().ok_or_else(|| {
                "usage: make-proxy <case-dir> <video-id|path> [--max-width <pixels>] [--output <path>]".to_string()
            })?;
            let selector = args.next().ok_or_else(|| {
                "usage: make-proxy <case-dir> <video-id|path> [--max-width <pixels>] [--output <path>]".to_string()
            })?;
            let options = parse_proxy_options(args.collect())?;
            make_proxy(Path::new(&case_dir), &selector, options)
        }
        "make-thumbnail" => {
            let case_dir = args.next().ok_or_else(|| {
                "usage: make-thumbnail <case-dir> <video-id|path> [--time <seconds>] [--output <path>]".to_string()
            })?;
            let selector = args.next().ok_or_else(|| {
                "usage: make-thumbnail <case-dir> <video-id|path> [--time <seconds>] [--output <path>]".to_string()
            })?;
            let options = parse_thumbnail_options(args.collect())?;
            make_thumbnail(Path::new(&case_dir), &selector, options)
        }
        "carve-file" => {
            let case_dir = args.next().ok_or_else(|| {
                "usage: carve-file <case-dir> <source-file> [--max-bytes <n>] [--max-candidates <n>]".to_string()
            })?;
            let source_file = args.next().ok_or_else(|| {
                "usage: carve-file <case-dir> <source-file> [--max-bytes <n>] [--max-candidates <n>]".to_string()
            })?;
            let options = parse_carve_options(args.collect())?;
            carve_file(Path::new(&case_dir), Path::new(&source_file), options)
        }
        "inspect" => {
            let case_dir = args
                .next()
                .ok_or_else(|| "usage: inspect <case-dir>".to_string())?;
            inspect(Path::new(&case_dir))
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => Err(format!("unknown command: {other}")),
    }
}

#[derive(Debug, Clone, Default)]
struct InitCaseOptions {
    title: Option<String>,
    operator: Option<String>,
    device_id: Option<String>,
    device_serial: Option<String>,
    write_protect: Option<String>,
    acquisition_tool: Option<String>,
    evidence_hash: Option<String>,
    notes: Option<String>,
}

fn init_case(case_dir: &Path, options: &InitCaseOptions) -> Result<(), String> {
    create_case_layout(case_dir).map_err(|err| format!("failed to create case layout: {err}"))?;

    let case_id = format!("FT-{}", now_unix()?);
    let manifest = CaseManifest {
        schema_version: 1,
        case_id,
        title: options
            .title
            .clone()
            .unwrap_or_else(|| "Untitled FrameTrace case".to_string()),
        created_unix: now_unix()?,
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

fn scan_folder(case_dir: &Path, source_dir: &Path, options: ScanOptions) -> Result<(), String> {
    ensure_case(case_dir)?;
    if !source_dir.is_dir() {
        return Err(format!(
            "source is not a directory: {}",
            source_dir.display()
        ));
    }

    let result = scan::scan_folder(case_dir, source_dir, &options)?;
    println!("scan complete");
    println!("videos indexed: {}", result.video_count);
    println!("bytes indexed: {}", result.total_bytes);
    println!("index: {}", case_dir.join("db/video_index.json").display());
    Ok(())
}

fn make_review(case_dir: &Path) -> Result<(), String> {
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
    println!("review written: {}", review_path.display());
    Ok(())
}

fn make_report(case_dir: &Path) -> Result<(), String> {
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
    let html = report::render_case_report(
        &manifest_json,
        &index_json,
        &export_log,
        &proxy_log,
        &thumbnail_log,
        &carve_log,
    );
    let report_path = case_dir.join("reports/case-report.html");
    write_text(&report_path, &html).map_err(|err| format!("failed to write report html: {err}"))?;
    println!("report written: {}", report_path.display());
    Ok(())
}

fn export_video(case_dir: &Path, selector: &str, options: ExportOptions) -> Result<(), String> {
    ensure_case(case_dir)?;
    let result = video_export::export_video(case_dir, selector, &options)?;
    println!("video exported");
    println!("source: {}", result.source_path.display());
    println!("output: {}", result.output_path.display());
    println!("format: {}", result.format.extension());
    Ok(())
}

fn make_proxy(case_dir: &Path, selector: &str, options: ProxyOptions) -> Result<(), String> {
    ensure_case(case_dir)?;
    let result = artifacts::generate_proxy(case_dir, selector, &options)?;
    println!("proxy generated");
    println!("source: {}", result.source_path.display());
    println!("output: {}", result.output_path.display());
    Ok(())
}

fn make_thumbnail(
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

fn carve_file(case_dir: &Path, source_file: &Path, options: CarveOptions) -> Result<(), String> {
    ensure_case(case_dir)?;
    let result = carve::carve_file(case_dir, source_file, &options)?;
    println!("carve complete");
    println!("source: {}", result.source_path.display());
    println!("artifacts carved: {}", result.artifacts.len());
    println!(
        "results: {}",
        case_dir.join("db/carve_results.json").display()
    );
    Ok(())
}

fn inspect(case_dir: &Path) -> Result<(), String> {
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

fn parse_init_case_options(args: Vec<String>) -> Result<InitCaseOptions, String> {
    let mut options = InitCaseOptions::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--title" => {
                options.title = Some(next_option_value(&mut args, "--title")?);
            }
            "--operator" => options.operator = Some(next_option_value(&mut args, "--operator")?),
            "--device-id" => options.device_id = Some(next_option_value(&mut args, "--device-id")?),
            "--device-serial" => {
                options.device_serial = Some(next_option_value(&mut args, "--device-serial")?)
            }
            "--write-protect" => {
                options.write_protect = Some(next_option_value(&mut args, "--write-protect")?)
            }
            "--acquisition-tool" => {
                options.acquisition_tool = Some(next_option_value(&mut args, "--acquisition-tool")?)
            }
            "--evidence-hash" => {
                options.evidence_hash = Some(next_option_value(&mut args, "--evidence-hash")?)
            }
            "--notes" => options.notes = Some(next_option_value(&mut args, "--notes")?),
            other => return Err(format!("unknown init-case option: {other}")),
        }
    }
    Ok(options)
}

fn next_option_value(
    args: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<String, String> {
    args.next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{option} requires a value"))
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

fn parse_scan_options(args: Vec<String>) -> Result<ScanOptions, String> {
    let mut options = ScanOptions::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--hash" => options.hash_files = true,
            "--no-ffprobe" => options.use_ffprobe = false,
            "--max-depth" => {
                let raw = args
                    .next()
                    .ok_or_else(|| "--max-depth requires a value".to_string())?;
                options.max_depth = Some(
                    raw.parse::<usize>()
                        .map_err(|_| format!("invalid --max-depth value: {raw}"))?,
                );
            }
            other => return Err(format!("unknown scan-folder option: {other}")),
        }
    }
    Ok(options)
}

fn parse_export_options(args: Vec<String>) -> Result<ExportOptions, String> {
    let mut format = None;
    let mut start_seconds = None;
    let mut duration_seconds = None;
    let mut output_path = None;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--format" => {
                let raw = args
                    .next()
                    .ok_or_else(|| "--format requires mp4 or avi".to_string())?;
                format = Some(ExportFormat::parse(&raw)?);
            }
            "--start" => {
                let raw = args
                    .next()
                    .ok_or_else(|| "--start requires seconds".to_string())?;
                start_seconds = Some(parse_seconds("--start", &raw)?);
            }
            "--duration" => {
                let raw = args
                    .next()
                    .ok_or_else(|| "--duration requires seconds".to_string())?;
                duration_seconds = Some(parse_seconds("--duration", &raw)?);
            }
            "--output" => {
                output_path = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--output requires a path".to_string())?,
                ));
            }
            other => return Err(format!("unknown export-video option: {other}")),
        }
    }

    Ok(ExportOptions {
        format: format.ok_or_else(|| "--format <mp4|avi> is required".to_string())?,
        start_seconds,
        duration_seconds,
        output_path,
    })
}

fn parse_proxy_options(args: Vec<String>) -> Result<ProxyOptions, String> {
    let mut options = ProxyOptions::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--max-width" => {
                let raw = args
                    .next()
                    .ok_or_else(|| "--max-width requires pixels".to_string())?;
                options.max_width = raw
                    .parse::<u32>()
                    .map_err(|_| format!("invalid --max-width value: {raw}"))?;
            }
            "--output" => {
                options.output_path = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--output requires a path".to_string())?,
                ));
            }
            other => return Err(format!("unknown make-proxy option: {other}")),
        }
    }
    Ok(options)
}

fn parse_thumbnail_options(args: Vec<String>) -> Result<ThumbnailOptions, String> {
    let mut options = ThumbnailOptions::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--time" => {
                let raw = args
                    .next()
                    .ok_or_else(|| "--time requires seconds".to_string())?;
                options.time_seconds = parse_seconds("--time", &raw)?;
            }
            "--output" => {
                options.output_path = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--output requires a path".to_string())?,
                ));
            }
            other => return Err(format!("unknown make-thumbnail option: {other}")),
        }
    }
    Ok(options)
}

fn parse_carve_options(args: Vec<String>) -> Result<CarveOptions, String> {
    let mut options = CarveOptions::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--max-bytes" => {
                let raw = args
                    .next()
                    .ok_or_else(|| "--max-bytes requires a value".to_string())?;
                options.max_bytes = raw
                    .parse::<u64>()
                    .map_err(|_| format!("invalid --max-bytes value: {raw}"))?;
            }
            "--max-candidates" => {
                let raw = args
                    .next()
                    .ok_or_else(|| "--max-candidates requires a value".to_string())?;
                options.max_candidates = raw
                    .parse::<usize>()
                    .map_err(|_| format!("invalid --max-candidates value: {raw}"))?;
            }
            other => return Err(format!("unknown carve-file option: {other}")),
        }
    }
    Ok(options)
}

fn parse_seconds(label: &str, raw: &str) -> Result<f64, String> {
    let value = raw
        .parse::<f64>()
        .map_err(|_| format!("invalid {label} value: {raw}"))?;
    if value.is_sign_negative() || !value.is_finite() {
        return Err(format!("{label} must be a non-negative finite number"));
    }
    Ok(value)
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

fn print_help() {
    println!(
        "\
FrameTrace

Commands:
  {INIT_CASE_USAGE}
      Create a local forensic case folder.

  scan-folder <case-dir> <source-dir> [--hash] [--no-ffprobe] [--max-depth <n>]
      Index video candidates and likely manufacturer/parser lanes from a mounted drive, copied folder, or disk image export.
      Full SHA-256 hashing is opt-in because terabyte-scale evidence can take hours.

  make-review <case-dir>
      Generate a serverless HTML review dashboard at review/index.html.

  make-report <case-dir>
      Generate a case report at reports/case-report.html.

  export-video <case-dir> <video-id|path> --format <mp4|avi> [--start <seconds>] [--duration <seconds>] [--output <path>]
      Export an indexed video or selected range as a client-deliverable MP4/AVI.

  make-proxy <case-dir> <video-id|path> [--max-width <pixels>] [--output <path>]
      Generate a lower-bitrate review proxy MP4.

  make-thumbnail <case-dir> <video-id|path> [--time <seconds>] [--output <path>]
      Generate a JPEG thumbnail for review/reporting.

  carve-file <case-dir> <source-file> [--max-bytes <n>] [--max-candidates <n>]
      Scan a raw file or forensic image for contiguous MP4/AVI/Dahua-DAV candidates and copy them as recovery artifacts.

  inspect <case-dir>
      Print the current case/index status.
"
    );
}
