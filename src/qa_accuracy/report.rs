use super::types::{
    AccuracyMetrics, DomainSummary, ExpectedEvidence, ExternalReferenceSummary, IndexedEvidence,
};
use crate::util::html_escape;
use serde_json::{Map, Value, json};

pub(super) struct AccuracyJsonInput<'a> {
    pub(super) metrics: &'a AccuracyMetrics,
    pub(super) expected: &'a [ExpectedEvidence],
    pub(super) indexed: &'a [IndexedEvidence],
    pub(super) domains: &'a [DomainSummary],
    pub(super) release_keys: &'a Map<String, Value>,
    pub(super) external_references: &'a [ExternalReferenceSummary],
}

pub(super) fn accuracy_json(input: AccuracyJsonInput<'_>) -> Result<String, String> {
    let expected_json = input
        .expected
        .iter()
        .map(|item| {
            json!({
                "case_id": item.case_id,
                "domain": item.domain,
                "source_path": item.source_path,
                "source_sha256": item.sha256,
                "ground_truth": item.ground_truth,
                "expected_outputs": item.expected_outputs
            })
        })
        .collect::<Vec<_>>();
    let domains_json = input
        .domains
        .iter()
        .map(|domain| {
            json!({
                "key": domain.key,
                "status": domain.status,
                "reason": domain.reason
            })
        })
        .collect::<Vec<_>>();
    let external_references_json = input
        .external_references
        .iter()
        .map(|reference| {
            json!({
                "corpus_id": reference.corpus_id,
                "description": reference.description,
                "sha256": reference.sha256,
                "hash_only": reference.hash_only
            })
        })
        .collect::<Vec<_>>();
    let metrics = input.metrics;
    let value = json!({
        "schema_version": 1,
        "qa_type": "accuracy",
        "passed": metrics.passed,
        "precision": metrics.precision,
        "recall": metrics.recall,
        "true_positive": metrics.true_positive,
        "false_positive": metrics.false_positive,
        "false_negative": metrics.false_negative,
        "false_positives": metrics.false_positive,
        "false_negatives": metrics.false_negative,
        "hash_mismatch": metrics.hash_mismatch,
        "expected_count": input.expected.len(),
        "indexed_count": input.indexed.len(),
        "expected": expected_json,
        "domains": domains_json,
        "release_keys": input.release_keys,
        "external_references": external_references_json
    });
    serde_json::to_string_pretty(&value)
        .map(|mut text| {
            text.push('\n');
            text
        })
        .map_err(|err| format!("failed to render accuracy JSON report: {err}"))
}

pub(super) fn accuracy_html(title: &str, body: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title></head><body><h1>{}</h1><pre>{}</pre></body></html>\n",
        html_escape(title),
        html_escape(title),
        html_escape(body)
    )
}
