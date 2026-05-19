use super::*;
use crate::model::{ScanResult, VideoRecord};
use crate::util::{json_escape, now_unix, path_to_file_url};
use rusqlite::{Connection, OpenFlags, OptionalExtension, Transaction, params};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub fn register_evidence_source(
    case_dir: &Path,
    input: &EvidenceSourceInput,
) -> Result<EvidenceSourceRow, String> {
    let conn = open_case_db(case_dir)?;
    init_schema(&conn)?;
    let now = now_unix()?;
    let path = input.path.to_string_lossy().to_string();
    let source_id = input
        .source_id
        .clone()
        .unwrap_or_else(|| stable_source_id(&input.kind, &path));
    conn.execute(
        r#"
        INSERT INTO evidence_sources (
            source_id,
            kind,
            path,
            registered_unix,
            last_seen_unix,
            write_protect,
            acquisition_tool,
            evidence_hash,
            notes,
            metadata_json
        )
        VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(source_id) DO UPDATE SET
            kind = excluded.kind,
            path = excluded.path,
            last_seen_unix = excluded.last_seen_unix,
            write_protect = excluded.write_protect,
            acquisition_tool = excluded.acquisition_tool,
            evidence_hash = excluded.evidence_hash,
            notes = excluded.notes,
            metadata_json = excluded.metadata_json
        ON CONFLICT(kind, path) DO UPDATE SET
            last_seen_unix = excluded.last_seen_unix,
            write_protect = COALESCE(excluded.write_protect, evidence_sources.write_protect),
            acquisition_tool = COALESCE(excluded.acquisition_tool, evidence_sources.acquisition_tool),
            evidence_hash = COALESCE(excluded.evidence_hash, evidence_sources.evidence_hash),
            notes = COALESCE(excluded.notes, evidence_sources.notes),
            metadata_json = excluded.metadata_json
        "#,
        params![
            source_id.as_str(),
            input.kind.as_str(),
            path.as_str(),
            u64_to_i64(now),
            input.write_protect.as_deref(),
            input.acquisition_tool.as_deref(),
            input.evidence_hash.as_deref(),
            input.notes.as_deref(),
            input.metadata_json.as_deref().unwrap_or("{}"),
        ],
    )
    .map_err(|err| format!("failed to register evidence source: {err}"))?;

    conn.query_row(
        "SELECT source_id, kind, path FROM evidence_sources WHERE kind = ?1 AND path = ?2",
        params![input.kind.as_str(), path.as_str()],
        |row| {
            Ok(EvidenceSourceRow {
                source_id: row.get(0)?,
                kind: row.get(1)?,
                path: row.get(2)?,
            })
        },
    )
    .map_err(|err| format!("failed to read registered evidence source: {err}"))
}

pub(crate) fn stable_source_id(kind: &str, path: &str) -> String {
    let digest = crate::sha256::digest_bytes(format!("{kind}\n{path}").as_bytes());
    format!("src_{}", &digest[..16])
}
