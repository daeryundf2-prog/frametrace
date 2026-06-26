#[path = "qa_accuracy/contract.rs"]
mod contract;
#[path = "qa_accuracy/indexed.rs"]
mod indexed;
#[path = "qa_accuracy/manifest.rs"]
mod manifest;
#[path = "qa_accuracy/metrics.rs"]
mod metrics;
#[path = "qa_accuracy/report.rs"]
mod report;
#[path = "qa_accuracy/schema.rs"]
mod schema;
#[path = "qa_accuracy/types.rs"]
mod types;

use crate::qa::QaReport;
use crate::util::write_text;
use std::fs;
use std::path::Path;

const PRECISION_TARGET: f64 = 0.98;
const RECALL_TARGET: f64 = 0.98;

pub fn accuracy_report(
    case_dir: &Path,
    corpus_manifest: &Path,
    output_dir: &Path,
) -> Result<QaReport, String> {
    let manifest = manifest::read_expected_manifest(corpus_manifest)?;
    let indexed = indexed::read_indexed_evidence(case_dir)?;
    let metrics = metrics::compute_accuracy_metrics(
        &manifest.expected,
        &indexed,
        PRECISION_TARGET,
        RECALL_TARGET,
    );

    fs::create_dir_all(output_dir)
        .map_err(|err| format!("failed to create QA output directory: {err}"))?;
    let json_path = output_dir.join("accuracy-report.json");
    let html_path = output_dir.join("accuracy-report.html");
    write_text(
        &json_path,
        &report::accuracy_json(report::AccuracyJsonInput {
            metrics: &metrics,
            expected: &manifest.expected,
            indexed: &indexed,
            domains: &manifest.domains,
            release_keys: &manifest.release_keys,
            external_references: &manifest.external_references,
        })?,
    )
    .map_err(|err| format!("failed to write accuracy JSON report: {err}"))?;
    write_text(
        &html_path,
        &report::accuracy_html(
            "FrameTrace Accuracy QA",
            &format!(
                "passed={} precision={:.6} recall={:.6} tp={} fp={} fn={} hash_mismatch={}",
                metrics.passed,
                metrics.precision,
                metrics.recall,
                metrics.true_positive,
                metrics.false_positive,
                metrics.false_negative,
                metrics.hash_mismatch
            ),
        ),
    )
    .map_err(|err| format!("failed to write accuracy HTML report: {err}"))?;

    if metrics.passed {
        Ok(QaReport {
            report_path: json_path,
            passed: true,
        })
    } else {
        Err(format!(
            "accuracy QA failed: precision={:.6}, recall={:.6}, false_positive={}, false_negative={}, hash_mismatch={}",
            metrics.precision,
            metrics.recall,
            metrics.false_positive,
            metrics.false_negative,
            metrics.hash_mismatch
        ))
    }
}
