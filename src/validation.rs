mod log;
mod target;
#[cfg(test)]
mod target_tests;

use crate::audit;
use crate::ffprobe;
use crate::model::ProbeSummary;
use crate::tool_policy::{ResolvedExternalTool, resolve_external_tool};
use crate::util::now_unix;
use log::{append_validation_log, validation_status};
use std::path::{Path, PathBuf};
use target::resolve_validation_target;

#[derive(Debug, Clone)]
pub struct ValidationOptions {
    pub ffprobe_bin: String,
    pub operator: Option<String>,
    pub allow_external_source: bool,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            ffprobe_bin: "ffprobe".to_string(),
            operator: None,
            allow_external_source: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub selector: String,
    pub target_path: PathBuf,
    pub target_sha256: String,
    pub source_artifact_id: Option<String>,
    pub source_artifact_path: Option<PathBuf>,
    pub source_artifact_sha256: Option<String>,
    pub derived_artifact_id: Option<String>,
    pub target_artifact_id: Option<String>,
    pub validation_status: String,
    pub validation_note: String,
    pub probe: ProbeSummary,
    pub validated_unix: u64,
    pub ffprobe_tool: ResolvedExternalTool,
}

pub fn validate_artifact(
    case_dir: &Path,
    selector: &str,
    options: &ValidationOptions,
) -> Result<ValidationResult, String> {
    let ffprobe_tool = resolve_external_tool(&options.ffprobe_bin, &["ffprobe"], "-version")?;
    let target = resolve_validation_target(case_dir, selector, options.allow_external_source)?;
    let validated_unix = now_unix()?;
    let target_sha256 = audit::digest_file(&target.path)?;
    let probe = ffprobe::probe_with_tool(&ffprobe_tool, &target.path);
    let (validation_status, validation_note) = validation_status(&probe);
    let result = ValidationResult {
        selector: selector.to_string(),
        target_path: target.path,
        target_sha256,
        source_artifact_id: target.source_artifact_id,
        source_artifact_path: target.source_artifact_path,
        source_artifact_sha256: target.source_artifact_sha256,
        derived_artifact_id: target.derived_artifact_id,
        target_artifact_id: target.target_artifact_id,
        validation_status: validation_status.to_string(),
        validation_note: validation_note.to_string(),
        probe,
        validated_unix,
        ffprobe_tool,
    };
    append_validation_log(case_dir, &result, options)?;
    Ok(result)
}
