use crate::qa::QaReport;
use crate::util::{html_escape, read_to_string, write_text};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

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
