use super::{ValidationOptions, ValidationResult};
use crate::audit;
use crate::ffprobe;
use crate::media_contract;
use crate::model::ProbeSummary;
use crate::tool_policy::require_case_output_path;
use crate::util::json_escape;
use std::path::Path;

#[cfg(test)]
mod tests;

pub(super) fn validation_status(probe: &ProbeSummary) -> (&'static str, &'static str) {
    if !probe.ok {
        return (
            "validation-failed",
            "ffprobe could not parse the file; keep as candidate until manual/vendor-player validation.",
        );
    }
    if probe.video_codec.is_none() {
        return (
            "validation-failed",
            "ffprobe parsed the container but found no video stream.",
        );
    }
    (
        "ffprobe-video-stream-confirmed",
        "ffprobe parsed a video stream; examiner playback review is still required before final reporting.",
    )
}

pub(super) fn append_validation_log(
    case_dir: &Path,
    result: &ValidationResult,
    options: &ValidationOptions,
) -> Result<(), String> {
    let operator = media_contract::resolve_operator(case_dir, options.operator.as_deref())?;
    let line = validation_log_body_json(result, options, &operator);
    let log_path = case_dir.join("evidence/logs/validation-log.jsonl");
    require_case_output_path(case_dir, &log_path, "validation log")?;
    audit::append_chained_jsonl(&log_path, &line)
}

fn validation_log_body_json(
    result: &ValidationResult,
    _options: &ValidationOptions,
    operator: &str,
) -> String {
    let command_args = ffprobe::probe_command_args(&result.target_path);
    let source_artifact_sha256 = result
        .source_artifact_sha256
        .as_deref()
        .unwrap_or(&result.target_sha256);
    let source_artifact_id = result.source_artifact_id.clone().unwrap_or_else(|| {
        media_contract::source_artifact_id(&result.selector, source_artifact_sha256)
    });
    let source_artifact_path = result
        .source_artifact_path
        .as_ref()
        .unwrap_or(&result.target_path);
    let target_artifact_id = result
        .target_artifact_id
        .as_deref()
        .or(result.derived_artifact_id.as_deref())
        .unwrap_or(&source_artifact_id);
    format!(
        "{{\"schema_version\":3,\"event\":\"validate-artifact\",\"validated_unix\":{},\"operator\":\"{}\",\"method\":\"ffprobe-container-video-stream\",\"tool\":\"ffprobe\",\"tool_version\":\"{}\",\"resolved_tool_path\":\"{}\",\"command\":\"{}\",\"command_args\":{},\"selector\":\"{}\",\"source_artifact_id\":\"{}\",\"source_artifact_path\":\"{}\",\"source_artifact_sha256\":\"{}\",\"derived_artifact_id\":{},\"target_artifact_id\":\"{}\",\"target_path\":\"{}\",\"target_sha256\":\"{}\",\"validation_artifact_path\":\"evidence/logs/validation-log.jsonl\",\"validation_status\":\"{}\",\"validation_note\":\"{}\",\"duration_seconds\":{},\"format_name\":{},\"video_codec\":{},\"audio_codec\":{},\"width\":{},\"height\":{},\"ffprobe_ok\":{},\"ffprobe_error\":{},\"ffprobe_version\":\"{}\"}}",
        result.validated_unix,
        json_escape(operator),
        json_escape(result.ffprobe_tool.version()),
        json_escape(result.ffprobe_tool.path()),
        json_escape(result.ffprobe_tool.path()),
        audit::json_string_array(&command_args),
        json_escape(&result.selector),
        json_escape(&source_artifact_id),
        json_escape(&source_artifact_path.to_string_lossy()),
        json_escape(source_artifact_sha256),
        audit::optional_string(result.derived_artifact_id.as_deref()),
        json_escape(target_artifact_id),
        json_escape(&result.target_path.to_string_lossy()),
        json_escape(&result.target_sha256),
        json_escape(&result.validation_status),
        json_escape(&result.validation_note),
        optional_f64(result.probe.duration_seconds),
        audit::optional_string(result.probe.format_name.as_deref()),
        audit::optional_string(result.probe.video_codec.as_deref()),
        audit::optional_string(result.probe.audio_codec.as_deref()),
        optional_u32(result.probe.width),
        optional_u32(result.probe.height),
        result.probe.ok,
        audit::optional_string(result.probe.error.as_deref()),
        json_escape(result.ffprobe_tool.version())
    )
}

fn optional_f64(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "null".to_string())
}

fn optional_u32(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}
