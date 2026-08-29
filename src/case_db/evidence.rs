use super::*;
use crate::util::now_unix;
use rusqlite::params;
use std::path::Path;

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

#[derive(Debug, Clone, PartialEq)]
pub struct ReviewMarkRow {
    pub record_id: String,
    pub status: String,
    pub marked_unix: u64,
    pub record_path: Option<String>,
    pub examiner: Option<String>,
}

/// Upserts examiner review marks imported from the viewer's marks file.
/// Returns the number of stored rows.
pub fn upsert_review_marks(case_dir: &Path, marks: &[ReviewMarkRow]) -> Result<usize, String> {
    let mut conn = open_case_db(case_dir)?;
    init_schema(&conn)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start SQLite marks transaction: {err}"))?;
    for mark in marks {
        tx.execute(
            r#"
            INSERT INTO review_marks (record_id, status, marked_unix, record_path, examiner)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(record_id) DO UPDATE SET
                status = excluded.status,
                marked_unix = excluded.marked_unix,
                record_path = COALESCE(excluded.record_path, review_marks.record_path),
                examiner = COALESCE(excluded.examiner, review_marks.examiner)
            "#,
            params![
                mark.record_id.as_str(),
                mark.status.as_str(),
                u64_to_i64(mark.marked_unix),
                mark.record_path.as_deref(),
                mark.examiner.as_deref(),
            ],
        )
        .map_err(|err| format!("failed to upsert review mark {}: {err}", mark.record_id))?;
    }
    tx.commit()
        .map_err(|err| format!("failed to commit review marks: {err}"))?;
    Ok(marks.len())
}

pub fn load_review_marks(case_dir: &Path) -> Result<Vec<ReviewMarkRow>, String> {
    let path = case_db_path(case_dir);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let conn = open_readonly_case_db(&path)?;
    if !table_exists(&conn, "review_marks")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare("SELECT record_id, status, marked_unix, record_path, examiner FROM review_marks ORDER BY record_id")
        .map_err(|err| format!("failed to prepare review marks query: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ReviewMarkRow {
                record_id: row.get(0)?,
                status: row.get(1)?,
                marked_unix: i64_to_u64(row.get(2)?),
                record_path: row.get(3)?,
                examiner: row.get(4)?,
            })
        })
        .map_err(|err| format!("failed to query review marks: {err}"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|err| format!("failed to read review mark row: {err}"))?);
    }
    Ok(out)
}
