use crate::audit;
use crate::media_contract;
use crate::tool_policy::{reject_source_output_path, require_case_output_path};
use crate::util::{json_escape, now_unix, read_to_string, unique_path};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Mp4,
    Avi,
}

impl ExportFormat {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.to_ascii_lowercase().as_str() {
            "mp4" => Ok(Self::Mp4),
            "avi" => Ok(Self::Avi),
            other => Err(format!(
                "unsupported export format: {other} (use mp4 or avi)"
            )),
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Avi => "avi",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub start_seconds: Option<f64>,
    pub duration_seconds: Option<f64>,
    pub output_path: Option<PathBuf>,
    pub operator: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExportResult {
    pub source_path: PathBuf,
    pub output_path: PathBuf,
    pub format: ExportFormat,
}

pub fn export_video(
    case_dir: &Path,
    selector: &str,
    options: &ExportOptions,
) -> Result<ExportResult, String> {
    let source_path = resolve_video_source(case_dir, selector)?;
    let operator = media_contract::resolve_operator(case_dir, options.operator.as_deref())?;
    let export_unix = now_unix()?;
    let output_path = if let Some(output_path) = &options.output_path {
        require_case_output_path(case_dir, output_path, "video export")?;
        reject_source_output_path(&source_path, output_path, "video export")?;
        if output_path.exists() {
            return Err(format!(
                "output already exists: {} (choose a new --output path)",
                output_path.display()
            ));
        }
        output_path.clone()
    } else {
        unique_path(&case_dir.join("artifacts/clips").join(format!(
            "{}_{}.{}",
            sanitize_filename(selector),
            export_unix,
            options.format.extension()
        )))
    };
    reject_source_output_path(&source_path, &output_path, "video export")?;

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create output directory: {err}"))?;
    }

    run_ffmpeg_export(&source_path, &output_path, options)?;
    write_export_log(
        case_dir,
        selector,
        &source_path,
        &output_path,
        options,
        &operator,
    )?;

    Ok(ExportResult {
        source_path,
        output_path,
        format: options.format,
    })
}

fn run_ffmpeg_export(
    source_path: &Path,
    output_path: &Path,
    options: &ExportOptions,
) -> Result<(), String> {
    let args = ffmpeg_export_args(source_path, output_path, options);
    let output = Command::new("ffmpeg")
        .args(&args)
        .output()
        .map_err(|err| format!("failed to run ffmpeg: {err}"))?;

    if !output.status.success() {
        return Err(format!(
            "ffmpeg export failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

fn ffmpeg_export_args(
    source_path: &Path,
    output_path: &Path,
    options: &ExportOptions,
) -> Vec<String> {
    let mut args = vec![
        "-n".to_string(),
        "-hide_banner".to_string(),
        "-i".to_string(),
        audit::path_string(source_path),
    ];
    if let Some(start) = options.start_seconds {
        args.push("-ss".to_string());
        args.push(format!("{start:.3}"));
    }
    if let Some(duration) = options.duration_seconds {
        args.push("-t".to_string());
        args.push(format!("{duration:.3}"));
    }
    args.extend([
        "-map".to_string(),
        "0:v:0".to_string(),
        "-map".to_string(),
        "0:a?".to_string(),
    ]);

    match options.format {
        ExportFormat::Mp4 => {
            args.extend(
                [
                    "-c:v",
                    "libx264",
                    "-preset",
                    "veryfast",
                    "-crf",
                    "20",
                    "-pix_fmt",
                    "yuv420p",
                    "-c:a",
                    "aac",
                    "-movflags",
                    "+faststart",
                ]
                .iter()
                .map(|arg| arg.to_string()),
            );
        }
        ExportFormat::Avi => {
            args.extend(
                ["-c:v", "mpeg4", "-q:v", "3", "-c:a", "libmp3lame"]
                    .iter()
                    .map(|arg| arg.to_string()),
            );
        }
    }
    args.push(audit::path_string(output_path));
    args
}

pub fn resolve_video_source(case_dir: &Path, selector: &str) -> Result<PathBuf, String> {
    let direct = PathBuf::from(selector);
    if direct.is_file() {
        return direct
            .canonicalize()
            .map_err(|err| format!("failed to canonicalize source path: {err}"));
    }

    let index_path = case_dir.join("db/video_paths.tsv");
    let text = read_to_string(&index_path).map_err(|err| {
        format!(
            "failed to read {}: {err} (run scan-folder first)",
            index_path.display()
        )
    })?;

    for line in text.lines().skip(1) {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 3 {
            continue;
        }
        let id = fields[0];
        let source_path = tsv_unescape(fields[1]);
        let relative_path = tsv_unescape(fields[2]);
        if selector == id || selector == source_path || selector == relative_path {
            let path = PathBuf::from(&source_path);
            if path.is_file() {
                return path
                    .canonicalize()
                    .map_err(|err| format!("failed to canonicalize source path: {err}"));
            }
            return Err(format!(
                "indexed source file no longer exists: {source_path}"
            ));
        }
    }

    Err(format!(
        "video selector not found: {selector} (use an indexed id like vid_000001 or a source path)"
    ))
}

fn write_export_log(
    case_dir: &Path,
    selector: &str,
    source_path: &Path,
    output_path: &Path,
    options: &ExportOptions,
    operator: &str,
) -> Result<(), String> {
    let path = case_dir.join("artifacts/clips/export-log.jsonl");
    let exported_unix = now_unix()?;
    let source_sha256 = audit::indexed_source_hash(case_dir, selector, source_path);
    let output_sha256 = audit::digest_file(output_path)?;
    let line = export_log_body_json(ExportLogBody {
        selector,
        source_path,
        output_path,
        options,
        source_sha256: source_sha256.as_deref(),
        output_sha256: &output_sha256,
        exported_unix,
        operator,
    });
    audit::append_chained_jsonl(&path, &line)
}

struct ExportLogBody<'a> {
    selector: &'a str,
    source_path: &'a Path,
    output_path: &'a Path,
    options: &'a ExportOptions,
    source_sha256: Option<&'a str>,
    output_sha256: &'a str,
    exported_unix: u64,
    operator: &'a str,
}

fn export_log_body_json(input: ExportLogBody<'_>) -> String {
    let args = ffmpeg_export_args(input.source_path, input.output_path, input.options);
    let ffmpeg_version = audit::command_version("ffmpeg");
    let source_artifact_id = media_contract::source_artifact_id(
        input.selector,
        input.source_sha256.unwrap_or("unknown"),
    );
    let derived_artifact_id = media_contract::derived_artifact_id("clip", input.output_sha256);
    format!(
        "{{\"schema_version\":3,\"event\":\"export-video\",\"exported_unix\":{},\"selector\":\"{}\",\"artifact_state\":\"derived\",\"operator\":\"{}\",\"method\":\"ffmpeg-clip-export\",\"tool\":\"ffmpeg\",\"tool_version\":\"{}\",\"source_artifact_id\":\"{}\",\"source_artifact_path\":\"{}\",\"source_artifact_sha256\":{},\"source_path\":\"{}\",\"source_index_sha256\":{},\"derived_artifact_id\":\"{}\",\"output_artifact_path\":\"{}\",\"output_artifact_sha256\":\"{}\",\"output_path\":\"{}\",\"output_sha256\":\"{}\",\"format\":\"{}\",\"start_seconds\":{},\"duration_seconds\":{},\"ffmpeg_version\":\"{}\",\"command\":\"ffmpeg\",\"command_args\":{}}}",
        input.exported_unix,
        json_escape(input.selector),
        json_escape(input.operator),
        json_escape(&ffmpeg_version),
        json_escape(&source_artifact_id),
        json_escape(&input.source_path.to_string_lossy()),
        audit::optional_string(input.source_sha256),
        json_escape(&input.source_path.to_string_lossy()),
        audit::optional_string(input.source_sha256),
        json_escape(&derived_artifact_id),
        json_escape(&input.output_path.to_string_lossy()),
        json_escape(input.output_sha256),
        json_escape(&input.output_path.to_string_lossy()),
        json_escape(input.output_sha256),
        input.options.format.extension(),
        optional_f64(input.options.start_seconds),
        optional_f64(input.options.duration_seconds),
        json_escape(&ffmpeg_version),
        audit::json_string_array(&args)
    )
}

fn optional_f64(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "null".to_string())
}

fn tsv_unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('n') => out.push('\n'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

pub fn sanitize_filename(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "clip".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ExportFormat, ExportLogBody, ExportOptions, export_log_body_json, ffmpeg_export_args,
        sanitize_filename, tsv_unescape,
    };
    use std::path::Path;

    #[test]
    fn parses_export_formats() {
        assert_eq!(ExportFormat::parse("mp4").unwrap(), ExportFormat::Mp4);
        assert_eq!(ExportFormat::parse("AVI").unwrap(), ExportFormat::Avi);
        assert!(ExportFormat::parse("mkv").is_err());
    }

    #[test]
    fn sanitizes_clip_names() {
        assert_eq!(sanitize_filename("vid_000001"), "vid_000001");
        assert_eq!(sanitize_filename("a/b:c.mp4"), "a_b_c_mp4");
    }

    #[test]
    fn builds_export_command_args() {
        let options = ExportOptions {
            format: ExportFormat::Mp4,
            start_seconds: Some(1.0),
            duration_seconds: Some(2.0),
            output_path: None,
            operator: Some("qa-operator".to_string()),
        };
        let args = ffmpeg_export_args(Path::new("in.mp4"), Path::new("out.mp4"), &options);
        assert!(args.contains(&"-n".to_string()));
        assert!(args.contains(&"libx264".to_string()));
        assert_eq!(args.last().map(String::as_str), Some("out.mp4"));
    }

    #[test]
    fn export_log_body_records_derived_artifact_contract() {
        let options = ExportOptions {
            format: ExportFormat::Mp4,
            start_seconds: Some(1.0),
            duration_seconds: Some(2.0),
            output_path: None,
            operator: Some("qa-operator".to_string()),
        };
        let source_sha256 =
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let output_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

        let body = export_log_body_json(ExportLogBody {
            selector: "vid_000001",
            source_path: Path::new("/case/source/clip.mp4"),
            output_path: Path::new("/case/artifacts/clips/clip.mp4"),
            options: &options,
            source_sha256,
            output_sha256,
            exported_unix: 1_789_000_000,
            operator: "qa-operator",
        });

        assert!(body.contains(r#""event":"export-video""#));
        assert!(body.contains(r#""artifact_state":"derived""#));
        assert!(body.contains(r#""operator":"qa-operator""#));
        assert!(body.contains(r#""method":"ffmpeg-clip-export""#));
        assert!(body.contains(r#""tool":"ffmpeg""#));
        assert!(body.contains(r#""derived_artifact_id":"derived-clip-bbbbbbbbbbbb""#));
        assert!(body.contains(r#""source_artifact_id":"source-vid_000001-aaaaaaaaaaaa""#));
        assert!(body.contains(r#""output_artifact_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb""#));
    }

    #[test]
    fn unescapes_tsv_paths() {
        assert_eq!(tsv_unescape("a\\tb\\\\c"), "a\tb\\c");
    }
}
