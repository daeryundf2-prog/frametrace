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
    pub performance_output_dir: Option<PathBuf>,
    pub performance_rows: usize,
}

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
    let passed = missing.is_empty();
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
    write_text(&report_path, &text)
        .map_err(|err| format!("failed to write report-defense checklist: {err}"))?;
    if passed {
        Ok(QaReport {
            report_path,
            passed,
        })
    } else {
        Err(format!(
            "report defensibility QA failed: missing {} required artifacts",
            missing.len()
        ))
    }
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
        Err(format!(
            "release readiness failed: {blocker_count} blocker(s); see {}",
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
    let path = case_dir.join("db/videos.jsonl");
    let text = read_to_string(&path)
        .map_err(|err| format!("failed to read indexed evidence {}: {err}", path.display()))?;
    Ok(text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| IndexedEvidence {
            source_path: extract_json_string(line, "source_path").unwrap_or_default(),
            sha256: extract_json_string(line, "sha256"),
        })
        .collect())
}

fn normalized_case_core(case_dir: &Path) -> Result<String, String> {
    let mut parts = Vec::new();
    for rel in ["db/videos.jsonl", "db/video_paths.tsv"] {
        let path = case_dir.join(rel);
        let text = read_to_string(&path).map_err(|err| {
            format!(
                "failed to read reproducibility input {}: {err}",
                path.display()
            )
        })?;
        let mut lines = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        lines.sort();
        parts.push(format!("{rel}\n{}\n", lines.join("\n")));
    }
    Ok(parts.join("\n"))
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
    use super::accuracy_report;
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
}
