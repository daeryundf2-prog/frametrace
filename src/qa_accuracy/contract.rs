use super::schema::TypedCorpusCase;
use super::types::REQUIRED_GROUND_TRUTH_FIELDS;
use std::path::Path;

pub(super) fn require_exact_schema(
    path: &Path,
    corpus_id: &str,
    domain_key: &str,
    fields: &[String],
) -> Result<(), String> {
    let actual = fields.iter().map(String::as_str).collect::<Vec<_>>();
    if actual == REQUIRED_GROUND_TRUTH_FIELDS {
        Ok(())
    } else {
        Err(format!(
            "corpus manifest {} corpus {} supported domain {} requires exact ground_truth_schema fields {:?}",
            path.display(),
            corpus_id,
            domain_key,
            REQUIRED_GROUND_TRUTH_FIELDS
        ))
    }
}

pub(super) fn require_non_empty_schema(
    path: &Path,
    corpus_id: &str,
    domain_key: &str,
    schema_name: &str,
    fields: &[String],
) -> Result<(), String> {
    if fields.is_empty() {
        Err(format!(
            "corpus manifest {} corpus {} supported domain {} requires non-empty {}",
            path.display(),
            corpus_id,
            domain_key,
            schema_name
        ))
    } else {
        Ok(())
    }
}

pub(super) fn require_case_ground_truth(
    path: &Path,
    corpus_id: &str,
    case: &TypedCorpusCase,
) -> Result<(), String> {
    if case.ground_truth.corpus_id != corpus_id {
        return Err(format!(
            "corpus case {} in {} has ground_truth corpus_id `{}`; expected `{corpus_id}`",
            case.case_id,
            path.display(),
            case.ground_truth.corpus_id
        ));
    }
    if case.ground_truth.source_sha256 != case.source_sha256 {
        return Err(format!(
            "corpus case {} source_sha256 does not match ground_truth.source_sha256",
            case.case_id
        ));
    }
    require_non_empty_ground_truth_fields(case)?;
    if case.ground_truth.expected_hash != case.source_sha256 {
        return Err(format!(
            "corpus case {} source_sha256 does not match expected_hash",
            case.case_id
        ));
    }
    if case.ground_truth.expected_timestamp_range.start_unix
        > case.ground_truth.expected_timestamp_range.end_unix
    {
        return Err(format!(
            "corpus case {} expected_timestamp_range start_unix is after end_unix",
            case.case_id
        ));
    }
    if case.ground_truth.negative_controls.is_empty() {
        return Err(format!(
            "corpus case {} requires at least one negative_controls entry",
            case.case_id
        ));
    }
    if case.ground_truth.notes.trim().is_empty() {
        return Err(format!("corpus case {} requires notes", case.case_id));
    }
    Ok(())
}

fn require_non_empty_ground_truth_fields(case: &TypedCorpusCase) -> Result<(), String> {
    for (field, value) in [
        (
            "source_artifact_id",
            case.ground_truth.source_artifact_id.as_str(),
        ),
        (
            "expected_artifact_type",
            case.ground_truth.expected_artifact_type.as_str(),
        ),
        (
            "expected_path_pattern",
            case.ground_truth.expected_path_pattern.as_str(),
        ),
        ("expected_state", case.ground_truth.expected_state.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("corpus case {} requires {field}", case.case_id));
        }
    }
    Ok(())
}
