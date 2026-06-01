use crate::case_db;
use crate::util::{html_escape, json_escape, read_to_string, write_text};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const PRECISION_TARGET: f64 = 0.98;
const RECALL_TARGET: f64 = 0.98;

#[derive(Debug, Clone)]
struct ExpectedEvidence {
    source_path: String,
    sha256: Option<String>,
}

#[derive(Debug, Clone)]
struct IndexedEvidence {
    source_path: String,
    sha256: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QaReport {
    pub report_path: PathBuf,
    pub passed: bool,
}

#[derive(Debug, Clone)]
pub struct ReleaseReadinessOptions {
    pub corpus_manifest: Option<PathBuf>,
    pub comparison_case_dir: Option<PathBuf>,
    pub review_manifest: Option<PathBuf>,
    pub performance_output_dir: Option<PathBuf>,
    pub performance_rows: usize,
}

const REVIEW_GATES: &[(&str, &str)] = &[
    ("technical_review", "Technical Review"),
    ("security_review", "Security Review"),
    ("migration_validation", "Migration Validation"),
    ("operator_review", "Operator Review"),
    ("legal_review", "Legal Review"),
];

const DISALLOWED_REPORT_CLAIMS: &[&str] =
    &["court-ready", "court-grade", "court-proven", "legal-grade"];

pub fn accuracy_report(
    case_dir: &Path,
    corpus_manifest: &Path,
    output_dir: &Path,
) -> Result<QaReport, String> {
    let expected = read_expected_manifest(corpus_manifest)?;
    let indexed = read_indexed_evidence(case_dir)?;
    let indexed_by_source = indexed
        .iter()
        .map(|item| (item.source_path.as_str(), item))
        .collect::<HashMap<_, _>>();
    let expected_sources = expected
        .iter()
        .map(|item| item.source_path.as_str())
        .collect::<HashSet<_>>();

    let mut true_positive = 0usize;
    let mut false_negative = 0usize;
    let mut hash_mismatch = 0usize;
    for item in &expected {
        match indexed_by_source.get(item.source_path.as_str()) {
            Some(indexed) if item.sha256.is_none() || item.sha256 == indexed.sha256 => {
                true_positive += 1;
            }
            Some(_) => {
                false_negative += 1;
                hash_mismatch += 1;
            }
            None => false_negative += 1,
        }
    }
    let false_positive = indexed
        .iter()
        .filter(|item| !expected_sources.contains(item.source_path.as_str()))
        .count();
    let predicted_positive = true_positive + false_positive;
    let ground_truth_positive = expected.len();
    let precision = if predicted_positive == 0 {
        1.0
    } else {
        true_positive as f64 / predicted_positive as f64
    };
    let recall = if ground_truth_positive == 0 {
        1.0
    } else {
        true_positive as f64 / ground_truth_positive as f64
    };
    let passed = precision >= PRECISION_TARGET && recall >= RECALL_TARGET && hash_mismatch == 0;

    fs::create_dir_all(output_dir)
        .map_err(|err| format!("failed to create QA output directory: {err}"))?;
    let json_path = output_dir.join("accuracy-report.json");
    let html_path = output_dir.join("accuracy-report.html");
    write_text(
        &json_path,
        &format!(
            "{{\n  \"schema_version\": 1,\n  \"qa_type\": \"accuracy\",\n  \"passed\": {},\n  \"precision\": {:.6},\n  \"recall\": {:.6},\n  \"true_positive\": {},\n  \"false_positive\": {},\n  \"false_negative\": {},\n  \"hash_mismatch\": {},\n  \"expected_count\": {},\n  \"indexed_count\": {}\n}}\n",
            passed,
            precision,
            recall,
            true_positive,
            false_positive,
            false_negative,
            hash_mismatch,
            expected.len(),
            indexed.len()
        ),
    )
    .map_err(|err| format!("failed to write accuracy JSON report: {err}"))?;
    write_text(
        &html_path,
        &simple_html_report(
            "FrameTrace Accuracy QA",
            &format!(
                "passed={} precision={:.6} recall={:.6} tp={} fp={} fn={} hash_mismatch={}",
                passed,
                precision,
                recall,
                true_positive,
                false_positive,
                false_negative,
                hash_mismatch
            ),
        ),
    )
    .map_err(|err| format!("failed to write accuracy HTML report: {err}"))?;

    if passed {
        Ok(QaReport {
            report_path: json_path,
            passed,
        })
    } else {
        Err(format!(
            "accuracy QA failed: precision={precision:.6}, recall={recall:.6}, false_positive={false_positive}, false_negative={false_negative}, hash_mismatch={hash_mismatch}"
        ))
    }
}

pub fn reproducibility_report(
    left_case_dir: &Path,
    right_case_dir: &Path,
    output_dir: &Path,
) -> Result<QaReport, String> {
    let left = normalized_case_core(left_case_dir)?;
    let right = normalized_case_core(right_case_dir)?;
    let passed = left == right;
    fs::create_dir_all(output_dir)
        .map_err(|err| format!("failed to create QA output directory: {err}"))?;
    let report_path = output_dir.join("reproducibility-report.json");
    write_text(
        &report_path,
        &format!(
            "{{\n  \"schema_version\": 1,\n  \"qa_type\": \"reproducibility\",\n  \"passed\": {},\n  \"left_case\": \"{}\",\n  \"right_case\": \"{}\",\n  \"normalized_left_bytes\": {},\n  \"normalized_right_bytes\": {}\n}}\n",
            passed,
            json_escape(&left_case_dir.to_string_lossy()),
            json_escape(&right_case_dir.to_string_lossy()),
            left.len(),
            right.len()
        ),
    )
    .map_err(|err| format!("failed to write reproducibility report: {err}"))?;
    if passed {
        Ok(QaReport {
            report_path,
            passed,
        })
    } else {
        Err("reproducibility QA failed: normalized core outputs differ".to_string())
    }
}

pub fn report_defense_check(case_dir: &Path, output_dir: &Path) -> Result<QaReport, String> {
    let checks = [
        ("case manifest", case_dir.join("case.json")),
        ("case database", case_db::case_db_path(case_dir)),
        ("video JSON index", case_dir.join("db/video_index.json")),
        ("video JSONL index", case_dir.join("db/videos.jsonl")),
        ("video path TSV", case_dir.join("db/video_paths.tsv")),
        ("case report", case_dir.join("reports/case-report.html")),
    ];
    let missing = checks
        .iter()
        .filter(|(_, path)| !path.is_file())
        .map(|(name, path)| format!("{name}: {}", path.display()))
        .collect::<Vec<_>>();
    let claim_violations = report_claim_violations(case_dir)?;
    let passed = missing.is_empty() && claim_violations.is_empty();
    fs::create_dir_all(output_dir)
        .map_err(|err| format!("failed to create QA output directory: {err}"))?;
    let report_path = output_dir.join("report-defense-checklist.md");
    let mut text = String::from("# Report Defensibility Checklist\n\n");
    for (name, path) in checks {
        let status = if path.is_file() { "PASS" } else { "FAIL" };
        text.push_str(&format!("- [{status}] {name}: `{}`\n", path.display()));
    }
    if !missing.is_empty() {
        text.push_str("\n## Missing\n\n");
        for item in &missing {
            text.push_str(&format!("- {item}\n"));
        }
    }
    if !claim_violations.is_empty() {
        text.push_str("\n## Disallowed Report Claims\n\n");
        for item in &claim_violations {
            text.push_str(&format!("- {item}\n"));
        }
    }
    write_text(&report_path, &text)
        .map_err(|err| format!("failed to write report-defense checklist: {err}"))?;
    if passed {
        Ok(QaReport {
            report_path,
            passed,
        })
    } else {
        Err(format!(
            "report defensibility QA failed: missing {} required artifacts, {} disallowed claim(s)",
            missing.len(),
            claim_violations.len()
        ))
    }
}

fn report_claim_violations(case_dir: &Path) -> Result<Vec<String>, String> {
    let mut violations = Vec::new();
    for rel in ["reports/case-report.html", "review/evidence-viewer.html"] {
        let path = case_dir.join(rel);
        if !path.is_file() {
            continue;
        }
        let text = read_to_string(&path)
            .map_err(|err| format!("failed to read report output {}: {err}", path.display()))?;
        let lower = text.to_ascii_lowercase();
        for term in DISALLOWED_REPORT_CLAIMS {
            if lower.contains(term) {
                violations.push(format!("{rel}: contains disallowed claim `{term}`"));
            }
        }
    }
    Ok(violations)
}

pub fn performance_report(output_dir: &Path, rows: usize) -> Result<QaReport, String> {
    let result = case_db::benchmark_case_db(output_dir, rows)?;
    let report_path = output_dir.join("performance-report.json");
    let rows_per_minute = if result.elapsed_ms == 0 {
        rows as u128 * 60_000
    } else {
        rows as u128 * 60_000 / result.elapsed_ms
    };
    let passed = rows_per_minute >= 50_000;
    write_text(
        &report_path,
        &format!(
            "{{\n  \"schema_version\": 1,\n  \"qa_type\": \"performance\",\n  \"passed\": {},\n  \"rows\": {},\n  \"elapsed_ms\": {},\n  \"rows_per_minute\": {},\n  \"database_path\": \"{}\"\n}}\n",
            passed,
            result.rows,
            result.elapsed_ms,
            rows_per_minute,
            json_escape(&result.path.to_string_lossy())
        ),
    )
    .map_err(|err| format!("failed to write performance report: {err}"))?;
    if passed {
        Ok(QaReport {
            report_path,
            passed,
        })
    } else {
        Err(format!(
            "performance QA failed: rows_per_minute={rows_per_minute}, target=50000"
        ))
    }
}

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

fn read_review_manifest(path: &Path) -> Result<HashMap<String, bool>, String> {
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

fn read_expected_manifest(path: &Path) -> Result<Vec<ExpectedEvidence>, String> {
    let text = read_to_string(path)
        .map_err(|err| format!("failed to read corpus manifest {}: {err}", path.display()))?;
    let mut out = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("source_path") {
            continue;
        }
        let columns = line.split('\t').collect::<Vec<_>>();
        if columns.is_empty() || columns[0].trim().is_empty() {
            return Err(format!(
                "invalid corpus manifest row {} in {}",
                line_index + 1,
                path.display()
            ));
        }
        out.push(ExpectedEvidence {
            source_path: columns[0].trim().to_string(),
            sha256: columns
                .get(1)
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(str::to_string),
        });
    }
    Ok(out)
}

fn read_indexed_evidence(case_dir: &Path) -> Result<Vec<IndexedEvidence>, String> {
    let mut indexed = HashMap::<String, IndexedEvidence>::new();

    let path = case_dir.join("db/videos.jsonl");
    let text = read_to_string(&path)
        .map_err(|err| format!("failed to read indexed evidence {}: {err}", path.display()))?;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        insert_indexed_evidence(
            &mut indexed,
            extract_json_string(line, "source_path"),
            extract_json_string(line, "sha256"),
        );
    }

    for rel_log in [
        "artifacts/carved/carve-log.jsonl",
        "evidence/logs/tsk-audit.jsonl",
        "evidence/logs/validation-log.jsonl",
    ] {
        let log_path = case_dir.join(rel_log);
        let text = read_to_string(&log_path).unwrap_or_default();
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let source_path = extract_json_string(line, "output_path")
                .or_else(|| extract_json_string(line, "target_path"));
            let sha256 = extract_json_string(line, "sha256")
                .or_else(|| extract_json_string(line, "target_sha256"));
            insert_indexed_evidence(&mut indexed, source_path, sha256);
        }
    }

    let mut out = indexed.into_values().collect::<Vec<_>>();
    out.sort_by(|left, right| left.source_path.cmp(&right.source_path));
    Ok(out)
}

fn insert_indexed_evidence(
    indexed: &mut HashMap<String, IndexedEvidence>,
    source_path: Option<String>,
    sha256: Option<String>,
) {
    let Some(source_path) = source_path.filter(|value| !value.trim().is_empty()) else {
        return;
    };
    indexed
        .entry(source_path.clone())
        .and_modify(|item| {
            if sha256.is_some() {
                item.sha256 = sha256.clone();
            }
        })
        .or_insert(IndexedEvidence {
            source_path,
            sha256,
        });
}

fn normalized_case_core(case_dir: &Path) -> Result<String, String> {
    let mut parts = Vec::new();
    for rel in ["db/videos.jsonl", "db/video_paths.tsv"] {
        parts.push(normalized_required_file_part(case_dir, rel)?);
    }
    for rel in [
        "db/carve_results.json",
        "artifacts/carved/carve-log.jsonl",
        "evidence/logs/tsk-audit.jsonl",
        "evidence/logs/validation-log.jsonl",
    ] {
        if let Some(part) = normalized_optional_file_part(case_dir, rel)? {
            parts.push(part);
        }
    }
    parts.extend(normalized_optional_dir_parts(case_dir, "db/filesystem")?);
    parts.extend(normalized_package_manifest_parts(case_dir)?);
    parts.sort();
    Ok(parts.join("\n"))
}

fn normalized_required_file_part(case_dir: &Path, rel: &str) -> Result<String, String> {
    let path = case_dir.join(rel);
    let text = read_to_string(&path).map_err(|err| {
        format!(
            "failed to read reproducibility input {}: {err}",
            path.display()
        )
    })?;
    Ok(normalized_text_part(case_dir, rel, &text))
}

fn normalized_optional_file_part(case_dir: &Path, rel: &str) -> Result<Option<String>, String> {
    let path = case_dir.join(rel);
    if !path.is_file() {
        return Ok(None);
    }
    let text = read_to_string(&path).map_err(|err| {
        format!(
            "failed to read reproducibility input {}: {err}",
            path.display()
        )
    })?;
    Ok(Some(normalized_text_part(case_dir, rel, &text)))
}

fn normalized_optional_dir_parts(case_dir: &Path, rel_dir: &str) -> Result<Vec<String>, String> {
    let dir = case_dir.join(rel_dir);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect_files_recursively(&dir, &mut files)?;
    files.sort();

    let mut normalized_contents = Vec::new();
    for path in files {
        let text = read_to_string(&path).map_err(|err| {
            format!(
                "failed to read reproducibility input {}: {err}",
                path.display()
            )
        })?;
        normalized_contents.push(normalized_text_part(case_dir, rel_dir, &text));
    }
    normalized_contents.sort();
    Ok(normalized_contents
        .into_iter()
        .enumerate()
        .map(|(index, part)| format!("{rel_dir}_artifact_{index}\n{part}"))
        .collect())
}

fn normalized_package_manifest_parts(case_dir: &Path) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    collect_files_recursively(case_dir, &mut files)?;
    files.retain(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "package-manifest.json" || name == "manifest.sha256")
    });

    let mut normalized_contents = Vec::new();
    for path in files {
        let text = read_to_string(&path).map_err(|err| {
            format!(
                "failed to read package reproducibility input {}: {err}",
                path.display()
            )
        })?;
        let label = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("package-manifest");
        normalized_contents.push(normalized_text_part(case_dir, label, &text));
    }
    normalized_contents.sort();
    Ok(normalized_contents
        .into_iter()
        .enumerate()
        .map(|(index, part)| format!("package_artifact_{index}\n{part}"))
        .collect())
}

fn collect_files_recursively(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir)
        .map_err(|err| format!("failed to read directory {}: {err}", dir.display()))?
    {
        let entry = entry.map_err(|err| format!("failed to read directory entry: {err}"))?;
        let file_type = entry
            .file_type()
            .map_err(|err| format!("failed to read file type for {:?}: {err}", entry.path()))?;
        if file_type.is_dir() {
            collect_files_recursively(&entry.path(), out)?;
        } else if file_type.is_file() {
            out.push(entry.path());
        }
    }
    Ok(())
}

fn normalized_text_part(case_dir: &Path, label: &str, text: &str) -> String {
    let mut normalized = normalize_case_paths(case_dir, text);
    for key in VOLATILE_NUMERIC_FIELDS {
        normalized = replace_json_number_field(&normalized, key, "0");
    }
    for key in VOLATILE_STRING_FIELDS {
        normalized = replace_json_string_field(&normalized, key, "<VOLATILE>");
    }
    let mut lines = normalized
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    lines.sort();
    format!("{label}\n{}\n", lines.join("\n"))
}

const VOLATILE_NUMERIC_FIELDS: &[&str] = &[
    "created_unix",
    "scanned_unix",
    "first_indexed_unix",
    "last_indexed_unix",
    "last_scanned_unix",
    "registered_unix",
    "last_seen_unix",
    "started_unix",
    "updated_unix",
    "completed_unix",
    "event_unix",
    "carved_unix",
    "inspected_unix",
    "recovered_unix",
    "validated_unix",
];

const VOLATILE_STRING_FIELDS: &[&str] = &["entry_sha256", "previous_entry_sha256"];

fn normalize_case_paths(case_dir: &Path, text: &str) -> String {
    let mut out = text.replace('\\', "/");
    let raw = case_dir.to_string_lossy().replace('\\', "/");
    out = out.replace(&raw, "<CASE>");
    if let Ok(canonical) = case_dir.canonicalize() {
        let canonical = canonical.to_string_lossy().replace('\\', "/");
        out = out.replace(&canonical, "<CASE>");
    }
    out
}

fn replace_json_number_field(text: &str, key: &str, replacement: &str) -> String {
    let marker = format!("\"{key}\":");
    let mut out = String::with_capacity(text.len());
    let mut remaining = text;
    while let Some(index) = remaining.find(&marker) {
        out.push_str(&remaining[..index + marker.len()]);
        let after_marker = &remaining[index + marker.len()..];
        let whitespace_len = after_marker
            .char_indices()
            .find(|(_, ch)| !ch.is_whitespace())
            .map(|(idx, _)| idx)
            .unwrap_or(after_marker.len());
        out.push_str(&after_marker[..whitespace_len]);
        let after_whitespace = &after_marker[whitespace_len..];
        let number_len = after_whitespace
            .char_indices()
            .take_while(|(_, ch)| ch.is_ascii_digit())
            .last()
            .map(|(idx, ch)| idx + ch.len_utf8())
            .unwrap_or(0);
        if number_len == 0 {
            remaining = after_marker;
            continue;
        }
        out.push_str(replacement);
        remaining = &after_whitespace[number_len..];
    }
    out.push_str(remaining);
    out
}

fn replace_json_string_field(text: &str, key: &str, replacement: &str) -> String {
    let marker = format!("\"{key}\":");
    let mut out = String::with_capacity(text.len());
    let mut remaining = text;
    while let Some(index) = remaining.find(&marker) {
        out.push_str(&remaining[..index + marker.len()]);
        let after_marker = &remaining[index + marker.len()..];
        let whitespace_len = after_marker
            .char_indices()
            .find(|(_, ch)| !ch.is_whitespace())
            .map(|(idx, _)| idx)
            .unwrap_or(after_marker.len());
        out.push_str(&after_marker[..whitespace_len]);
        let after_whitespace = &after_marker[whitespace_len..];
        let Some(rest) = after_whitespace.strip_prefix('"') else {
            remaining = after_marker;
            continue;
        };
        let Some(string_len) = json_string_content_len(rest) else {
            remaining = after_marker;
            continue;
        };
        out.push('"');
        out.push_str(replacement);
        out.push('"');
        remaining = &rest[string_len + 1..];
    }
    out.push_str(remaining);
    out
}

fn json_string_content_len(value: &str) -> Option<usize> {
    let mut escaping = false;
    for (index, ch) in value.char_indices() {
        if escaping {
            escaping = false;
            continue;
        }
        if ch == '\\' {
            escaping = true;
            continue;
        }
        if ch == '"' {
            return Some(index);
        }
    }
    None
}

fn extract_json_string(line: &str, key: &str) -> Option<String> {
    let key = format!("\"{}\":", key);
    let start = line.find(&key)? + key.len();
    let value = line[start..].trim_start();
    if value.starts_with("null") {
        return None;
    }
    let value = value.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'b' => out.push('\u{08}'),
                'f' => out.push('\u{0C}'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                other => out.push(other),
            },
            other => out.push(other),
        }
    }
    None
}

fn simple_html_report(title: &str, body: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title></head><body><h1>{}</h1><pre>{}</pre></body></html>\n",
        html_escape(title),
        html_escape(title),
        html_escape(body)
    )
}

#[cfg(test)]
mod tests {
    use super::{
        accuracy_report, read_review_manifest, report_defense_check, reproducibility_report,
    };
    use std::fs;

    #[test]
    fn accuracy_report_passes_for_matching_manifest() {
        let root = std::env::temp_dir().join(format!("frametrace-qa-test-{}", std::process::id()));
        let case_dir = root.join("case");
        let output_dir = root.join("qa");
        let manifest = root.join("corpus.tsv");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(case_dir.join("db")).unwrap();
        let source_path = "/evidence/a.mp4";
        fs::write(
            case_dir.join("db/videos.jsonl"),
            format!(
                "{{\"source_path\":\"{}\",\"sha256\":\"abc\"}}\n",
                source_path
            ),
        )
        .unwrap();
        fs::write(&manifest, format!("{source_path}\tabc\n")).unwrap();

        let report = accuracy_report(&case_dir, &manifest, &output_dir).unwrap();
        assert!(report.passed);
        assert!(report.report_path.is_file());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn accuracy_report_includes_recovery_artifacts() {
        let root = std::env::temp_dir().join(format!(
            "frametrace-qa-recovery-test-{}",
            std::process::id()
        ));
        let case_dir = root.join("case");
        let output_dir = root.join("qa");
        let manifest = root.join("corpus.tsv");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(case_dir.join("db")).unwrap();
        fs::create_dir_all(case_dir.join("artifacts/carved")).unwrap();
        fs::create_dir_all(case_dir.join("evidence/logs")).unwrap();
        fs::write(case_dir.join("db/videos.jsonl"), "").unwrap();
        let carved_path = root.join("case/artifacts/carved/carve_000001.mp4");
        fs::write(
            case_dir.join("artifacts/carved/carve-log.jsonl"),
            format!(
                "{{\"id\":\"carve_000001\",\"output_path\":\"{}\",\"sha256\":\"abc\",\"validation_status\":\"candidate-unvalidated\"}}\n",
                carved_path.display()
            ),
        )
        .unwrap();
        fs::write(
            case_dir.join("evidence/logs/validation-log.jsonl"),
            format!(
                "{{\"selector\":\"carve_000001\",\"target_path\":\"{}\",\"target_sha256\":\"abc\",\"validation_status\":\"ffprobe-video-stream-confirmed\"}}\n",
                carved_path.display()
            ),
        )
        .unwrap();
        fs::write(&manifest, format!("{}\tabc\n", carved_path.display())).unwrap();

        let report = accuracy_report(&case_dir, &manifest, &output_dir).unwrap();
        assert!(report.passed);
        assert!(report.report_path.is_file());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn report_defense_rejects_disallowed_legal_claims() {
        let root = std::env::temp_dir().join(format!(
            "frametrace-report-claims-test-{}",
            std::process::id()
        ));
        let case_dir = root.join("case");
        let output_dir = root.join("qa");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(case_dir.join("db")).unwrap();
        fs::create_dir_all(case_dir.join("reports")).unwrap();
        fs::create_dir_all(case_dir.join("review")).unwrap();
        fs::write(case_dir.join("case.json"), "{}").unwrap();
        fs::write(case_dir.join("db/case.db"), "").unwrap();
        fs::write(case_dir.join("db/video_index.json"), "{}").unwrap();
        fs::write(case_dir.join("db/videos.jsonl"), "").unwrap();
        fs::write(case_dir.join("db/video_paths.tsv"), "id\tsource_path\n").unwrap();
        fs::write(
            case_dir.join("reports/case-report.html"),
            "<html>court-ready recovery</html>",
        )
        .unwrap();
        fs::write(
            case_dir.join("review/evidence-viewer.html"),
            "<html></html>",
        )
        .unwrap();

        let err = report_defense_check(&case_dir, &output_dir).unwrap_err();
        assert!(err.contains("disallowed claim"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reproducibility_normalizes_case_paths_timestamps_and_package_times() {
        let root = std::env::temp_dir().join(format!(
            "frametrace-repro-normalized-test-{}",
            std::process::id()
        ));
        let left = root.join("left-case");
        let right = root.join("right-case");
        let output_dir = root.join("qa");
        let _ = fs::remove_dir_all(&root);
        seed_repro_case(&left, 111, "ffprobe-video-stream-confirmed", "abc");
        seed_repro_case(&right, 999, "ffprobe-video-stream-confirmed", "abc");

        let report = reproducibility_report(&left, &right, &output_dir).unwrap();
        assert!(report.passed);
        assert!(report.report_path.is_file());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reproducibility_detects_recovery_validation_drift() {
        let root = std::env::temp_dir().join(format!(
            "frametrace-repro-drift-test-{}",
            std::process::id()
        ));
        let left = root.join("left-case");
        let right = root.join("right-case");
        let output_dir = root.join("qa");
        let _ = fs::remove_dir_all(&root);
        seed_repro_case(&left, 111, "ffprobe-video-stream-confirmed", "abc");
        seed_repro_case(&right, 999, "validation-failed", "abc");

        let err = reproducibility_report(&left, &right, &output_dir).unwrap_err();
        assert!(err.contains("normalized core outputs differ"));

        let _ = fs::remove_dir_all(root);
    }

    fn seed_repro_case(
        case_dir: &std::path::Path,
        timestamp: u64,
        validation_status: &str,
        artifact_hash: &str,
    ) {
        fs::create_dir_all(case_dir.join("db/filesystem")).unwrap();
        fs::create_dir_all(case_dir.join("artifacts/carved")).unwrap();
        fs::create_dir_all(case_dir.join("evidence/logs")).unwrap();
        fs::create_dir_all(case_dir.join("reports/package")).unwrap();
        let artifact = case_dir.join("artifacts/carved/carve_000001.mp4");
        fs::write(
            case_dir.join("db/videos.jsonl"),
            format!(
                "{{\"id\":\"video_000001\",\"source_path\":\"/evidence/source.mp4\",\"sha256\":\"sourcehash\",\"first_indexed_unix\":{},\"last_indexed_unix\":{}}}\n",
                timestamp,
                timestamp + 1
            ),
        )
        .unwrap();
        fs::write(
            case_dir.join("db/video_paths.tsv"),
            "id\tsource_path\nvideo_000001\t/evidence/source.mp4\n",
        )
        .unwrap();
        fs::write(
            case_dir.join("artifacts/carved/carve-log.jsonl"),
            format!(
                "{{\"id\":\"carve_000001\",\"output_path\":\"{}\",\"sha256\":\"{}\",\"validation_status\":\"candidate-unvalidated\",\"carved_unix\":{},\"previous_entry_sha256\":\"left\",\"entry_sha256\":\"right\"}}\n",
                artifact.display(),
                artifact_hash,
                timestamp
            ),
        )
        .unwrap();
        fs::write(
            case_dir.join("evidence/logs/validation-log.jsonl"),
            format!(
                "{{\"selector\":\"carve_000001\",\"target_path\":\"{}\",\"target_sha256\":\"{}\",\"validation_status\":\"{}\",\"validated_unix\":{}}}\n",
                artifact.display(),
                artifact_hash,
                validation_status,
                timestamp + 2
            ),
        )
        .unwrap();
        fs::write(
            case_dir.join("db/filesystem/tsk-files-123.jsonl"),
            format!(
                "{{\"path\":\"{}\",\"inode\":\"42\",\"deleted\":true,\"inspected_unix\":{}}}\n",
                artifact.display(),
                timestamp + 3
            ),
        )
        .unwrap();
        fs::write(
            case_dir.join("reports/package/package-manifest.json"),
            format!(
                "{{\"schema_version\":1,\"created_unix\":{},\"files\":[{{\"relative_path\":\"artifacts/carved/carve_000001.mp4\",\"sha256\":\"{}\"}}]}}\n",
                timestamp + 4,
                artifact_hash
            ),
        )
        .unwrap();
        fs::write(
            case_dir.join("reports/package/manifest.sha256"),
            format!("{artifact_hash}  artifacts/carved/carve_000001.mp4\n"),
        )
        .unwrap();
    }

    #[test]
    fn review_manifest_accepts_key_value_and_markdown_gates() {
        let root =
            std::env::temp_dir().join(format!("frametrace-review-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let manifest = root.join("release-review.txt");
        fs::write(
            &manifest,
            "\
technical_review=pass
security review: approved
- [x] Migration Validation
- [x] Operator Review
legal-review=done
",
        )
        .unwrap();

        let gates = read_review_manifest(&manifest).unwrap();
        assert_eq!(gates.get("technical_review"), Some(&true));
        assert_eq!(gates.get("security_review"), Some(&true));
        assert_eq!(gates.get("migration_validation"), Some(&true));
        assert_eq!(gates.get("operator_review"), Some(&true));
        assert_eq!(gates.get("legal_review"), Some(&true));

        let _ = fs::remove_dir_all(root);
    }
}
