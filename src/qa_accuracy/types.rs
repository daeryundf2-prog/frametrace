use serde_json::{Map, Value};

#[derive(Debug, Clone)]
pub(super) struct ExpectedEvidence {
    pub(super) source_path: String,
    pub(super) sha256: Option<String>,
    pub(super) case_id: Option<String>,
    pub(super) domain: Option<String>,
    pub(super) ground_truth: Value,
    pub(super) expected_outputs: Value,
}

#[derive(Debug, Clone)]
pub(super) struct IndexedEvidence {
    pub(super) source_path: String,
    pub(super) sha256: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct CorpusManifest {
    pub(super) expected: Vec<ExpectedEvidence>,
    pub(super) domains: Vec<DomainSummary>,
    pub(super) external_references: Vec<ExternalReferenceSummary>,
    pub(super) release_keys: Map<String, Value>,
}

#[derive(Debug, Clone)]
pub(super) struct DomainSummary {
    pub(super) key: String,
    pub(super) status: String,
    pub(super) reason: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct ExternalReferenceSummary {
    pub(super) corpus_id: String,
    pub(super) description: String,
    pub(super) sha256: String,
    pub(super) hash_only: bool,
}

#[derive(Debug, Clone)]
pub(super) struct AccuracyMetrics {
    pub(super) passed: bool,
    pub(super) precision: f64,
    pub(super) recall: f64,
    pub(super) true_positive: usize,
    pub(super) false_positive: usize,
    pub(super) false_negative: usize,
    pub(super) hash_mismatch: usize,
}

pub(super) const REQUIRED_GROUND_TRUTH_FIELDS: &[&str] = &[
    "corpus_id",
    "source_artifact_id",
    "source_sha256",
    "expected_artifact_type",
    "expected_path_pattern",
    "expected_hash",
    "expected_timestamp_range",
    "expected_state",
    "negative_controls",
    "notes",
];
