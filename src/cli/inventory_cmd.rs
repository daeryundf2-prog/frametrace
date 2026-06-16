use crate::audit::json_string_array;
use crate::case_db::{
    BulkPreviewRequest, InventoryFacetCounts, InventoryListQuery, InventoryPage, InventoryRow,
    bulk_preview, get_file_detail, inventory_facets, list_inventory, search_inventory,
};
use crate::util::json_escape;
use std::path::Path;

use super::commands::Commands;

#[derive(Debug, Clone)]
pub struct InventoryCliOptions {
    pub search: Option<String>,
    pub file_id: Option<String>,
    pub facets: bool,
    pub offset: usize,
    pub limit: usize,
    pub extension: Option<String>,
    pub validation_state: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BulkPreviewCliOptions {
    pub action: String,
    pub operator: String,
    pub filters_json: Option<String>,
    pub file_ids: Vec<String>,
}

pub fn run_inventory_command(command: Commands) -> Result<(), String> {
    match command {
        Commands::Inventory {
            case_dir,
            search,
            file_id,
            facets,
            offset,
            limit,
            extension,
            validation_state,
        } => run_inventory(
            &case_dir,
            InventoryCliOptions {
                search,
                file_id,
                facets,
                offset,
                limit,
                extension,
                validation_state,
            },
        ),
        Commands::InventoryBulkPreview {
            case_dir,
            action,
            operator,
            filters_json,
            file_ids,
        } => run_inventory_bulk_preview(
            &case_dir,
            BulkPreviewCliOptions {
                action,
                operator,
                filters_json,
                file_ids,
            },
        ),
        _ => Err("not an inventory command".to_string()),
    }
}

pub fn run_inventory(case_dir: &Path, options: InventoryCliOptions) -> Result<(), String> {
    ensure_case(case_dir)?;
    if let Some(file_id) = options.file_id {
        let detail = get_file_detail(case_dir, &file_id)?;
        println!("{}", detail_json(&file_id, detail.as_ref()));
        return Ok(());
    }
    let facets = inventory_facets(case_dir)?;
    if options.facets {
        println!(
            "{{\"schema_version\":1,\"view\":\"facets\",\"facets\":{}}}",
            facets_json(&facets)
        );
        return Ok(());
    }
    let page = match options.search.as_deref() {
        Some(query) => search_inventory(case_dir, query, options.limit)?,
        None => list_inventory(
            case_dir,
            &InventoryListQuery {
                page_offset: options.offset,
                page_size: options.limit,
                extension: options.extension,
                validation_state: options.validation_state,
            },
        )?,
    };
    println!("{}", page_json(options.search.as_deref(), &page, &facets));
    Ok(())
}

pub fn run_inventory_bulk_preview(
    case_dir: &Path,
    options: BulkPreviewCliOptions,
) -> Result<(), String> {
    ensure_case(case_dir)?;
    let preview = bulk_preview(
        case_dir,
        &BulkPreviewRequest {
            file_ids: options.file_ids,
            action: options.action,
            operator: options.operator,
            filters_json: options.filters_json,
        },
    )?;
    println!(
        "{{\"schema_version\":1,\"view\":\"bulk-preview\",\
\"selected_count\":{},\"missing_ids\":{},\"audit_event\":{}}}",
        preview.selected_count,
        json_string_array(&preview.missing_ids),
        preview.audit_event_json
    );
    Ok(())
}

fn ensure_case(case_dir: &Path) -> Result<(), String> {
    if case_dir.join("case.json").is_file() {
        Ok(())
    } else {
        Err(format!(
            "not a FrameTrace case (missing case.json): {}",
            case_dir.display()
        ))
    }
}

fn page_json(query: Option<&str>, page: &InventoryPage, facets: &InventoryFacetCounts) -> String {
    format!(
        "{{\"schema_version\":1,\"view\":\"inventory\",\"query\":{},\
\"page_offset\":{},\"page_size\":{},\"total_rows\":{},\"facets\":{},\"rows\":[{}]}}",
        optional_json_string(query),
        page.page_offset,
        page.page_size,
        page.total_rows,
        facets_json(facets),
        page.rows.iter().map(row_json).collect::<Vec<_>>().join(",")
    )
}

fn detail_json(file_id: &str, row: Option<&InventoryRow>) -> String {
    let row_json = row.map(row_json).unwrap_or_else(|| "null".to_string());
    format!(
        "{{\"schema_version\":1,\"view\":\"detail\",\"file_id\":\"{}\",\"row\":{row_json}}}",
        json_escape(file_id)
    )
}

fn facets_json(facets: &InventoryFacetCounts) -> String {
    format!(
        "{{\"total_rows\":{},\"candidate_count\":{},\"confirmed_count\":{},\
\"by_extension\":[{}]}}",
        facets.total_rows,
        facets.candidate_count,
        facets.confirmed_count,
        facets
            .by_extension
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
