use crate::audit::json_string_array;
use crate::case_db::{
    BulkPreview, ExportManifestResult, InventoryFacet, InventoryFacetCounts, InventoryPage,
    InventoryRow,
};
use crate::util::json_escape;

pub(super) fn page_json(
    query: Option<&str>,
    page: &InventoryPage,
    facets: &InventoryFacetCounts,
) -> String {
    format!(
        "{{\"schema_version\":1,\"view\":\"inventory\",\"query\":{},\
\"page_offset\":{},\"page_size\":{},\"next_cursor\":{},\"query_id\":\"{}\",\
\"duration_ms\":{},\"total_rows\":{},\"truncated\":{},\"facets\":{},\"rows\":[{}]}}",
        optional_json_string(query),
        page.page_offset,
        page.page_size,
        optional_usize(page.next_cursor),
        json_escape(&page.query_id),
        page.duration_ms,
        page.total_rows,
        page.truncated,
        facets_json(facets),
        page.rows.iter().map(row_json).collect::<Vec<_>>().join(",")
    )
}

pub(super) fn detail_json(file_id: &str, row: Option<&InventoryRow>) -> String {
    let row_json = row.map(row_json).unwrap_or_else(|| "null".to_string());
    format!(
        "{{\"schema_version\":1,\"view\":\"detail\",\"file_id\":\"{}\",\"row\":{row_json}}}",
        json_escape(file_id)
    )
}

pub(super) fn facets_response_json(facets: &InventoryFacetCounts) -> String {
    format!(
        "{{\"schema_version\":1,\"view\":\"facets\",\"facets\":{}}}",
        facets_json(facets)
    )
}

pub(super) fn bulk_preview_json(preview: &BulkPreview) -> String {
    format!(
        "{{\"schema_version\":1,\"view\":\"bulk-preview\",\
\"preview_id\":\"{}\",\"selected_count\":{},\"missing_ids\":{},\"warnings\":{},\
\"expected_mutation\":\"{}\",\"audit_path\":\"{}\",\"audit_event\":{}}}",
        json_escape(&preview.preview_id),
        preview.selected_count,
        json_string_array(&preview.missing_ids),
        json_string_array(&preview.warnings),
        json_escape(&preview.expected_mutation),
        json_escape(&preview.audit_path),
        preview.audit_event_json
    )
}

pub(super) fn export_manifest_json(manifest: &ExportManifestResult) -> String {
    format!(
        "{{\"schema_version\":1,\"view\":\"inventory-export-manifest\",\
\"selected_count\":{},\"missing_ids\":{},\"output_path\":\"{}\",\
\"output_sha256\":\"{}\",\"audit_event\":{}}}",
        manifest.selected_count,
        json_string_array(&manifest.missing_ids),
        json_escape(&manifest.output_path.to_string_lossy()),
        json_escape(&manifest.output_sha256),
        manifest.audit_event_json
    )
}

fn facets_json(facets: &InventoryFacetCounts) -> String {
    format!(
        "{{\"total_rows\":{},\"candidate_count\":{},\"confirmed_count\":{},\
\"by_extension\":{},\"by_source\":{},\"by_type\":{},\"by_parser_lane\":{},\
\"by_validation_state\":{},\"by_review_state\":{},\"by_report_state\":{},\
\"by_hash_state\":{}}}",
        facets.total_rows,
        facets.candidate_count,
        facets.confirmed_count,
        facet_array_json(&facets.by_extension),
        facet_array_json(&facets.by_source),
        facet_array_json(&facets.by_type),
        facet_array_json(&facets.by_parser_lane),
        facet_array_json(&facets.by_validation_state),
        facet_array_json(&facets.by_review_state),
        facet_array_json(&facets.by_report_state),
        facet_array_json(&facets.by_hash_state)
    )
}

fn facet_array_json(facets: &[InventoryFacet]) -> String {
    format!(
        "[{}]",
        facets
            .iter()
            .map(|facet| format!(
                "{{\"value\":\"{}\",\"count\":{}}}",
                json_escape(&facet.value),
                facet.count
            ))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn row_json(row: &InventoryRow) -> String {
    format!(
        "{{\"file_id\":\"{}\",\"source_id\":\"{}\",\"source_label\":\"{}\",\
\"type_label\":\"{}\",\"parser_lane\":\"{}\",\"validation_state\":\"{}\",\
\"review_state\":\"{}\",\"report_state\":\"{}\",\"display_name\":\"{}\",\
\"relative_path\":\"{}\",\"source_path\":\"{}\",\"extension\":\"{}\",\
\"timestamp_start\":{},\"timestamp_source\":\"{}\",\"size_bytes\":{},\
\"hash_state\":\"{}\",\"sha256\":{},\"inode\":{},\"byte_offset\":{},\
\"partition_offset\":{},\"parent_artifact_id\":{},\"duplicate_of\":{},\
\"last_action_unix\":{}}}",
        json_escape(&row.file_id),
        json_escape(&row.source_id),
        json_escape(&row.source_label),
        json_escape(&row.type_label),
        json_escape(&row.parser_lane),
        json_escape(&row.validation_state),
        json_escape(&row.review_state),
        json_escape(&row.report_state),
        json_escape(&row.display_name),
        json_escape(&row.relative_path),
        json_escape(&row.full_path),
        json_escape(&row.extension),
        optional_u64(row.timestamp_start),
        json_escape(&row.timestamp_source),
        row.size_bytes,
        json_escape(&row.hash_state),
        optional_json_string(row.sha256.as_deref()),
        optional_json_string(row.inode.as_deref()),
        optional_u64(row.byte_offset),
        optional_u64(row.partition_offset),
        optional_json_string(row.parent_artifact_id.as_deref()),
        optional_json_string(row.duplicate_of.as_deref()),
        row.last_action_unix
    )
}

fn optional_json_string(value: Option<&str>) -> String {
    value
        .map(|inner| format!("\"{}\"", json_escape(inner)))
        .unwrap_or_else(|| "null".to_string())
}

fn optional_u64(value: Option<u64>) -> String {
    value
        .map(|inner| inner.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn optional_usize(value: Option<usize>) -> String {
    value
        .map(|inner| inner.to_string())
        .unwrap_or_else(|| "null".to_string())
}
