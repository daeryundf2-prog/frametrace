use crate::util::{now_unix, read_to_string, unique_path, write_text};
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
    let export_unix = now_unix()?;
    let output_path = if let Some(output_path) = &options.output_path {
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

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create output directory: {err}"))?;
    }

    run_ffmpeg_export(&source_path, &output_path, options)?;
    write_export_log(case_dir, selector, &source_path, &output_path, options)?;

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
    let mut command = Command::new("ffmpeg");
    command
        .arg("-n")
        .arg("-hide_banner")
        .arg("-i")
        .arg(source_path);
    if let Some(start) = options.start_seconds {
        command.arg("-ss").arg(format!("{start:.3}"));
    }
    if let Some(duration) = options.duration_seconds {
        command.arg("-t").arg(format!("{duration:.3}"));
    }
    command.arg("-map").arg("0:v:0").arg("-map").arg("0:a?");

    match options.format {
        ExportFormat::Mp4 => {
            command
                .arg("-c:v")
                .arg("libx264")
                .arg("-preset")
                .arg("veryfast")
                .arg("-crf")
                .arg("20")
                .arg("-pix_fmt")
                .arg("yuv420p")
                .arg("-c:a")
                .arg("aac")
                .arg("-movflags")
                .arg("+faststart");
        }
        ExportFormat::Avi => {
            command
                .arg("-c:v")
                .arg("mpeg4")
                .arg("-q:v")
                .arg("3")
                .arg("-c:a")
                .arg("libmp3lame");
        }
    }

    let output = command
        .arg(output_path)
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
        let source_path = fields[1];
        let relative_path = fields[2];
        if selector == id || selector == source_path || selector == relative_path {
            let path = PathBuf::from(source_path);
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
) -> Result<(), String> {
    let path = case_dir.join("artifacts/clips/export-log.jsonl");
    let existing = read_to_string(&path).unwrap_or_default();
    let line = format!(
        "{{\"exported_unix\":{},\"selector\":\"{}\",\"source_path\":\"{}\",\"output_path\":\"{}\",\"format\":\"{}\",\"start_seconds\":{},\"duration_seconds\":{}}}\n",
        now_unix()?,
        crate::util::json_escape(selector),
        crate::util::json_escape(&source_path.to_string_lossy()),
        crate::util::json_escape(&output_path.to_string_lossy()),
        options.format.extension(),
        optional_f64(options.start_seconds),
        optional_f64(options.duration_seconds)
    );
    write_text(&path, &(existing + &line))
        .map_err(|err| format!("failed to write export log: {err}"))
}

fn optional_f64(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "null".to_string())
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
    use super::{ExportFormat, sanitize_filename};

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
}
