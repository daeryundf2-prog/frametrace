use crate::case_db::{
    BulkPreviewRequest, ExportManifestRequest, InventoryListQuery, bulk_preview, export_manifest,
    get_file_detail, inventory_facets, list_inventory, search_inventory,
};
use std::path::{Path, PathBuf};

use super::commands::Commands;
use super::inventory_json::{
    bulk_preview_json, detail_json, export_manifest_json, facets_response_json, page_json,
};

#[derive(Debug, Clone)]
pub struct InventoryCliOptions {
    pub search: Option<String>,
    pub file_id: Option<String>,
    pub facets: bool,
    pub offset: usize,
    pub limit: usize,
    pub extension: Option<String>,
    pub validation_state: Option<String>,
    pub sort: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BulkPreviewCliOptions {
    pub action: String,
    pub operator: String,
    pub filters_json: Option<String>,
    pub file_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ExportManifestCliOptions {
    pub operator: String,
    pub filters_json: Option<String>,
    pub output: Option<PathBuf>,
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
            sort,
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
                sort,
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
        Commands::InventoryExportManifest {
            case_dir,
            operator,
            filters_json,
            output,
            file_ids,
        } => run_inventory_export_manifest(
            &case_dir,
            ExportManifestCliOptions {
                operator,
                filters_json,
                output,
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
        println!("{}", facets_response_json(&facets));
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
                sort: options.sort,
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
    println!("{}", bulk_preview_json(&preview));
    Ok(())
}

pub fn run_inventory_export_manifest(
    case_dir: &Path,
    options: ExportManifestCliOptions,
) -> Result<(), String> {
    ensure_case(case_dir)?;
    let manifest = export_manifest(
        case_dir,
        &ExportManifestRequest {
            file_ids: options.file_ids,
            operator: options.operator,
            filters_json: options.filters_json,
            output_path: options.output,
        },
    )?;
    println!("{}", export_manifest_json(&manifest));
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
