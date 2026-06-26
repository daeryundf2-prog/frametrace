use super::{ExportOptions, ffmpeg_export_args};
use crate::audit;
use crate::media_contract;
use crate::tool_policy::{ResolvedExternalTool, require_case_output_path};
use crate::util::{json_escape, now_unix};
use std::path::Path;

pub(super) fn write_export_log(
    case_dir: &Path,
    selector: &str,
    source_path: &Path,
    output_path: &Path,
    options: &ExportOptions,
    operator: &str,
    tool: &ResolvedExternalTool,
) -> Result<(), String> {
    let path = case_dir.join("artifacts/clips/export-log.jsonl");
    require_case_output_path(case_dir, &path, "video export log")?;
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
        tool,
    });
    audit::append_chained_jsonl(&path, &line)
}

pub(super) struct ExportLogBody<'a> {
    pub(super) selector: &'a str,
    pub(super) source_path: &'a Path,
    pub(super) output_path: &'a Path,
    pub(super) options: &'a ExportOptions,
    pub(super) source_sha256: Option<&'a str>,
    pub(super) output_sha256: &'a str,
    pub(super) exported_unix: u64,
    pub(super) operator: &'a str,
    pub(super) tool: &'a ResolvedExternalTool,
}

pub(super) fn export_log_body_json(input: ExportLogBody<'_>) -> String {
    let args = ffmpeg_export_args(input.source_path, input.output_path, input.options);
    let source_artifact_id = media_contract::source_artifact_id(
        input.selector,
        input.source_sha256.unwrap_or("unknown"),
    );
    let derived_artifact_id = media_contract::derived_artifact_id("clip", input.output_sha256);
    format!(
        "{{\"schema_version\":3,\"event\":\"export-video\",\"exported_unix\":{},\"selector\":\"{}\",\"artifact_state\":\"derived\",\"operator\":\"{}\",\"method\":\"ffmpeg-clip-export\",\"tool\":\"ffmpeg\",\"tool_version\":\"{}\",\"resolved_tool_path\":\"{}\",\"source_artifact_id\":\"{}\",\"source_artifact_path\":\"{}\",\"source_artifact_sha256\":{},\"source_path\":\"{}\",\"source_index_sha256\":{},\"derived_artifact_id\":\"{}\",\"output_artifact_path\":\"{}\",\"output_artifact_sha256\":\"{}\",\"output_path\":\"{}\",\"output_sha256\":\"{}\",\"format\":\"{}\",\"start_seconds\":{},\"duration_seconds\":{},\"ffmpeg_version\":\"{}\",\"command\":\"{}\",\"command_args\":{}}}",
        input.exported_unix,
        json_escape(input.selector),
        json_escape(input.operator),
        json_escape(input.tool.version()),
        json_escape(input.tool.path()),
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
        json_escape(input.tool.version()),
        json_escape(input.tool.path()),
        audit::json_string_array(&args)
    )
}

fn optional_f64(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "null".to_string())
}
