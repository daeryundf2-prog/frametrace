use super::qa_release_decision::{ReleaseDecisionCheck, release_decision_json};
use super::qa_release_manifest::evaluate_review_manifest;
use crate::qa::{
    QaReport, REVIEW_GATES, ReleaseReadinessOptions, accuracy_report, performance_report,
    privacy_review_check, report_defense_check, reproducibility_report,
    workstation_shell_contract_check,
};
use crate::util::{json_escape, now_unix, write_text};
use crate::windows_prerequisites;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

pub fn release_readiness_report(
    case_dir: &Path,
    output_dir: &Path,
    options: &ReleaseReadinessOptions,
) -> Result<QaReport, String> {
    fs::create_dir_all(output_dir)
        .map_err(|err| format!("failed to create QA output directory: {err}"))?;
    let mut checks = Vec::new();

    checks.push(run_typed_qa_input(
        "privacy_review",
        "privacy_review",
        output_dir.join("privacy-review.json"),
        || privacy_review_check(case_dir, output_dir),
    ));
    checks.push(run_typed_qa_input(
        "report_defense",
        "report_defense",
        output_dir.join("report-defense-report.json"),
        || report_defense_check(case_dir, output_dir),
    ));
    checks.push(run_release_check("workstation_shell_contract", || {
        workstation_shell_contract_check(case_dir, output_dir)
    }));
    checks.push(run_release_check("windows_prerequisites", || {
        windows_prerequisites::release_validation_check(output_dir)
    }));
    checks.extend(review_gate_checks(options.review_manifest.as_deref()));

    if let Some(corpus_manifest) = &options.corpus_manifest {
        checks.push(run_release_check("accuracy", || {
            accuracy_report(case_dir, corpus_manifest, output_dir).map(|report| report.report_path)
        }));
    } else {
        checks.push(ReleaseCheck::blocked(
            "accuracy",
            "missing --corpus-manifest",
        ));
    }

    if let Some(comparison_case_dir) = &options.comparison_case_dir {
        checks.push(run_release_check("reproducibility", || {
            reproducibility_report(case_dir, comparison_case_dir, output_dir)
                .map(|report| report.report_path)
        }));
    } else {
        checks.push(ReleaseCheck::blocked(
            "reproducibility",
            "missing --comparison-case",
        ));
    }

    let performance_output_dir = options
        .performance_output_dir
        .clone()
        .unwrap_or_else(|| output_dir.join("performance"));
    checks.push(run_release_check("performance", || {
        performance_report(&performance_output_dir, options.performance_rows)
            .map(|report| report.report_path)
    }));

    let passed = checks.iter().all(|check| check.status == "PASS");
    let blocker_count = checks.iter().filter(|check| check.status != "PASS").count();
    let json_path = output_dir.join("release-readiness.json");
    let markdown_path = output_dir.join("release-readiness.md");
    let decision_path = output_dir.join("release-decision.json");
    let generated_at_unix = now_unix()?;
    let decision_checks = checks
        .iter()
        .map(|check| ReleaseDecisionCheck {
            name: &check.name,
            status: &check.status,
            evidence: &check.evidence,
        })
        .collect::<Vec<_>>();
    write_text(&json_path, &release_json(passed, &checks))
        .map_err(|err| format!("failed to write release readiness JSON: {err}"))?;
    write_text(&markdown_path, &release_markdown(passed, &checks))
        .map_err(|err| format!("failed to write release readiness checklist: {err}"))?;
    write_text(
        &decision_path,
        &release_decision_json(generated_at_unix, &decision_checks),
    )
    .map_err(|err| format!("failed to write release decision JSON: {err}"))?;

    if passed {
        Ok(QaReport {
            report_path: json_path,
            passed,
        })
    } else {
        let blocker_summary = checks
            .iter()
            .filter(|check| check.status != "PASS")
            .map(|check| format!("{}: {}", check.name, check.evidence))
            .collect::<Vec<_>>()
            .join("; ");
        Err(format!(
            "release readiness failed: {blocker_count} blocker(s): {blocker_summary}; see {}",
            markdown_path.display()
        ))
    }
}

#[derive(Debug, Clone)]
struct ReleaseCheck {
    name: String,
    status: String,
    evidence: String,
}

impl ReleaseCheck {
    fn blocked(name: &str, reason: &str) -> Self {
        Self {
            name: name.to_string(),
            status: "BLOCKED".to_string(),
            evidence: reason.to_string(),
        }
    }
}

fn run_release_check(name: &str, run: impl FnOnce() -> Result<PathBuf, String>) -> ReleaseCheck {
    match run() {
        Ok(path) => ReleaseCheck {
            name: name.to_string(),
            status: "PASS".to_string(),
            evidence: path.to_string_lossy().to_string(),
        },
        Err(err) => ReleaseCheck {
            name: name.to_string(),
            status: "FAIL".to_string(),
            evidence: err,
        },
    }
}

fn run_typed_qa_input(
    name: &str,
    expected_qa_type: &str,
    artifact_path: PathBuf,
    run: impl FnOnce() -> Result<QaReport, String>,
) -> ReleaseCheck {
    match run() {
        Ok(_) => release_check_from_typed_artifact(name, expected_qa_type, &artifact_path),
        Err(run_error) => match read_typed_qa_artifact(&artifact_path, expected_qa_type) {
            Ok(artifact) if !artifact.passed => {
                typed_artifact_failure(name, &artifact_path, &artifact)
            }
            _ => ReleaseCheck {
                name: name.to_string(),
                status: "FAIL".to_string(),
                evidence: run_error,
            },
        },
    }
}

fn release_check_from_typed_artifact(
    name: &str,
    expected_qa_type: &str,
    artifact_path: &Path,
) -> ReleaseCheck {
    match read_typed_qa_artifact(artifact_path, expected_qa_type) {
        Ok(artifact) if artifact.passed => ReleaseCheck {
            name: name.to_string(),
            status: "PASS".to_string(),
            evidence: artifact_path.to_string_lossy().to_string(),
        },
        Ok(artifact) => typed_artifact_failure(name, artifact_path, &artifact),
        Err(err) => ReleaseCheck {
            name: name.to_string(),
            status: "FAIL".to_string(),
            evidence: err,
        },
    }
}

fn typed_artifact_failure(
    name: &str,
    artifact_path: &Path,
    artifact: &TypedQaArtifact,
) -> ReleaseCheck {
    let failing_keys = artifact.failing_keys().join(",");
    ReleaseCheck {
        name: name.to_string(),
        status: "FAIL".to_string(),
        evidence: format!("{}: finding_keys={failing_keys}", artifact_path.display()),
    }
}

fn review_gate_checks(manifest_path: Option<&Path>) -> Vec<ReleaseCheck> {
    let Some(path) = manifest_path else {
        return REVIEW_GATES
            .iter()
            .map(|(key, _)| {
                ReleaseCheck::blocked(&review_gate_check_name(key), "missing --review-manifest")
            })
            .collect();
    };

    match evaluate_review_manifest(path) {
        Ok(evaluation) => REVIEW_GATES
            .iter()
            .map(|(key, label)| match evaluation.errors.get(*key) {
                Some(err) => ReleaseCheck {
                    name: review_gate_check_name(key),
                    status: err.status.clone(),
                    evidence: err.message.clone(),
                },
                None if evaluation.gates.get(*key).copied().unwrap_or(false) => ReleaseCheck {
                    name: review_gate_check_name(key),
                    status: "PASS".to_string(),
                    evidence: path.to_string_lossy().to_string(),
                },
                None => ReleaseCheck {
                    name: review_gate_check_name(key),
                    status: "BLOCKED".to_string(),
                    evidence: format!("{label} ({key}) is not approved in {}", path.display()),
                },
            })
            .collect(),
        Err(err) => REVIEW_GATES
            .iter()
            .map(|(key, _)| ReleaseCheck {
                name: review_gate_check_name(key),
                status: "FAIL".to_string(),
                evidence: err.clone(),
            })
            .collect(),
    }
}

fn review_gate_check_name(key: &str) -> String {
    format!("review_gate_{key}")
}

fn release_json(passed: bool, checks: &[ReleaseCheck]) -> String {
    let checks_json = checks
        .iter()
        .map(|check| {
            format!(
                "    {{\"name\":\"{}\",\"status\":\"{}\",\"evidence\":\"{}\"}}",
                json_escape(&check.name),
                json_escape(&check.status),
                json_escape(&check.evidence)
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        "{{\n  \"schema_version\": 1,\n  \"qa_type\": \"release_readiness\",\n  \"passed\": {},\n  \"checks\": [\n{}\n  ]\n}}\n",
        passed, checks_json
    )
}

fn release_markdown(passed: bool, checks: &[ReleaseCheck]) -> String {
    let mut text = String::from("# Release Readiness\n\n");
    text.push_str(&format!(
        "Overall: **{}**\n\n",
        if passed { "PASS" } else { "BLOCKED" }
    ));
    text.push_str("| Check | Status | Evidence |\n| --- | --- | --- |\n");
    for check in checks {
        text.push_str(&format!(
            "| {} | {} | `{}` |\n",
            check.name, check.status, check.evidence
        ));
    }
    text
}

#[derive(Debug, Deserialize)]
struct TypedQaArtifact {
    schema_version: u64,
    qa_type: String,
    passed: bool,
    findings: Vec<TypedQaFinding>,
}

impl TypedQaArtifact {
    fn failing_keys(&self) -> Vec<String> {
        self.findings
            .iter()
            .filter(|finding| matches!(finding.status.as_str(), "failed" | "partial" | "skipped"))
            .map(|finding| finding.key.clone())
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct TypedQaFinding {
    key: String,
    status: String,
}

fn read_typed_qa_artifact(path: &Path, expected_qa_type: &str) -> Result<TypedQaArtifact, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("failed to read typed QA input {}: {err}", path.display()))?;
    let artifact = serde_json::from_str::<TypedQaArtifact>(&text).map_err(|err| {
        format!(
            "typed QA input {} must be machine-readable JSON: {err}",
            path.display()
        )
    })?;
    if artifact.schema_version != 1 {
        return Err(format!(
            "typed QA input {} has unsupported schema_version {}; expected 1",
            path.display(),
            artifact.schema_version
        ));
    }
    if artifact.qa_type != expected_qa_type {
        return Err(format!(
            "typed QA input {} has qa_type `{}`; expected `{expected_qa_type}`",
            path.display(),
            artifact.qa_type
        ));
    }
    Ok(artifact)
}
