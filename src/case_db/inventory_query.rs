use super::inventory_types::{
    INVENTORY_MAX_PAGE_SIZE, InventoryListQuery, InventoryPage, InventoryRow,
};
use super::{case_db_path, open_readonly_case_db, table_exists};
use rusqlite::{Connection, OptionalExtension, Row, params, params_from_iter, types::Value};
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) const INVENTORY_COLUMNS: &str = "id, source_path, relative_path, extension, size_bytes, \
modified_unix, sha256, hash_status, source_profile_json, ffprobe_ok, last_indexed_unix, \
last_scanned_unix";

pub(crate) fn open_inventory_db(case_dir: &Path) -> Result<Option<Connection>, String> {
    let path = case_db_path(case_dir);
    if !path.is_file() {
        return Ok(None);
    }
    let conn = open_readonly_case_db(&path)?;
    if !table_exists(&conn, "videos")? {
        return Ok(None);
    }
    Ok(Some(conn))
}

pub(crate) fn inventory_filters(
    query: &InventoryListQuery,
) -> Result<(String, Vec<Value>), String> {
    let mut clauses = Vec::new();
    let mut params = Vec::new();
    if let Some(extension) = query
        .extension
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        clauses.push("extension = ?");
        params.push(Value::Text(extension.to_string()));
    }
    if let Some(state) = query
        .validation_state
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        match state {
            "candidate-unvalidated" => {
                clauses.push("ffprobe_ok = ?");
                params.push(Value::Integer(0));
            }
            "ffprobe-video-stream-confirmed" => {
                clauses.push("ffprobe_ok = ?");
                params.push(Value::Integer(1));
            }
            _ => return Err(format!("unsupported inventory validation state: {state}")),
        }
    }
    let where_sql = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    Ok((where_sql, params))
}

pub(crate) fn count_matching(
    conn: &Connection,
    where_sql: &str,
    params: &[Value],
) -> Result<usize, String> {
    let sql = format!("SELECT COUNT(*) FROM videos {where_sql}");
    scalar_count(conn, &sql, params_from_iter(params.iter()))
}

pub(crate) fn query_rows(
    conn: &Connection,
    sql: &str,
    params: Vec<Value>,
) -> Result<Vec<InventoryRow>, String> {
    let mut stmt = conn
        .prepare(sql)
        .map_err(|err| format!("failed to prepare inventory query: {err}"))?;
    let rows = stmt
        .query_map(params_from_iter(params.iter()), map_inventory_row)
        .map_err(|err| format!("failed to query inventory rows: {err}"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|err| format!("failed to read inventory row: {err}"))?);
    }
    Ok(out)
}

pub(crate) fn map_inventory_row(row: &Row<'_>) -> rusqlite::Result<InventoryRow> {
    let file_id: String = row.get(0)?;
    let source_path: String = row.get(1)?;
    let relative_path: String = row.get(2)?;
    let extension: String = row.get(3)?;
    let size_bytes = nonnegative_i64(row.get(4)?);
    let modified_unix = row.get::<_, Option<i64>>(5)?.map(nonnegative_i64);
    let sha256: Option<String> = row.get(6)?;
    let hash_status: String = row.get(7)?;
    let source_profile_json: String = row.get(8)?;
    let ffprobe_ok: bool = row.get(9)?;
    let last_indexed_unix = nonnegative_i64(row.get(10)?);
    let last_scanned_unix = row.get::<_, Option<i64>>(11)?.map(nonnegative_i64);
    Ok(InventoryRow {
        file_id,
        source_id: source_path.clone(),
        source_label: source_path.clone(),
        type_label: "video".to_string(),
        parser_lane: parser_lane(&source_profile_json),
        validation_state: validation_state(ffprobe_ok),
        review_state: "unreviewed".to_string(),
        report_state: "not-selected".to_string(),
        display_name: display_name(&relative_path),
        relative_path,
        full_path: source_path,
        extension,
        timestamp_start: modified_unix,
        timestamp_source: timestamp_source(modified_unix),
        size_bytes,
        hash_state: hash_status,
        sha256,
        inode: None,
        byte_offset: None,
        partition_offset: None,
        parent_artifact_id: None,
        duplicate_of: None,
        last_action_unix: last_scanned_unix.unwrap_or(last_indexed_unix),
    })
}

pub(crate) fn existing_ids(
    case_dir: &Path,
    file_ids: &[String],
) -> Result<BTreeSet<String>, String> {
    let Some(conn) = open_inventory_db(case_dir)? else {
        return Ok(BTreeSet::new());
    };
    let mut found = BTreeSet::new();
    for file_id in file_ids {
        let id = conn
            .query_row(
                "SELECT id FROM videos WHERE id = ?1",
                params![file_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|err| format!("failed to query bulk preview id {file_id}: {err}"))?;
        if let Some(id) = id {
            found.insert(id);
        }
    }
    Ok(found)
}

pub(crate) fn capped_page_size(page_size: usize) -> usize {
    page_size.min(INVENTORY_MAX_PAGE_SIZE)
}

pub(crate) fn empty_page(page_offset: usize, page_size: usize) -> InventoryPage {
    InventoryPage {
        total_rows: 0,
        rows: Vec::new(),
        page_offset,
        page_size,
    }
}

pub(crate) fn scalar_count<P: rusqlite::Params>(
    conn: &Connection,
    sql: &str,
    params: P,
) -> Result<usize, String> {
    let count: i64 = conn
        .query_row(sql, params, |row| row.get(0))
        .map_err(|err| format!("failed inventory count query: {err}"))?;
    Ok(nonnegative_count(count))
}

pub(crate) fn nonnegative_count(value: i64) -> usize {
    value.max(0) as usize
}

pub(crate) fn prefix_upper_bound(prefix: &str) -> String {
    format!("{prefix}\u{10ffff}")
}

fn nonnegative_i64(value: i64) -> u64 {
    value.max(0) as u64
}

fn validation_state(ffprobe_ok: bool) -> String {
    if ffprobe_ok {
        "ffprobe-video-stream-confirmed"
    } else {
        "candidate-unvalidated"
    }
    .to_string()
}

fn timestamp_source(timestamp: Option<u64>) -> String {
    timestamp
        .map(|_| "filesystem-modified-unix")
        .unwrap_or("not-available")
        .to_string()
}

fn display_name(relative_path: &str) -> String {
    relative_path
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(relative_path)
        .to_string()
}

fn parser_lane(source_profile_json: &str) -> String {
    if source_profile_json.contains("\"parser\":\"benchmark\"") {
        "benchmark".to_string()
    } else {
        "video-index".to_string()
    }
}
