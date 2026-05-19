use crate::util::{json_escape, now_unix, read_to_string, unique_path, write_text};
use crate::video_export::{resolve_video_source, sanitize_filename};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ProxyOptions {
    pub output_path: Option<PathBuf>,
    pub max_width: u32,
}

impl Default for ProxyOptions {
    fn default() -> Self {
        Self {
            output_path: None,
            max_width: 1280,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ThumbnailOptions {
    pub output_path: Option<PathBuf>,
    pub time_seconds: f64,
}

impl Default for ThumbnailOptions {
    fn default() -> Self {
        Self {
            output_path: None,
            time_seconds: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DerivedArtifact {
    pub source_path: PathBuf,
    pub output_path: PathBuf,
    pub kind: String,
    pub created_unix: u64,
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
    let created_unix = now_unix()?;
    let output_path = if let Some(output_path) = &options.output_path {
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
    ensure_parent(&output_path)?;

    let scale = format!("scale='min({},{})':-2", options.max_width, "iw");
    let output = Command::new("ffmpeg")
        .arg("-n")
        .arg("-hide_banner")
        .arg("-i")
        .arg(&source_path)
        .arg("-vf")
        .arg(scale)
        .arg("-map")
        .arg("0:v:0")
        .arg("-map")
        .arg("0:a?")
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("veryfast")
        .arg("-crf")
        .arg("28")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-c:a")
        .arg("aac")
        .arg("-movflags")
        .arg("+faststart")
        .arg(&output_path)
        .output()
        .map_err(|err| format!("failed to run ffmpeg for proxy: {err}"))?;

    if !output.status.success() {
        return Err(format!(
            "proxy generation failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let artifact = DerivedArtifact {
        source_path,
        output_path,
        kind: "proxy".to_string(),
        created_unix,
    };
    append_artifact_log(case_dir, "artifacts/proxies/proxy-log.jsonl", &artifact)?;
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
    let created_unix = now_unix()?;
    let output_path = if let Some(output_path) = &options.output_path {
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
    ensure_parent(&output_path)?;

    let output = Command::new("ffmpeg")
        .arg("-n")
        .arg("-hide_banner")
        .arg("-ss")
        .arg(format!("{:.3}", options.time_seconds))
        .arg("-i")
        .arg(&source_path)
        .arg("-frames:v")
        .arg("1")
        .arg("-q:v")
        .arg("3")
        .arg(&output_path)
        .output()
        .map_err(|err| format!("failed to run ffmpeg for thumbnail: {err}"))?;

    if !output.status.success() {
        return Err(format!(
            "thumbnail generation failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let artifact = DerivedArtifact {
        source_path,
        output_path,
        kind: "thumbnail".to_string(),
        created_unix,
    };
    append_artifact_log(
        case_dir,
        "artifacts/thumbnails/thumbnail-log.jsonl",
        &artifact,
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

fn append_artifact_log(
    case_dir: &Path,
    relative_log: &str,
    artifact: &DerivedArtifact,
) -> Result<(), String> {
    let path = case_dir.join(relative_log);
    let existing = read_to_string(&path).unwrap_or_default();
    let line = format!(
        "{{\"created_unix\":{},\"kind\":\"{}\",\"source_path\":\"{}\",\"output_path\":\"{}\"}}\n",
        artifact.created_unix,
        json_escape(&artifact.kind),
        json_escape(&artifact.source_path.to_string_lossy()),
        json_escape(&artifact.output_path.to_string_lossy())
    );
    write_text(&path, &(existing + &line))
        .map_err(|err| format!("failed to write artifact log: {err}"))
}

#[cfg(test)]
mod tests {
    use super::{ProxyOptions, ThumbnailOptions};

    #[test]
    fn default_proxy_is_review_sized() {
        assert_eq!(ProxyOptions::default().max_width, 1280);
    }

    #[test]
    fn default_thumbnail_starts_at_zero() {
        assert_eq!(ThumbnailOptions::default().time_seconds, 0.0);
    }
}
