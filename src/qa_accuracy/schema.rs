use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TypedCorpusManifest {
    pub(super) schema_version: u64,
    pub(super) corpus_id: String,
    pub(super) corpus_kind: CorpusKind,
    pub(super) release_keys: Map<String, Value>,
    pub(super) domains: Vec<TypedCorpusDomain>,
    pub(super) cases: Vec<TypedCorpusCase>,
    pub(super) external_references: Vec<TypedExternalReference>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum CorpusKind {
    Synthetic,
    MixedRealWorldLike,
    ExternalHashOnly,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TypedCorpusDomain {
    pub(super) key: String,
    pub(super) status: DomainStatus,
    #[serde(default)]
    pub(super) reason: Option<String>,
    #[serde(default)]
    pub(super) ground_truth_schema: Vec<String>,
    #[serde(default)]
    pub(super) expected_outputs_schema: Vec<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum DomainStatus {
    Supported,
    Unsupported,
}

impl DomainStatus {
    pub(super) const fn as_str(&self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TypedCorpusCase {
    pub(super) case_id: String,
    pub(super) domain: String,
    pub(super) source_path: String,
    pub(super) source_sha256: String,
    pub(super) ground_truth: CorpusGroundTruth,
    pub(super) expected_outputs: Value,
}

#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CorpusGroundTruth {
    pub(super) corpus_id: String,
    pub(super) source_artifact_id: String,
    pub(super) source_sha256: String,
    pub(super) expected_artifact_type: String,
    pub(super) expected_path_pattern: String,
    pub(super) expected_hash: String,
    pub(super) expected_timestamp_range: TimestampRange,
    pub(super) expected_state: String,
    pub(super) negative_controls: Vec<String>,
    pub(super) notes: String,
}

#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TimestampRange {
    pub(super) start_unix: u64,
    pub(super) end_unix: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TypedExternalReference {
    pub(super) corpus_id: String,
    pub(super) description: String,
    pub(super) sha256: String,
    pub(super) hash_only: bool,
}

pub(super) fn matches_release_key_pass(value: &Value) -> bool {
    match value {
        Value::String(text) => matches!(
            text.trim().to_ascii_lowercase().as_str(),
            "pass" | "passed" | "supported" | "true"
        ),
        Value::Bool(flag) => *flag,
        _ => false,
    }
}
