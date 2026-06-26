use super::contract::{require_case_ground_truth, require_exact_schema, require_non_empty_schema};
use super::schema::{CorpusKind, DomainStatus, TypedCorpusManifest, matches_release_key_pass};
use super::types::{CorpusManifest, DomainSummary, ExpectedEvidence, ExternalReferenceSummary};
use crate::util::read_to_string;
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::path::Path;

pub(super) fn read_expected_manifest(path: &Path) -> Result<CorpusManifest, String> {
    let text = read_to_string(path)
        .map_err(|err| format!("failed to read corpus manifest {}: {err}", path.display()))?;
    if text.trim_start().starts_with('{') {
        return read_typed_corpus_manifest(path, &text);
    }
    read_legacy_tsv_manifest(path, &text)
}

fn read_legacy_tsv_manifest(path: &Path, text: &str) -> Result<CorpusManifest, String> {
    let mut expected = Vec::new();
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
        expected.push(ExpectedEvidence {
            source_path: columns[0].trim().to_string(),
            sha256: columns
                .get(1)
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            case_id: None,
            domain: None,
            ground_truth: Value::Null,
            expected_outputs: Value::Null,
        });
    }
    Ok(CorpusManifest {
        expected,
        domains: Vec::new(),
        external_references: Vec::new(),
        release_keys: Map::new(),
    })
}

fn read_typed_corpus_manifest(path: &Path, text: &str) -> Result<CorpusManifest, String> {
    let raw = serde_json::from_str::<TypedCorpusManifest>(text).map_err(|err| {
        format!(
            "corpus manifest {} must be typed JSON with required ground truth fields: {err}",
            path.display()
        )
    })?;
    validate_manifest_header(path, &raw)?;
    let (domains, supported_domains) = validate_domains(path, &raw)?;
    let expected = validate_cases(path, &raw, &supported_domains)?;
    let external_references = raw
        .external_references
        .into_iter()
        .map(|reference| ExternalReferenceSummary {
            corpus_id: reference.corpus_id,
            description: reference.description,
            sha256: reference.sha256,
            hash_only: reference.hash_only,
        })
        .collect();

    Ok(CorpusManifest {
        expected,
        domains,
        external_references,
        release_keys: raw.release_keys,
    })
}

fn validate_manifest_header(path: &Path, raw: &TypedCorpusManifest) -> Result<(), String> {
    if raw.schema_version != 1 {
        return Err(format!(
            "corpus manifest {} has unsupported schema_version {}; expected 1",
            path.display(),
            raw.schema_version
        ));
    }
    if raw.corpus_id.trim().is_empty() {
        return Err(format!(
            "corpus manifest {} requires non-empty corpus_id",
            path.display()
        ));
    }
    if raw.corpus_kind == CorpusKind::Synthetic
        && raw
            .release_keys
            .get("mixed_real_world_like")
            .is_some_and(matches_release_key_pass)
    {
        return Err(
            "corpus manifest cannot satisfy mixed_real_world_like with synthetic-only evidence"
                .to_string(),
        );
    }
    Ok(())
}

fn validate_domains(
    path: &Path,
    raw: &TypedCorpusManifest,
) -> Result<(Vec<DomainSummary>, HashSet<String>), String> {
    let mut domains = Vec::with_capacity(raw.domains.len());
    let mut supported_domains = HashSet::new();
    for domain in &raw.domains {
        let status = domain.status.as_str();
        if domain.status == DomainStatus::Supported {
            require_exact_schema(
                path,
                &raw.corpus_id,
                &domain.key,
                &domain.ground_truth_schema,
            )?;
            require_non_empty_schema(
                path,
                &raw.corpus_id,
                &domain.key,
                "expected_outputs_schema",
                &domain.expected_outputs_schema,
            )?;
            supported_domains.insert(domain.key.clone());
        }
        domains.push(DomainSummary {
            key: domain.key.clone(),
            status: status.to_string(),
            reason: domain.reason.clone(),
        });
    }
    Ok((domains, supported_domains))
}

fn validate_cases(
    path: &Path,
    raw: &TypedCorpusManifest,
    supported_domains: &HashSet<String>,
) -> Result<Vec<ExpectedEvidence>, String> {
    let mut expected = Vec::with_capacity(raw.cases.len());
    for case in &raw.cases {
        if !supported_domains.contains(case.domain.as_str()) {
            return Err(format!(
                "corpus case {} belongs to unsupported domain {}; unsupported domains must be recorded as unsupported, not passed",
                case.case_id, case.domain
            ));
        }
        require_case_ground_truth(path, &raw.corpus_id, case)?;
        expected.push(ExpectedEvidence {
            source_path: case.source_path.clone(),
            sha256: Some(case.ground_truth.expected_hash.clone()),
            case_id: Some(case.case_id.clone()),
            domain: Some(case.domain.clone()),
            ground_truth: serde_json::to_value(&case.ground_truth).map_err(|err| {
                format!(
                    "failed to render corpus case {} ground truth: {err}",
                    case.case_id
                )
            })?,
            expected_outputs: case.expected_outputs.clone(),
        });
    }
    Ok(expected)
}
