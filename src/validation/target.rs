use crate::audit;
use crate::media_contract;
use crate::util::read_to_string;
use crate::video_export::resolve_video_source;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(super) struct ValidationTarget {
    pub(super) path: PathBuf,
    pub(super) source_artifact_id: Option<String>,
    pub(super) source_artifact_path: Option<PathBuf>,
    pub(super) source_artifact_sha256: Option<String>,
    pub(super) derived_artifact_id: Option<String>,
    pub(super) target_artifact_id: Option<String>,
}

pub(super) fn resolve_validation_target(
    case_dir: &Path,
    selector: &str,
    allow_external_source: bool,
) -> Result<ValidationTarget, String> {
    let direct = PathBuf::from(selector);
    let case_relative_direct = if direct.is_absolute() {
        direct.clone()
    } else {
        case_dir.join(&direct)
    };
    let direct_candidate = if case_relative_direct.is_file() {
        Some(case_relative_direct)
    } else if direct.is_file() {
        Some(direct)
    } else {
        None
    };
    if let Some(direct_candidate) = direct_candidate {
        let path = direct_candidate
            .canonicalize()
            .map_err(|err| format!("failed to canonicalize validation target: {err}"))?;
        if !allow_external_source && !is_case_contained(case_dir, &path)? {
            return Err(format!(
                "direct validation target is outside the case directory; rerun with explicit external-source validation mode: {}",
                path.display()
            ));
        }
        return Ok(ValidationTarget {
            path,
            source_artifact_id: None,
            source_artifact_path: None,
            source_artifact_sha256: None,
            derived_artifact_id: None,
            target_artifact_id: None,
        });
    }

    if let Ok(path) = resolve_video_source(case_dir, selector) {
        let path = path
            .canonicalize()
            .map_err(|err| format!("failed to canonicalize validation target: {err}"))?;
        let source_artifact_sha256 = audit::indexed_source_hash(case_dir, selector, &path);
        let source_artifact_id = source_artifact_sha256
            .as_deref()
            .map(|sha256| media_contract::source_artifact_id(selector, sha256));
        return Ok(ValidationTarget {
            source_artifact_path: Some(path.clone()),
            path,
            source_artifact_id: source_artifact_id.clone(),
            source_artifact_sha256,
            derived_artifact_id: None,
            target_artifact_id: source_artifact_id,
        });
    }

    let mut provenance_errors = Vec::new();
    for rel_log in [
        "artifacts/carved/carve-log.jsonl",
        "artifacts/clips/export-log.jsonl",
        "artifacts/proxies/proxy-log.jsonl",
        "artifacts/thumbnails/thumbnail-log.jsonl",
        "artifacts/frames/frame-log.jsonl",
        "evidence/logs/tsk-audit.jsonl",
    ] {
        match resolve_from_log(&case_dir.join(rel_log), selector) {
            Ok(Some(mut target)) => {
                target.path = canonical_case_target(case_dir, &target.path)?;
                return Ok(target);
            }
            Ok(None) => {}
            Err(err) => provenance_errors.push(err),
        }
    }

    if !provenance_errors.is_empty() {
        return Err(format!(
            "validation target provenance rejected for {selector}: {}",
            provenance_errors.join("; ")
        ));
    }

    Err(format!(
        "validation target not found: {selector} (use an indexed video id, audited artifact id, inode recovery path, or explicit external-source direct file path)"
    ))
}

pub(super) fn resolve_from_log(
    log_path: &Path,
    selector: &str,
) -> Result<Option<ValidationTarget>, String> {
    if !log_path.is_file() {
        return Ok(None);
    }
    audit::verify_chained_jsonl(log_path)
        .map_err(|err| format!("audit chain rejected {}: {err}", log_path.display()))?;
    let text = read_to_string(log_path).map_err(|err| {
        format!(
            "failed to read validation provenance log {}: {err}",
            log_path.display()
        )
    })?;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let record: ValidationTargetRecord = serde_json::from_str(line).map_err(|err| {
            format!(
                "typed provenance parse failed in {} for selector {selector}: {err}",
                log_path.display()
            )
        })?;
        if record.matches(selector) {
            return record.into_target(log_path).map(Some);
        }
    }
    Ok(None)
}

#[derive(Debug, Deserialize)]
struct ValidationTargetRecord {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    inode: Option<String>,
    #[serde(default)]
    output_path: Option<String>,
    #[serde(default)]
    output_artifact_path: Option<String>,
    #[serde(default)]
    selector: Option<String>,
    #[serde(default)]
    derived_artifact_id: Option<String>,
    #[serde(default)]
    source_artifact_id: Option<String>,
    #[serde(default)]
    source_artifact_path: Option<String>,
    #[serde(default)]
    source_path: Option<String>,
    #[serde(default)]
    source_artifact_sha256: Option<String>,
    #[serde(default)]
    source_index_sha256: Option<String>,
}

impl ValidationTargetRecord {
    fn matches(&self, selector: &str) -> bool {
        self.id.as_deref() == Some(selector)
            || self.inode.as_deref() == Some(selector)
            || self.selector.as_deref() == Some(selector)
            || self.derived_artifact_id.as_deref() == Some(selector)
            || self.source_artifact_id.as_deref() == Some(selector)
            || self.output_path.as_deref() == Some(selector)
            || self.output_artifact_path.as_deref() == Some(selector)
    }

    fn into_target(self, log_path: &Path) -> Result<ValidationTarget, String> {
        let path = self
            .output_path
            .or(self.output_artifact_path)
            .ok_or_else(|| {
                format!(
                    "matched provenance record in {} has no output path",
                    log_path.display()
                )
            })?;
        let target_artifact_id = self
            .derived_artifact_id
            .clone()
            .or_else(|| self.id.clone())
            .or_else(|| self.inode.clone())
            .or_else(|| self.selector.clone());
        Ok(ValidationTarget {
            path: PathBuf::from(path),
            source_artifact_id: self.source_artifact_id,
            source_artifact_path: self
                .source_artifact_path
                .or(self.source_path)
                .map(PathBuf::from),
            source_artifact_sha256: self.source_artifact_sha256.or(self.source_index_sha256),
            derived_artifact_id: self.derived_artifact_id,
            target_artifact_id,
        })
    }
}

fn canonical_case_target(case_dir: &Path, target_path: &Path) -> Result<PathBuf, String> {
    let candidate = if target_path.is_absolute() {
        target_path.to_path_buf()
    } else {
        case_dir.join(target_path)
    };
    let path = candidate
        .canonicalize()
        .map_err(|err| format!("failed to canonicalize validation target: {err}"))?;
    if !is_case_contained(case_dir, &path)? {
        return Err(format!(
            "audited validation target is outside the case directory: {}",
            path.display()
        ));
    }
    Ok(path)
}

fn is_case_contained(case_dir: &Path, target_path: &Path) -> Result<bool, String> {
    let case_dir = case_dir
        .canonicalize()
        .map_err(|err| format!("failed to canonicalize case directory: {err}"))?;
    Ok(target_path.starts_with(case_dir))
}
