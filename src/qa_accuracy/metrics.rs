use super::types::{AccuracyMetrics, ExpectedEvidence, IndexedEvidence};
use std::collections::{HashMap, HashSet};

pub(super) fn compute_accuracy_metrics(
    expected: &[ExpectedEvidence],
    indexed: &[IndexedEvidence],
    precision_target: f64,
    recall_target: f64,
) -> AccuracyMetrics {
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
    for item in expected {
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
    let precision = if predicted_positive == 0 {
        1.0
    } else {
        true_positive as f64 / predicted_positive as f64
    };
    let recall = if expected.is_empty() {
        1.0
    } else {
        true_positive as f64 / expected.len() as f64
    };

    AccuracyMetrics {
        passed: precision >= precision_target && recall >= recall_target && hash_mismatch == 0,
        precision,
        recall,
        true_positive,
        false_positive,
        false_negative,
        hash_mismatch,
    }
}
