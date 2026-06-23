use crate::audit;
use crate::media_contract;
use crate::tool_policy::{reject_source_output_path, require_case_output_path};
use crate::util::{json_escape, now_unix, unique_path};
use crate::video_export::{resolve_video_source, sanitize_filename};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ProxyOptions {
    pub output_path: Option<PathBuf>,
    pub max_width: u32,
    pub operator: Option<String>,
}

impl Default for ProxyOptions {
    fn default() -> Self {
        Self {
            output_path: None,
            max_width: 1280,
            operator: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ThumbnailOptions {
    pub output_path: Option<PathBuf>,
    pub time_seconds: f64,
    pub operator: Option<String>,
}

impl Default for ThumbnailOptions {
    fn default() -> Self {
        Self {
            output_path: None,
            time_seconds: 0.0,
            operator: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FrameCaptureOptions {
    pub output_path: Option<PathBuf>,
    pub time_seconds: f64,
    pub operator: Option<String>,
}

impl Default for FrameCaptureOptions {
    fn default() -> Self {
        Self {
            output_path: None,
            time_seconds: 0.0,
            operator: None,
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

    let args = proxy_ffmpeg_args(&source_path, &output_path, options);
    let output = Command::new("ffmpeg")
        .args(&args)
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
        selector: selector.to_string(),
        operator,
    };
    append_artifact_log(
        case_dir,
        "artifacts/proxies/proxy-log.jsonl",
        &artifact,
        &args,
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

    let args = thumbnail_ffmpeg_args(&source_path, &output_path, options);
    let output = Command::new("ffmpeg")
        .args(&args)
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
        selector: selector.to_string(),
        operator,
    };
    append_artifact_log(
        case_dir,
        "artifacts/thumbnails/thumbnail-log.jsonl",
        &artifact,
        &args,
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

    let args = frame_capture_ffmpeg_args(&source_path, &output_path, options);
    let output = Command::new("ffmpeg")
        .args(&args)
        .output()
        .map_err(|err| format!("failed to run ffmpeg for frame capture: {err}"))?;

    if !output.status.success() {
        return Err(format!(
            "frame capture failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let artifact = DerivedArtifact {
        source_path,
        output_path,
        kind: "frame-capture".to_string(),
        created_unix,
        selector: selector.to_string(),
        operator,
    };
    append_artifact_log(
        case_dir,
        "artifacts/frames/frame-log.jsonl",
        &artifact,
        &args,
    )?;
    Ok(artifact)
}

fn proxy_ffmpeg_args(
    source_path: &Path,
    output_path: &Path,
    options: &ProxyOptions,
) -> Vec<String> {
    let scale = format!("scale='min({},{})':-2", options.max_width, "iw");
    vec![
        "-n".to_string(),
        "-hide_banner".to_string(),
        "-i".to_string(),
        audit::path_string(source_path),
        "-vf".to_string(),
        scale,
        "-map".to_string(),
        "0:v:0".to_string(),
        "-map".to_string(),
        "0:a?".to_string(),
        "-c:v".to_string(),
        "libx264".to_string(),
        "-preset".to_string(),
        "veryfast".to_string(),
        "-crf".to_string(),
        "28".to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        "-c:a".to_string(),
        "aac".to_string(),
        "-movflags".to_string(),
        "+faststart".to_string(),
        audit::path_string(output_path),
    ]
}

fn thumbnail_ffmpeg_args(
    source_path: &Path,
    output_path: &Path,
    options: &ThumbnailOptions,
) -> Vec<String> {
    vec![
        "-n".to_string(),
        "-hide_banner".to_string(),
        "-ss".to_string(),
        format!("{:.3}", options.time_seconds),
        "-i".to_string(),
        audit::path_string(source_path),
        "-frames:v".to_string(),
        "1".to_string(),
        "-q:v".to_string(),
        "3".to_string(),
        audit::path_string(output_path),
    ]
}

fn frame_capture_ffmpeg_args(
    source_path: &Path,
    output_path: &Path,
    options: &FrameCaptureOptions,
) -> Vec<String> {
    vec![
        "-n".to_string(),
        "-hide_banner".to_string(),
        "-ss".to_string(),
        format!("{:.3}", options.time_seconds),
        "-i".to_string(),
        audit::path_string(source_path),
        "-frames:v".to_string(),
        "1".to_string(),
        "-q:v".to_string(),
        "2".to_string(),
        audit::path_string(output_path),
    ]
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
    command_args: &[String],
) -> Result<(), String> {
    let path = case_dir.join(relative_log);
    require_case_output_path(case_dir, &path, "derived artifact log")?;
    let source_sha256 =
        audit::indexed_source_hash(case_dir, &artifact.selector, &artifact.source_path);
    let output_sha256 = audit::digest_file(&artifact.output_path)?;
    let line = derived_artifact_log_body_json(
        artifact,
        source_sha256.as_deref(),
        &output_sha256,
        command_args,
    );
    audit::append_chained_jsonl(&path, &line)
}

fn derived_artifact_log_body_json(
    artifact: &DerivedArtifact,
    source_sha256: Option<&str>,
    output_sha256: &str,
    command_args: &[String],
) -> String {
    let source_artifact_id =
        media_contract::source_artifact_id(&artifact.selector, source_sha256.unwrap_or("unknown"));
    let derived_artifact_id = media_contract::derived_artifact_id(&artifact.kind, output_sha256);
    let ffmpeg_version = audit::command_version("ffmpeg");
    format!(
        "{{\"schema_version\":3,\"event\":\"make-{}\",\"created_unix\":{},\"kind\":\"{}\",\"selector\":\"{}\",\"artifact_state\":\"derived\",\"operator\":\"{}\",\"method\":\"ffmpeg-{}\",\"tool\":\"ffmpeg\",\"tool_version\":\"{}\",\"source_artifact_id\":\"{}\",\"source_artifact_path\":\"{}\",\"source_artifact_sha256\":{},\"source_path\":\"{}\",\"source_index_sha256\":{},\"derived_artifact_id\":\"{}\",\"output_artifact_path\":\"{}\",\"output_artifact_sha256\":\"{}\",\"output_path\":\"{}\",\"output_sha256\":\"{}\",\"ffmpeg_version\":\"{}\",\"command\":\"ffmpeg\",\"command_args\":{}}}",
        json_escape(&artifact.kind),
        artifact.created_unix,
        json_escape(&artifact.kind),
        json_escape(&artifact.selector),
        json_escape(&artifact.operator),
        json_escape(&artifact.kind),
        json_escape(&ffmpeg_version),
        json_escape(&source_artifact_id),
        json_escape(&artifact.source_path.to_string_lossy()),
        audit::optional_string(source_sha256),
        json_escape(&artifact.source_path.to_string_lossy()),
        audit::optional_string(source_sha256),
        json_escape(&derived_artifact_id),
        json_escape(&artifact.output_path.to_string_lossy()),
        json_escape(output_sha256),
        json_escape(&artifact.output_path.to_string_lossy()),
        json_escape(output_sha256),
        json_escape(&ffmpeg_version),
        audit::json_string_array(command_args)
    )
}

#[cfg(test)]
mod tests {
    use super::{
        DerivedArtifact, ProxyOptions, ThumbnailOptions, derived_artifact_log_body_json,
        frame_capture_ffmpeg_args, proxy_ffmpeg_args, thumbnail_ffmpeg_args,
    };
    use std::path::{Path, PathBuf};

    #[test]
    fn default_proxy_is_review_sized() {
        assert_eq!(ProxyOptions::default().max_width, 1280);
    }

    #[test]
    fn default_thumbnail_starts_at_zero() {
        assert_eq!(ThumbnailOptions::default().time_seconds, 0.0);
    }

    #[test]
    fn frame_capture_args_record_exact_time() {
        let args = frame_capture_ffmpeg_args(
            Path::new("in.mp4"),
            Path::new("frame.jpg"),
            &super::FrameCaptureOptions {
                time_seconds: 3.5,
                ..super::FrameCaptureOptions::default()
            },
        );
        assert!(args.contains(&"-ss".to_string()));
        assert!(args.contains(&"3.500".to_string()));
        assert!(args.contains(&"-frames:v".to_string()));
        assert_eq!(args.last().map(String::as_str), Some("frame.jpg"));
    }

    #[test]
    fn derived_artifact_log_body_records_provenance_operator_and_ids() {
        let artifact = DerivedArtifact {
            source_path: PathBuf::from("/case/source/clip.mp4"),
            output_path: PathBuf::from("/case/artifacts/proxies/clip_proxy.mp4"),
            kind: "proxy".to_string(),
            created_unix: 1_789_000_000,
            selector: "vid_000001".to_string(),
            operator: "qa-operator".to_string(),
        };
        let output_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let source_sha256 =
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

        let body = derived_artifact_log_body_json(
            &artifact,
            source_sha256,
            output_sha256,
            &["-i".to_string(), "clip.mp4".to_string()],
        );

        assert!(body.contains(r#""event":"make-proxy""#));
        assert!(body.contains(r#""artifact_state":"derived""#));
        assert!(body.contains(r#""operator":"qa-operator""#));
        assert!(body.contains(r#""method":"ffmpeg-proxy""#));
        assert!(body.contains(r#""tool":"ffmpeg""#));
        assert!(body.contains(r#""selector":"vid_000001""#));
        assert!(body.contains(r#""derived_artifact_id":"derived-proxy-bbbbbbbbbbbb""#));
        assert!(body.contains(r#""source_artifact_id":"source-vid_000001-aaaaaaaaaaaa""#));
        assert!(body.contains(r#""source_artifact_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa""#));
        assert!(body.contains(r#""output_artifact_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb""#));
    }

    #[test]
    fn builds_artifact_command_args() {
        let proxy_args = proxy_ffmpeg_args(
            Path::new("in.mp4"),
            Path::new("proxy.mp4"),
            &ProxyOptions::default(),
        );
        assert!(proxy_args.contains(&"libx264".to_string()));
        assert_eq!(proxy_args.last().map(String::as_str), Some("proxy.mp4"));

        let thumb_args = thumbnail_ffmpeg_args(
            Path::new("in.mp4"),
            Path::new("thumb.jpg"),
            &ThumbnailOptions::default(),
        );
        assert!(thumb_args.contains(&"-frames:v".to_string()));
        assert_eq!(thumb_args.last().map(String::as_str), Some("thumb.jpg"));
    }
}
