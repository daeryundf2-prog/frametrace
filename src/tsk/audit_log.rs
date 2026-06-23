use super::types::{FlsEntry, MmlsPartition};
use crate::audit;
use crate::tool_policy::{reject_source_output_path, require_case_output_path};
use crate::util::json_escape;
use std::path::{Path, PathBuf};

pub(super) struct InspectSummaryInput<'a> {
    pub(super) image_path: &'a Path,
    pub(super) inspected_unix: u64,
    pub(super) partition_offset: u64,
    pub(super) partitions: &'a [MmlsPartition],
    pub(super) entries: &'a [FlsEntry],
    pub(super) warnings: &'a [String],
    pub(super) mmls_log_path: &'a Path,
    pub(super) fls_log_path: &'a Path,
    pub(super) entries_jsonl_path: &'a Path,
}

pub(super) fn inspect_summary_json(input: &InspectSummaryInput<'_>) -> String {
    let partitions_json = input
        .partitions
        .iter()
        .map(MmlsPartition::to_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\n  \"schema_version\": 1,\n  \"image_path\": \"{}\",\n  \"inspected_unix\": {},\n  \"partition_offset\": {},\n  \"partition_count\": {},\n  \"entry_count\": {},\n  \"deleted_count\": {},\n  \"video_candidate_count\": {},\n  \"warnings\": {},\n  \"mmls_log_path\": \"{}\",\n  \"fls_log_path\": \"{}\",\n  \"entries_jsonl_path\": \"{}\",\n  \"partitions\": [{}]\n}}\n",
        json_escape(&input.image_path.to_string_lossy()),
        input.inspected_unix,
        input.partition_offset,
        input.partitions.len(),
        input.entries.len(),
        input.entries.iter().filter(|entry| entry.deleted).count(),
        input
            .entries
            .iter()
            .filter(|entry| entry.video_candidate)
            .count(),
        audit::json_string_array(input.warnings),
        json_escape(&input.mmls_log_path.to_string_lossy()),
        json_escape(&input.fls_log_path.to_string_lossy()),
        json_escape(&input.entries_jsonl_path.to_string_lossy()),
        partitions_json
    )
}

pub(super) fn tsk_audit_log_path(
    case_dir: &Path,
    source_path: Option<&Path>,
) -> Result<PathBuf, String> {
    let path = case_dir.join("evidence/logs/tsk-audit.jsonl");
    require_case_output_path(case_dir, &path, "filesystem audit log")?;
    if let Some(source_path) = source_path {
        reject_source_output_path(source_path, &path, "filesystem audit log")?;
    }
    Ok(path)
}

pub(super) fn append_tsk_audit_at(path: &Path, body_json: &str) -> Result<(), String> {
    audit::append_chained_jsonl(path, body_json)
}
