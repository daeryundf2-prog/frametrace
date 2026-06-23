use crate::audit;
use crate::tool_policy::{reject_source_output_path, require_case_output_path};
use std::path::{Path, PathBuf};

pub(super) fn require_e01_output_path(
    case_dir: &Path,
    e01_path: &Path,
    output_path: &Path,
    label: &str,
) -> Result<(), String> {
    require_case_output_path(case_dir, output_path, label)?;
    reject_source_output_path(e01_path, output_path, label)
}

pub(super) fn e01_audit_log_path(case_dir: &Path, e01_path: &Path) -> Result<PathBuf, String> {
    let path = case_dir.join("evidence/logs/e01-audit.jsonl");
    require_e01_output_path(case_dir, e01_path, &path, "E01 audit log")?;
    Ok(path)
}

pub(super) fn append_e01_audit_at(path: &Path, body_json: &str) -> Result<(), String> {
    audit::append_chained_jsonl(path, body_json)
}
