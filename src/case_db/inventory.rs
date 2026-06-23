use super::inventory_query::{
    INVENTORY_COLUMNS, capped_page_size, count_matching, empty_page, existing_ids,
    inventory_filters, map_inventory_row, open_inventory_db, prefix_upper_bound, query_rows,
};
use super::inventory_types::{
    BulkPreview, BulkPreviewRequest, InventoryListQuery, InventoryPage, InventoryRow,
};
use crate::audit::json_string_array;
use crate::util::{compact_json_value_if_well_formed, json_escape, now_unix};
use rusqlite::{OptionalExtension, params, types::Value};
use std::path::Path;
use std::time::Instant;

pub fn list_inventory(
    case_dir: &Path,
    query: &InventoryListQuery,
) -> Result<InventoryPage, String> {
    let started = Instant::now();
    let page_size = capped_page_size(query.page_size);
    let Some(conn) = open_inventory_db(case_dir)? else {
        return Ok(empty_page(query.page_offset, page_size));
    };
    let (where_sql, filter_params) = inventory_filters(query)?;
    let total_rows = count_matching(&conn, &where_sql, &filter_params)?;
    let order_by = inventory_order_by(query.sort.as_deref())?;
    let sql = format!(
        "SELECT {INVENTORY_COLUMNS} FROM videos {where_sql} \
         {order_by} LIMIT ? OFFSET ?"
    );
    let mut params = filter_params;
    params.push(Value::Integer(page_size as i64));
    params.push(Value::Integer(query.page_offset as i64));
    let rows = query_rows(&conn, &sql, params)?;
    Ok(page_with_metadata(
        "inventory-list",
        total_rows,
        rows,
        query.page_offset,
        page_size,
        started.elapsed().as_millis(),
    ))
}

pub fn search_inventory(
    case_dir: &Path,
    query_text: &str,
    page_size: usize,
) -> Result<InventoryPage, String> {
    let started = Instant::now();
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
    Ok(page_with_metadata(
        "inventory-search",
        total_rows,
        rows,
        0,
        page_size,
        started.elapsed().as_millis(),
    ))
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
    let created_unix = now_unix()?;
    let existing = existing_ids(case_dir, &request.file_ids)?;
    let missing_ids = request
        .file_ids
        .iter()
        .filter(|file_id| !existing.contains(*file_id))
        .cloned()
        .collect::<Vec<_>>();
    let selected_count = request.file_ids.len().saturating_sub(missing_ids.len());
    let preview_id = format!("bulk-preview-{created_unix}-{}", request.action);
    let audit_path = format!("evidence/logs/{preview_id}.jsonl");
    let warnings = missing_warning(&missing_ids);
    let expected_mutation = expected_mutation(&request.action).to_string();
    let filters_json = filters_snapshot_json(request.filters_json.as_deref());
    let audit_event_json = format!(
        "{{\"schema_version\":1,\"event\":\"bulk-preview\",\"preview_id\":\"{}\",\
\"created_unix\":{},\
\"operator\":\"{}\",\"action\":\"{}\",\"requested_count\":{},\"selected_count\":{},\
\"missing_ids\":{},\"warnings\":{},\"expected_mutation\":\"{}\",\"audit_path\":\"{}\",\
\"mutation_committed\":false,\"filters_json\":{filters_json}}}",
        json_escape(&preview_id),
        created_unix,
        json_escape(&request.operator),
        json_escape(&request.action),
        request.file_ids.len(),
        selected_count,
        json_string_array(&missing_ids),
        json_string_array(&warnings),
        json_escape(&expected_mutation),
        json_escape(&audit_path)
    );
    Ok(BulkPreview {
        preview_id,
        selected_count,
        missing_ids,
        warnings,
        expected_mutation,
        audit_path,
        audit_event_json,
    })
}

fn page_with_metadata(
    prefix: &str,
    total_rows: usize,
    rows: Vec<InventoryRow>,
    page_offset: usize,
    page_size: usize,
    duration_ms: u128,
) -> InventoryPage {
    let next_offset = page_offset.saturating_add(rows.len());
    let next_cursor = (next_offset < total_rows).then_some(next_offset);
    InventoryPage {
        total_rows,
        rows,
        page_offset,
        page_size,
        next_cursor,
        query_id: format!("{prefix}-{page_offset}-{page_size}-{total_rows}"),
        duration_ms,
        truncated: next_cursor.is_some(),
    }
}

fn inventory_order_by(sort: Option<&str>) -> Result<&'static str, String> {
    match sort.unwrap_or("risk-timestamp-asc") {
        "risk-timestamp-asc" | "default" => {
            Ok("ORDER BY ffprobe_ok ASC, modified_unix ASC, id ASC")
        }
        "timestamp-desc" => Ok("ORDER BY modified_unix DESC, id ASC"),
        "path-asc" => Ok("ORDER BY relative_path ASC, id ASC"),
        "size-desc" => Ok("ORDER BY size_bytes DESC, id ASC"),
        sort => Err(format!("unsupported inventory sort: {sort}")),
    }
}

fn expected_mutation(action: &str) -> &'static str {
    match action {
        "mark-reviewed" => "review_state -> reviewed",
        "report-set" | "add-to-report" => "report_state -> included",
        "exclude-from-report" => "report_state -> excluded",
        "queue-validation" => "validation job queued",
        "queue-hash" => "hash job queued",
        "queue-proxy-thumbnail" => "proxy/thumbnail job queued",
        "export-manifest" => "inventory manifest export",
        _ => "action preview only",
    }
}

fn missing_warning(missing_ids: &[String]) -> Vec<String> {
    match missing_ids.len() {
        0 => Vec::new(),
        1 => vec!["1 requested file ID was not found".to_string()],
        count => vec![format!("{count} requested file IDs were not found")],
    }
}

fn filters_snapshot_json(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|inner| !inner.is_empty())
        .map(|inner| {
            compact_json_value_if_well_formed(inner)
                .unwrap_or_else(|| format!("\"{}\"", json_escape(inner)))
        })
        .unwrap_or_else(|| "null".to_string())
}
