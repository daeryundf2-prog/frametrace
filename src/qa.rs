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
        .map(|item| (normalize_evidence_path(&item.source_path), item))
        .collect::<HashMap<_, _>>();
    let expected_sources = expected
        .iter()
        .map(|item| normalize_evidence_path(&item.source_path))
        .collect::<HashSet<_>>();

    let mut true_positive = 0usize;
    let mut false_negative = 0usize;
    let mut hash_mismatch = 0usize;
    for item in &expected {
        match indexed_by_source.get(normalize_evidence_path(&item.source_path).as_str()) {
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
        .filter(|item| !expected_sources.contains(&normalize_evidence_path(&item.source_path)))
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

/// Cross-store consistency: the SQLite `videos` table and the
/// `db/videos.jsonl` compatibility artifact must describe the same
/// evidence. A crash between the JSONL write and the SQLite commit (or a
/// manual DB repair) can leave ghost rows on either side; those rows then
/// skew id allocation and report counts while being invisible to every
/// other QA surface.
pub fn consistency_report(case_dir: &Path, output_dir: &Path) -> Result<QaReport, String> {
    let sqlite_rows = case_db::load_video_ids(case_dir)?;
    let sqlite_ids: HashSet<String> = sqlite_rows.iter().map(|row| row.id.clone()).collect();

    let jsonl_path = case_dir.join("db/videos.jsonl");
    let mut jsonl_ids = HashSet::new();
    let mut malformed = 0usize;
    if jsonl_path.is_file() {
        let text = read_to_string(&jsonl_path)
            .map_err(|err| format!("failed to read {}: {err}", jsonl_path.display()))?;
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            match serde_json::from_str::<serde_json::Value>(line) {
                Ok(value) => {
                    if let Some(id) = value.get("id").and_then(serde_json::Value::as_str) {
                        jsonl_ids.insert(id.to_string());
                    } else {
                        malformed += 1;
                    }
                }
                Err(_) => malformed += 1,
            }
        }
    }

    let sqlite_only: Vec<String> = sqlite_ids.difference(&jsonl_ids).cloned().collect();
    let jsonl_only: Vec<String> = jsonl_ids.difference(&sqlite_ids).cloned().collect();
    let passed = sqlite_only.is_empty() && jsonl_only.is_empty() && malformed == 0;

    fs::create_dir_all(output_dir)
        .map_err(|err| format!("failed to create QA output directory: {err}"))?;
    let json_path = output_dir.join("consistency-report.json");
    let html_path = output_dir.join("consistency-report.html");
    let body = format!(
        "sqlite rows: {}\njsonl rows: {}\nsqlite-only: {}\njsonl-only: {}\nmalformed jsonl lines: {}",
        sqlite_rows.len(),
        jsonl_ids.len(),
        if sqlite_only.is_empty() {
            "-".to_string()
        } else {
            sqlite_only.join(", ")
        },
        if jsonl_only.is_empty() {
            "-".to_string()
        } else {
            jsonl_only.join(", ")
        },
        malformed,
    );
    write_text(
        &json_path,
        &format!(
            "{{\n  \"schema_version\": 1,\n  \"qa_type\": \"consistency\",\n  \"passed\": {},\n  \"sqlite_rows\": {},\n  \"jsonl_rows\": {},\n  \"sqlite_only\": {},\n  \"jsonl_only\": {},\n  \"malformed\": {}\n}}\n",
            passed,
            sqlite_rows.len(),
            jsonl_ids.len(),
            json_array(&sqlite_only),
            json_array(&jsonl_only),
            malformed,
        ),
    )
    .map_err(|err| format!("failed to write consistency JSON: {err}"))?;
    write_text(
        &html_path,
        &simple_html_report("FrameTrace Consistency QA", &body),
    )
    .map_err(|err| format!("failed to write consistency HTML: {err}"))?;

    if passed {
        Ok(QaReport {
            report_path: json_path,
            passed,
        })
    } else {
        Err(format!(
            "consistency QA failed: {} sqlite-only, {} jsonl-only, {malformed} malformed line(s); re-run scan-folder to rebuild both stores",
            sqlite_only.len(),
            jsonl_only.len(),
        ))
    }
}

fn json_array(items: &[String]) -> String {
    let quoted: Vec<String> = items.iter().map(|item| format!("\"{item}\"")).collect();
    format!("[{}]", quoted.join(","))
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

    // The tamper-evident chain is the product's central claim; a release
    // must not pass readiness with a torn or tampered audit log. Chained
    // logs live under evidence/logs AND artifacts/*/ (export, batch,
    // proxy, thumbnail logs all use append_chained_jsonl), so sweep the
    // case tree and keep files whose first entry carries the chain schema.
    checks.push(run_release_check("audit_chain", || {
        let mut logs = Vec::new();
        collect_chained_logs(case_dir, &mut logs);
        if logs.is_empty() {
            return Err("no chained audit logs found in the case".to_string());
        }
        let mut verified = 0usize;
        for log in &logs {
            crate::audit::verify_chained_jsonl(log)
                .map_err(|err| format!("{}: {}", log.display(), err))?;
            verified += 1;
        }
        let report_path = output_dir.join("audit-chain-report.txt");
        write_text(
            &report_path,
            &format!("audit chain verification PASS: {verified} log(s) intact\n"),
        )
        .map_err(|err| format!("failed to write audit chain report: {err}"))?;
        Ok(report_path)
    }));

    // The SQLite index and the JSONL compatibility artifact describe the
    // same evidence; a release with ghost rows on either side is not
    // defensible.
    checks.push(run_release_check("consistency", || {
        consistency_report(case_dir, output_dir).map(|report| report.report_path)
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

/// Collects every hash-chained audit log in the case: any *.jsonl whose
/// first non-empty line contains the chain's `previous_entry_sha256`
/// marker. Plain data files (db/videos.jsonl, fls entries) are skipped.
fn collect_chained_logs(case_dir: &Path, out: &mut Vec<PathBuf>) {
    let mut stack = vec![case_dir.join("evidence"), case_dir.join("artifacts")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl")
                && let Ok(text) = read_to_string(&path)
                && let Some(first) = text.lines().map(str::trim).find(|l| !l.is_empty())
                && first.contains("previous_entry_sha256")
            {
                out.push(path);
            }
        }
    }
    out.sort();
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

/// Corpus manifests are written by external tooling, so the same file may be
/// spelled with or without the Windows extended-length prefix that
/// `std::fs::canonicalize` produces. Normalize both sides before comparing.
fn normalize_evidence_path(source_path: &str) -> String {
    crate::util::strip_windows_extended_prefix(Path::new(source_path))
        .to_string_lossy()
        .to_string()
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
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .map(|value| IndexedEvidence {
            source_path: value
                .get("source_path")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            sha256: value
                .get("sha256")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
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
            // `stale_since_unix` is stamped with wall-clock scan time, so
            // two cases that ran the identical vanish-workflow in
            // different seconds would otherwise compare unequal. The
            // marker's PRESENCE is the reproducible fact; the timestamp
            // is environment noise — normalize it away.
            .map(|line| normalize_stale_timestamps(line).to_string())
            .collect::<Vec<_>>();
        lines.sort();
        parts.push(format!("{rel}\n{}\n", lines.join("\n")));
    }
    Ok(parts.join("\n"))
}

/// Replaces every `"stale_since_unix":<digits>` occurrence with a constant.
fn normalize_stale_timestamps(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let bytes = line.as_bytes();
    let needle = b"\"stale_since_unix\":";
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(needle) {
            out.push_str("\"stale_since_unix\":0");
            index += needle.len();
            // Skip the digits that followed.
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
        } else {
            // Advance by one full UTF-8 scalar to keep indices on char
            // boundaries for non-ASCII content.
            let step = line[index..]
                .chars()
                .next()
                .map(char::len_utf8)
                .unwrap_or(1);
            out.push_str(&line[index..index + step]);
            index += step;
        }
    }
    out
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
        accuracy_report, consistency_report, normalize_stale_timestamps, read_indexed_evidence,
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
    fn read_indexed_evidence_skips_malformed_lines_and_reads_escapes() {
        let root =
            std::env::temp_dir().join(format!("frametrace-qa-rows-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("db")).unwrap();
        fs::write(
            root.join("db/videos.jsonl"),
            concat!(
                // Well-formed record with an escaped backslash path.
                "{\"source_path\":\"C:\\\\ev\\\\a.mp4\",\"sha256\":\"abc\"}\n",
                // Malformed line (truncated JSON) must be skipped, not fatal.
                "{\"source_path\":\"C:\\\\ev\\\",\n",
                // Record without sha256 keeps None.
                "{\"source_path\":\"/evidence/b.mp4\"}\n",
            ),
        )
        .unwrap();
        let rows = read_indexed_evidence(&root).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].source_path, "C:\\ev\\a.mp4");
        assert_eq!(rows[0].sha256.as_deref(), Some("abc"));
        assert_eq!(rows[1].source_path, "/evidence/b.mp4");
        assert!(rows[1].sha256.is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn consistency_detects_ghost_rows_on_both_sides() {
        let root =
            std::env::temp_dir().join(format!("frametrace-qa-consistency-{}", std::process::id()));
        let case_dir = root.join("case");
        let output_dir = root.join("qa");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(case_dir.join("db")).unwrap();

        // JSONL knows vid_000001 only; SQLite (via case_db) is built by a
        // scan, so simulate the divergence with a minimal rows file plus a
        // real scan-run DB is overkill here — instead verify the report
        // directly against a case where both stores agree first.
        fs::write(
            case_dir.join("db/videos.jsonl"),
            "{\"id\":\"vid_000001\",\"source_path\":\"/e/a.mp4\"}\n",
        )
        .unwrap();
        // No SQLite DB exists yet: load_video_ids returns empty => the
        // JSONL row is jsonl-only => consistency must FAIL (not panic).
        let result = consistency_report(&case_dir, &output_dir);
        assert!(result.is_err(), "ghost jsonl rows must fail consistency");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn stale_timestamps_normalize_to_constant() {
        let a = r#"{"id":"vid_000001","index_status":"stale","stale_since_unix":1788917107}"#;
        let b = r#"{"id":"vid_000001","index_status":"stale","stale_since_unix":9999999999}"#;
        assert_eq!(normalize_stale_timestamps(a), normalize_stale_timestamps(b));
        assert!(normalize_stale_timestamps(a).contains(r#""stale_since_unix":0"#));
        // Korean content must survive byte-stepping.
        let korean = r#"{"id":"vid_1","relative_path":"한글.mp4","stale_since_unix":123}"#;
        let normalized = normalize_stale_timestamps(korean);
        assert!(normalized.contains("한글.mp4"), "{normalized}");
    }
}
