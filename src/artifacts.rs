mod ffmpeg;
#[cfg(test)]
mod tests;

use crate::media_contract;
use crate::tool_policy::{reject_source_output_path, require_case_output_path};
use crate::util::{now_unix, unique_path};
use crate::video_export::{resolve_video_source, sanitize_filename};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ProxyOptions {
    pub output_path: Option<PathBuf>,
    pub max_width: u32,
    pub operator: Option<String>,
    pub ffmpeg_bin: String,
}

impl Default for ProxyOptions {
    fn default() -> Self {
        Self {
            output_path: None,
            max_width: 1280,
            operator: None,
            ffmpeg_bin: "ffmpeg".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ThumbnailOptions {
    pub output_path: Option<PathBuf>,
    pub time_seconds: f64,
    pub operator: Option<String>,
    pub ffmpeg_bin: String,
}

impl Default for ThumbnailOptions {
    fn default() -> Self {
        Self {
            output_path: None,
            time_seconds: 0.0,
            operator: None,
            ffmpeg_bin: "ffmpeg".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FrameCaptureOptions {
    pub output_path: Option<PathBuf>,
    pub time_seconds: f64,
    pub operator: Option<String>,
    pub ffmpeg_bin: String,
}

impl Default for FrameCaptureOptions {
    fn default() -> Self {
        Self {
            output_path: None,
            time_seconds: 0.0,
            operator: None,
            ffmpeg_bin: "ffmpeg".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DerivedArtifact {
    pub source_path: PathBuf,
    pub output_path: PathBuf,
    pub kind: String,
    pub created_unix: u64,
    pub selector: String,
    pub operator: String,
}

pub fn generate_proxy(
    case_dir: &Path,
    selector: &str,
    options: &ProxyOptions,
) -> Result<DerivedArtifact, String> {
    if options.max_width < 160 {
        return Err("--max-width must be at least 160".to_string());
    }
    let source_path = resolve_video_source(case_dir, selector)?;
    let operator = media_contract::resolve_operator(case_dir, options.operator.as_deref())?;
    let created_unix = now_unix()?;
    let output_path = if let Some(output_path) = &options.output_path {
        require_case_output_path(case_dir, output_path, "proxy")?;
        reject_source_output_path(&source_path, output_path, "proxy")?;
        if output_path.exists() {
            return Err(format!(
                "output already exists: {} (choose a new --output path)",
                output_path.display()
            ));
        }
        output_path.clone()
    } else {
        unique_path(&case_dir.join("artifacts/proxies").join(format!(
            "{}_proxy_{}.mp4",
            sanitize_filename(selector),
            created_unix
        )))
    };
    require_case_output_path(case_dir, &output_path, "proxy")?;
    reject_source_output_path(&source_path, &output_path, "proxy")?;
    ensure_parent(&output_path)?;

    let args = ffmpeg::proxy_args(&source_path, &output_path, options);
    let tool = ffmpeg::run(&options.ffmpeg_bin, &args)?;

    let artifact = DerivedArtifact {
        source_path,
        output_path,
        kind: "proxy".to_string(),
        created_unix,
        selector: selector.to_string(),
        operator,
    };
    ffmpeg::append_log(
        case_dir,
        "artifacts/proxies/proxy-log.jsonl",
        &artifact,
        &args,
        &tool,
    )?;
    Ok(artifact)
}

pub fn generate_thumbnail(
    case_dir: &Path,
    selector: &str,
    options: &ThumbnailOptions,
) -> Result<DerivedArtifact, String> {
    if options.time_seconds.is_sign_negative() || !options.time_seconds.is_finite() {
        return Err("--time must be a non-negative finite number".to_string());
    }
    let source_path = resolve_video_source(case_dir, selector)?;
    let operator = media_contract::resolve_operator(case_dir, options.operator.as_deref())?;
    let created_unix = now_unix()?;
    let output_path = if let Some(output_path) = &options.output_path {
        require_case_output_path(case_dir, output_path, "thumbnail")?;
        reject_source_output_path(&source_path, output_path, "thumbnail")?;
        if output_path.exists() {
            return Err(format!(
                "output already exists: {} (choose a new --output path)",
                output_path.display()
            ));
        }
        output_path.clone()
    } else {
        unique_path(&case_dir.join("artifacts/thumbnails").join(format!(
            "{}_thumb_{}.jpg",
            sanitize_filename(selector),
            created_unix
        )))
    };
    require_case_output_path(case_dir, &output_path, "thumbnail")?;
    reject_source_output_path(&source_path, &output_path, "thumbnail")?;
    ensure_parent(&output_path)?;

    let args = ffmpeg::thumbnail_args(&source_path, &output_path, options);
    let tool = ffmpeg::run(&options.ffmpeg_bin, &args)?;

    let artifact = DerivedArtifact {
        source_path,
        output_path,
        kind: "thumbnail".to_string(),
        created_unix,
        selector: selector.to_string(),
        operator,
    };
    ffmpeg::append_log(
        case_dir,
        "artifacts/thumbnails/thumbnail-log.jsonl",
        &artifact,
        &args,
        &tool,
    )?;
    Ok(artifact)
}

pub fn capture_frame(
    case_dir: &Path,
    selector: &str,
    options: &FrameCaptureOptions,
) -> Result<DerivedArtifact, String> {
    if options.time_seconds.is_sign_negative() || !options.time_seconds.is_finite() {
        return Err("--time must be a non-negative finite number".to_string());
    }
    let source_path = resolve_video_source(case_dir, selector)?;
    let operator = media_contract::resolve_operator(case_dir, options.operator.as_deref())?;
    let created_unix = now_unix()?;
    let output_path = if let Some(output_path) = &options.output_path {
        require_case_output_path(case_dir, output_path, "frame capture")?;
        reject_source_output_path(&source_path, output_path, "frame capture")?;
        if output_path.exists() {
            return Err(format!(
                "output already exists: {} (choose a new --output path)",
                output_path.display()
            ));
        }
        output_path.clone()
    } else {
        unique_path(&case_dir.join("artifacts/frames").join(format!(
            "{}_frame_{}.jpg",
            sanitize_filename(selector),
            created_unix
        )))
    };
    require_case_output_path(case_dir, &output_path, "frame capture")?;
    reject_source_output_path(&source_path, &output_path, "frame capture")?;
    ensure_parent(&output_path)?;

    let args = ffmpeg::frame_capture_args(&source_path, &output_path, options);
    let tool = ffmpeg::run(&options.ffmpeg_bin, &args)?;

    let artifact = DerivedArtifact {
        source_path,
        output_path,
        kind: "frame-capture".to_string(),
        created_unix,
        selector: selector.to_string(),
        operator,
    };
    ffmpeg::append_log(
        case_dir,
        "artifacts/frames/frame-log.jsonl",
        &artifact,
        &args,
        &tool,
    )?;
    Ok(artifact)
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create output directory: {err}"))?;
    }
    Ok(())
}
