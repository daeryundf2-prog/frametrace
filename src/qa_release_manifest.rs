use crate::qa::REVIEW_GATES;
use crate::util::read_to_string;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug)]
pub(crate) struct ReviewManifestEvaluation {
    pub(crate) gates: HashMap<String, bool>,
    pub(crate) errors: HashMap<String, String>,
}

pub(crate) fn evaluate_review_manifest(path: &Path) -> Result<ReviewManifestEvaluation, String> {
    let text = read_to_string(path).map_err(|err| {
        format!(
            "failed to read release review manifest {}: {err}",
            path.display()
        )
    })?;
    let manifest: ReviewManifest = serde_json::from_str(&text).map_err(|err| {
        format!(
            "release review manifest {} must be a typed JSON review manifest: {err}",
            path.display()
        )
    })?;
    if manifest.schema_version != 1 {
        return Err(format!(
            "release review manifest {} has unsupported schema_version {}; expected 1",
            path.display(),
            manifest.schema_version
        ));
    }
    let mut gates = HashMap::with_capacity(manifest.gates.len());
    let mut errors = HashMap::new();
    for entry in manifest.gates {
        let key = normalize_review_gate(&entry.key);
        if is_release_gate_key(&key) {
            match entry.validate(path, &key) {
                Ok(()) => {
                    gates.insert(key, true);
                }
                Err(err) => {
                    errors.insert(key, err);
                }
            }
        } else {
            return Err(format!(
                "unknown review gate key `{key}` in {}",
                path.display()
            ));
        }
    }
    Ok(ReviewManifestEvaluation { gates, errors })
}

#[cfg(test)]
pub(crate) fn read_review_manifest(path: &Path) -> Result<HashMap<String, bool>, String> {
    let evaluation = evaluate_review_manifest(path)?;
    if let Some(err) = evaluation.errors.values().next() {
        return Err(err.clone());
    }
    Ok(evaluation.gates)
}

#[derive(Debug, Deserialize)]
struct ReviewManifest {
    schema_version: u64,
    gates: Vec<ReviewGateEntry>,
}

#[derive(Debug, Deserialize)]
struct ReviewGateEntry {
    key: String,
    status: String,
    artifact_path: String,
    tool: String,
    timestamp: String,
    reviewer: Option<String>,
    operator: Option<String>,
    cleanup_status: String,
}

impl ReviewGateEntry {
    fn validate(&self, manifest_path: &Path, key: &str) -> Result<(), String> {
        require_non_empty(key, "artifact_path", &self.artifact_path)?;
        require_non_empty(key, "tool", &self.tool)?;
        require_non_empty(key, "timestamp", &self.timestamp)?;
        require_non_empty(key, "cleanup_status", &self.cleanup_status)?;
        if !self.status.eq_ignore_ascii_case("PASS") {
            return Err(format!(
                "review gate {key} has status `{}`; expected typed PASS artifact",
                self.status
            ));
        }
        if !self.cleanup_status.eq_ignore_ascii_case("clean") {
            return Err(format!(
                "review gate {key} cleanup_status `{}` is not clean",
                self.cleanup_status
            ));
        }
        if self
            .reviewer
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
            && self
                .operator
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
        {
            return Err(format!(
                "review gate {key} requires reviewer or operator metadata"
            ));
        }
        let artifact_path = manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&self.artifact_path);
        if !artifact_path.is_file() {
            return Err(format!(
                "review gate {key} artifact_path `{}` does not exist",
                artifact_path.display()
            ));
        }
        Ok(())
    }
}

fn require_non_empty(key: &str, field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("review gate {key} requires {field}"))
    } else {
        Ok(())
    }
}

fn normalize_review_gate(value: &str) -> String {
    value
        .trim()
        .trim_matches(|ch: char| !ch.is_alphanumeric())
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn is_release_gate_key(key: &str) -> bool {
    REVIEW_GATES.iter().any(|(gate_key, _)| *gate_key == key)
}
