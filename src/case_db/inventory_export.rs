use super::inventory::get_file_detail;
use super::inventory_query::existing_ids;
use super::inventory_types::{ExportManifestRequest, ExportManifestResult, InventoryRow};
use crate::audit::{digest_file, json_string_array};
use crate::util::{
    compact_json_value_if_well_formed, json_escape, now_unix, unique_path, write_text,
};
use std::path::{Path, PathBuf};

pub fn export_manifest(
    case_dir: &Path,
    request: &ExportManifestRequest,
) -> Result<ExportManifestResult, String> {
    let created_unix = now_unix()?;
    let existing = existing_ids(case_dir, &request.file_ids)?;
    let missing_ids = request
        .file_ids
        .iter()
        .filter(|file_id| !existing.contains(*file_id))
        .cloned()
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    for file_id in request
        .file_ids
        .iter()
        .filter(|file_id| existing.contains(*file_id))
    {
        if let Some(row) = get_file_detail(case_dir, file_id)? {
            rows.push(row);
        }
    }

    let output_path = manifest_output_path(case_dir, created_unix, request.output_path.as_ref());
    let manifest_json = manifest_json(created_unix, request, &missing_ids, &rows);
    write_text(&output_path, &manifest_json).map_err(|err| {
        format!(
            "failed to write inventory manifest {}: {err}",
            output_path.display()
        )
    })?;
    let output_sha256 = digest_file(&output_path)?;
    let audit_event_json = format!(
        "{{\"schema_version\":1,\"event\":\"inventory-export-manifest\",\
\"created_unix\":{},\"operator\":\"{}\",\"selected_count\":{},\"missing_ids\":{},\
\"output_path\":\"{}\",\"output_sha256\":\"{}\",\"case_state_mutated\":false}}",
        created_unix,
        json_escape(&request.operator),
        rows.len(),
        json_string_array(&missing_ids),
        json_escape(&output_path.to_string_lossy()),
        json_escape(&output_sha256)
    );

    Ok(ExportManifestResult {
        selected_count: rows.len(),
        missing_ids,
        output_path,
        output_sha256,
        audit_event_json,
    })
}

fn manifest_output_path(
    case_dir: &Path,
    created_unix: u64,
    requested: Option<&PathBuf>,
) -> PathBuf {
    requested.cloned().unwrap_or_else(|| {
        unique_path(&case_dir.join(format!("reports/inventory-export-{created_unix}.json")))
    })
}

fn manifest_json(
    created_unix: u64,
    request: &ExportManifestRequest,
    missing_ids: &[String],
    rows: &[InventoryRow],
) -> String {
    format!(
        "{{\"schema_version\":1,\"manifest_kind\":\"inventory-export\",\
\"created_unix\":{},\"operator\":\"{}\",\"source_of_truth\":\"case.db/videos\",\
\"browser_large_case_policy\":\"paged-query-only\",\"selected_count\":{},\
\"requested_count\":{},\"missing_ids\":{},\"filters_json\":{},\
\"unsupported_or_partial\":[],\"rows\":[{}]}}",
        created_unix,
        json_escape(&request.operator),
        rows.len(),
        request.file_ids.len(),
        json_string_array(missing_ids),
        filters_snapshot_json(request.filters_json.as_deref()),
        rows.iter()
            .map(manifest_row_json)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn manifest_row_json(row: &InventoryRow) -> String {
    format!(
        "{{\"file_id\":\"{}\",\"source_id\":\"{}\",\"source_path\":\"{}\",\
\"relative_path\":\"{}\",\"display_name\":\"{}\",\"sha256\":{},\"hash_state\":\"{}\",\
\"validation_state\":\"{}\",\"review_state\":\"{}\",\"report_state\":\"{}\",\
\"parent_artifact_id\":{},\"duplicate_of\":{},\"last_action_unix\":{}}}",
        json_escape(&row.file_id),
        json_escape(&row.source_id),
        json_escape(&row.full_path),
        json_escape(&row.relative_path),
        json_escape(&row.display_name),
        optional_json_string(row.sha256.as_deref()),
        json_escape(&row.hash_state),
        json_escape(&row.validation_state),
        json_escape(&row.review_state),
        json_escape(&row.report_state),
        optional_json_string(row.parent_artifact_id.as_deref()),
        optional_json_string(row.duplicate_of.as_deref()),
        row.last_action_unix
    )
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

fn optional_json_string(value: Option<&str>) -> String {
    value
        .map(|inner| format!("\"{}\"", json_escape(inner)))
        .unwrap_or_else(|| "null".to_string())
}
