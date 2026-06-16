use crate::qa::{
    QaReport, ReleaseReadinessOptions, accuracy_report, performance_report, report_defense_check,
    reproducibility_report,
};
use crate::util::{json_escape, read_to_string, write_text};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const REVIEW_GATES: &[(&str, &str)] = &[
    ("technical_review", "Technical Review"),
    ("security_review", "Security Review"),
    ("migration_validation", "Migration Validation"),
    ("operator_review", "Operator Review"),
    ("legal_review", "Legal Review"),
];

pub fn release_readiness_report(
    case_dir: &Path,
    output_dir: &Path,
    options: &ReleaseReadinessOptions,
) -> Result<QaReport, String> {
    fs::create_dir_all(output_dir)
        .map_err(|err| format!("failed to create QA output directory: {err}"))?;
    let mut checks = Vec::new();

    checks.push(run_release_check("report_defense", || {
        report_defense_check(case_dir, output_dir).map(|report| report.report_path)
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
    write_text(&json_path, &release_json(passed, &checks))
        .map_err(|err| format!("failed to write release readiness JSON: {err}"))?;
    write_text(&markdown_path, &release_markdown(passed, &checks))
        .map_err(|err| format!("failed to write release readiness checklist: {err}"))?;

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

fn review_gate_checks(manifest_path: Option<&Path>) -> Vec<ReleaseCheck> {
    let Some(path) = manifest_path else {
        return REVIEW_GATES
            .iter()
            .map(|(key, _)| ReleaseCheck::blocked(key, "missing --review-manifest"))
            .collect();
    };

    match read_review_manifest(path) {
        Ok(gates) => REVIEW_GATES
            .iter()
            .map(|(key, label)| {
                if gates.get(*key).copied().unwrap_or(false) {
                    ReleaseCheck {
                        name: (*key).to_string(),
                        status: "PASS".to_string(),
                        evidence: path.to_string_lossy().to_string(),
                    }
                } else {
                    ReleaseCheck {
                        name: (*key).to_string(),
                        status: "FAIL".to_string(),
                        evidence: format!("{label} is not approved in {}", path.display()),
                    }
                }
            })
            .collect(),
        Err(err) => REVIEW_GATES
            .iter()
            .map(|(key, _)| ReleaseCheck {
                name: (*key).to_string(),
                status: "FAIL".to_string(),
                evidence: err.clone(),
            })
            .collect(),
    }
}

pub(crate) fn read_review_manifest(path: &Path) -> Result<HashMap<String, bool>, String> {
    let text = read_to_string(path).map_err(|err| {
        format!(
            "failed to read release review manifest {}: {err}",
            path.display()
        )
    })?;
    let mut gates = HashMap::new();
    for (line_index, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((checked, label)) = parse_markdown_checkbox(line) {
            gates.insert(normalize_review_gate(label), checked);
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            gates.insert(normalize_review_gate(key), review_value_is_pass(value));
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            gates.insert(normalize_review_gate(key), review_value_is_pass(value));
            continue;
        }
        return Err(format!(
            "invalid review manifest row {} in {}",
            line_index + 1,
            path.display()
        ));
    }
    Ok(gates)
}

fn parse_markdown_checkbox(line: &str) -> Option<(bool, &str)> {
    let rest = line.strip_prefix('-')?.trim_start();
    let checked = rest
        .strip_prefix("[x]")
        .or_else(|| rest.strip_prefix("[X]"));
    if let Some(label) = checked {
        return Some((true, label.trim()));
    }
    rest.strip_prefix("[ ]").map(|label| (false, label.trim()))
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

fn review_value_is_pass(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "pass" | "passed" | "approved" | "true" | "yes" | "complete" | "completed" | "done" | "x"
    )
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
