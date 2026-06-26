use super::types::IndexedEvidence;
use crate::util::read_to_string;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

pub(super) fn read_indexed_evidence(case_dir: &Path) -> Result<Vec<IndexedEvidence>, String> {
    let mut indexed = HashMap::<String, IndexedEvidence>::new();
    read_video_index(case_dir, &mut indexed)?;
    read_optional_logs(case_dir, &mut indexed)?;

    let mut out = indexed.into_values().collect::<Vec<_>>();
    out.sort_by(|left, right| left.source_path.cmp(&right.source_path));
    Ok(out)
}

fn read_video_index(
    case_dir: &Path,
    indexed: &mut HashMap<String, IndexedEvidence>,
) -> Result<(), String> {
    let path = case_dir.join("db/videos.jsonl");
    let text = read_to_string(&path)
        .map_err(|err| format!("failed to read indexed evidence {}: {err}", path.display()))?;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let record = serde_json::from_str::<VideoIndexRecord>(line).map_err(|err| {
            format!(
                "failed to parse indexed evidence {} as JSONL: {err}",
                path.display()
            )
        })?;
        insert_indexed_evidence(indexed, Some(record.source_path), record.sha256);
    }
    Ok(())
}

fn read_optional_logs(
    case_dir: &Path,
    indexed: &mut HashMap<String, IndexedEvidence>,
) -> Result<(), String> {
    for rel_log in [
        "artifacts/carved/carve-log.jsonl",
        "evidence/logs/tsk-audit.jsonl",
        "evidence/logs/validation-log.jsonl",
    ] {
        let log_path = case_dir.join(rel_log);
        if !log_path.is_file() {
            continue;
        }
        let text = read_to_string(&log_path).map_err(|err| {
            format!(
                "failed to read indexed evidence {}: {err}",
                log_path.display()
            )
        })?;
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let record = serde_json::from_str::<EvidenceLogRecord>(line).map_err(|err| {
                format!(
                    "failed to parse indexed evidence {} as JSONL: {err}",
                    log_path.display()
                )
            })?;
            insert_indexed_evidence(
                indexed,
                record.output_path.or(record.target_path),
                record.sha256.or(record.target_sha256),
            );
        }
    }
    Ok(())
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

#[derive(Debug, Deserialize)]
struct VideoIndexRecord {
    source_path: String,
    sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EvidenceLogRecord {
    output_path: Option<String>,
    target_path: Option<String>,
    sha256: Option<String>,
    target_sha256: Option<String>,
}
