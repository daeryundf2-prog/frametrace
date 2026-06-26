use super::{DerivedArtifact, FrameCaptureOptions, ProxyOptions, ThumbnailOptions};
use crate::audit;
use crate::media_contract;
use crate::tool_policy::{
    ResolvedExternalTool, require_case_output_path, resolve_external_tool, run_external_tool,
};
use crate::util::json_escape;
use std::path::Path;

pub(super) fn run(binary: &str, args: &[String]) -> Result<ResolvedExternalTool, String> {
    let tool = resolve_external_tool(binary, &["ffmpeg"], "-version")?;
    let output = run_external_tool(&tool, args)?;
    if !output.status.success() {
        return Err(format!(
            "ffmpeg derived artifact failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(tool)
}

pub(super) fn proxy_args(
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

pub(super) fn thumbnail_args(
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

pub(super) fn frame_capture_args(
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

pub(super) fn append_log(
    case_dir: &Path,
    relative_log: &str,
    artifact: &DerivedArtifact,
    command_args: &[String],
    tool: &ResolvedExternalTool,
) -> Result<(), String> {
    let path = case_dir.join(relative_log);
    require_case_output_path(case_dir, &path, "derived artifact log")?;
    let source_sha256 =
        audit::indexed_source_hash(case_dir, &artifact.selector, &artifact.source_path);
    let output_sha256 = audit::digest_file(&artifact.output_path)?;
    let line = log_body_json(
        artifact,
        source_sha256.as_deref(),
        &output_sha256,
        command_args,
        tool,
    );
    audit::append_chained_jsonl(&path, &line)
}

pub(super) fn log_body_json(
    artifact: &DerivedArtifact,
    source_sha256: Option<&str>,
    output_sha256: &str,
    command_args: &[String],
    tool: &ResolvedExternalTool,
) -> String {
    let source_artifact_id =
        media_contract::source_artifact_id(&artifact.selector, source_sha256.unwrap_or("unknown"));
    let derived_artifact_id = media_contract::derived_artifact_id(&artifact.kind, output_sha256);
    format!(
        "{{\"schema_version\":3,\"event\":\"make-{}\",\"created_unix\":{},\"kind\":\"{}\",\"selector\":\"{}\",\"artifact_state\":\"derived\",\"operator\":\"{}\",\"method\":\"ffmpeg-{}\",\"tool\":\"ffmpeg\",\"tool_version\":\"{}\",\"resolved_tool_path\":\"{}\",\"source_artifact_id\":\"{}\",\"source_artifact_path\":\"{}\",\"source_artifact_sha256\":{},\"source_path\":\"{}\",\"source_index_sha256\":{},\"derived_artifact_id\":\"{}\",\"output_artifact_path\":\"{}\",\"output_artifact_sha256\":\"{}\",\"output_path\":\"{}\",\"output_sha256\":\"{}\",\"ffmpeg_version\":\"{}\",\"command\":\"{}\",\"command_args\":{}}}",
        json_escape(&artifact.kind),
        artifact.created_unix,
        json_escape(&artifact.kind),
        json_escape(&artifact.selector),
        json_escape(&artifact.operator),
        json_escape(&artifact.kind),
        json_escape(tool.version()),
        json_escape(tool.path()),
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
        json_escape(tool.version()),
        json_escape(tool.path()),
        audit::json_string_array(command_args)
    )
}
