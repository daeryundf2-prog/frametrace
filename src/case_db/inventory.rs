use super::inventory_query::{
    INVENTORY_COLUMNS, capped_page_size, count_matching, empty_page, existing_ids,
    inventory_filters, map_inventory_row, nonnegative_count, open_inventory_db, prefix_upper_bound,
    query_rows, scalar_count,
};
use super::inventory_types::{
    BulkPreview, BulkPreviewRequest, InventoryFacet, InventoryFacetCounts, InventoryListQuery,
    InventoryPage, InventoryRow,
};
use crate::audit::json_string_array;
use crate::util::{json_escape, now_unix};
use rusqlite::{OptionalExtension, params, types::Value};
use std::path::Path;

pub fn list_inventory(
    case_dir: &Path,
    query: &InventoryListQuery,
) -> Result<InventoryPage, String> {
    let page_size = capped_page_size(query.page_size);
    let Some(conn) = open_inventory_db(case_dir)? else {
        return Ok(empty_page(query.page_offset, page_size));
    };
    let (where_sql, filter_params) = inventory_filters(query)?;
    let total_rows = count_matching(&conn, &where_sql, &filter_params)?;
    let sql = format!(
        "SELECT {INVENTORY_COLUMNS} FROM videos {where_sql} \
         ORDER BY ffprobe_ok ASC, modified_unix ASC, id ASC LIMIT ? OFFSET ?"
    );
    let mut params = filter_params;
    params.push(Value::Integer(page_size as i64));
    params.push(Value::Integer(query.page_offset as i64));
    let rows = query_rows(&conn, &sql, params)?;
    Ok(InventoryPage {
        total_rows,
        rows,
        page_offset: query.page_offset,
        page_size,
    })
}

pub fn search_inventory(
    case_dir: &Path,
    query_text: &str,
    page_size: usize,
) -> Result<InventoryPage, String> {
    let page_size = capped_page_size(page_size);
    let needle = query_text.trim();
    if needle.is_empty() {
        return list_inventory(
            case_dir,
            &InventoryListQuery {
                page_size,
                ..InventoryListQuery::default()
            },
        );
    }
    let Some(conn) = open_inventory_db(case_dir)? else {
        return Ok(empty_page(0, page_size));
    };
    let upper = prefix_upper_bound(needle);
    let where_sql = "WHERE id = ? OR sha256 = ? OR source_path = ? \
        OR (relative_path >= ? AND relative_path < ?)";
    let base_params = vec![
        Value::Text(needle.to_string()),
        Value::Text(needle.to_string()),
        Value::Text(needle.to_string()),
        Value::Text(needle.to_string()),
        Value::Text(upper),
    ];
    let total_rows = count_matching(&conn, where_sql, &base_params)?;
    let sql = format!(
        "SELECT {INVENTORY_COLUMNS} FROM videos {where_sql} \
         ORDER BY relative_path ASC, id ASC LIMIT ?"
    );
    let mut params = base_params;
    params.push(Value::Integer(page_size as i64));
    let rows = query_rows(&conn, &sql, params)?;
    Ok(InventoryPage {
        total_rows,
        rows,
        page_offset: 0,
        page_size,
    })
}

pub fn inventory_facets(case_dir: &Path) -> Result<InventoryFacetCounts, String> {
    let Some(conn) = open_inventory_db(case_dir)? else {
        return Ok(InventoryFacetCounts {
            total_rows: 0,
            confirmed_count: 0,
            candidate_count: 0,
            by_extension: Vec::new(),
        });
    };
    let total_rows = scalar_count(&conn, "SELECT COUNT(*) FROM videos", [])?;
    let confirmed_count = scalar_count(
        &conn,
        "SELECT COUNT(*) FROM videos WHERE ffprobe_ok = 1",
        [],
    )?;
    let candidate_count = scalar_count(
        &conn,
        "SELECT COUNT(*) FROM videos WHERE ffprobe_ok = 0",
        [],
    )?;
    let mut stmt = conn
        .prepare("SELECT extension, COUNT(*) FROM videos GROUP BY extension ORDER BY COUNT(*) DESC")
        .map_err(|err| format!("failed to prepare inventory facet query: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(InventoryFacet {
                value: row.get(0)?,
                count: nonnegative_count(row.get::<_, i64>(1)?),
            })
        })
        .map_err(|err| format!("failed to query inventory facets: {err}"))?;
    let mut by_extension = Vec::new();
    for row in rows {
        by_extension.push(row.map_err(|err| format!("failed to read inventory facet: {err}"))?);
    }
    Ok(InventoryFacetCounts {
        total_rows,
        confirmed_count,
        candidate_count,
        by_extension,
    })
}

pub fn get_file_detail(case_dir: &Path, file_id: &str) -> Result<Option<InventoryRow>, String> {
    let Some(conn) = open_inventory_db(case_dir)? else {
        return Ok(None);
    };
    let sql = format!("SELECT {INVENTORY_COLUMNS} FROM videos WHERE id = ?1");
    conn.query_row(&sql, params![file_id], map_inventory_row)
        .optional()
        .map_err(|err| format!("failed to query inventory detail: {err}"))
}

pub fn bulk_preview(case_dir: &Path, request: &BulkPreviewRequest) -> Result<BulkPreview, String> {
    let existing = existing_ids(case_dir, &request.file_ids)?;
    let missing_ids = request
        .file_ids
        .iter()
        .filter(|file_id| !existing.contains(*file_id))
        .cloned()
        .collect::<Vec<_>>();
    let selected_count = request.file_ids.len().saturating_sub(missing_ids.len());
    let filters_json = request
        .filters_json
        .as_deref()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .unwrap_or_else(|| "null".to_string());
    let audit_event_json = format!(
        "{{\"schema_version\":1,\"event\":\"bulk-preview\",\"created_unix\":{},\
\"operator\":\"{}\",\"action\":\"{}\",\"requested_count\":{},\"selected_count\":{},\
\"missing_ids\":{},\"mutation_committed\":false,\"filters_json\":{filters_json}}}",
        now_unix()?,
        json_escape(&request.operator),
        json_escape(&request.action),
        request.file_ids.len(),
        selected_count,
        json_string_array(&missing_ids)
    );
    Ok(BulkPreview {
        selected_count,
        missing_ids,
        audit_event_json,
    })
}
