pub const INVENTORY_MAX_PAGE_SIZE: usize = 500;

#[derive(Debug, Clone, Default)]
pub struct InventoryListQuery {
    pub page_offset: usize,
    pub page_size: usize,
    pub extension: Option<String>,
    pub validation_state: Option<String>,
    pub sort: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InventoryRow {
    pub file_id: String,
    pub source_id: String,
    pub source_label: String,
    pub type_label: String,
    pub parser_lane: String,
    pub validation_state: String,
    pub review_state: String,
    pub report_state: String,
    pub display_name: String,
    pub relative_path: String,
    pub full_path: String,
    pub extension: String,
    pub timestamp_start: Option<u64>,
    pub timestamp_source: String,
    pub size_bytes: u64,
    pub hash_state: String,
    pub sha256: Option<String>,
    pub inode: Option<String>,
    pub byte_offset: Option<u64>,
    pub partition_offset: Option<u64>,
    pub parent_artifact_id: Option<String>,
    pub duplicate_of: Option<String>,
    pub last_action_unix: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InventoryPage {
    pub total_rows: usize,
    pub rows: Vec<InventoryRow>,
    pub page_offset: usize,
    pub page_size: usize,
    pub next_cursor: Option<usize>,
    pub query_id: String,
    pub duration_ms: u128,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InventoryFacet {
    pub value: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InventoryFacetCounts {
    pub total_rows: usize,
    pub confirmed_count: usize,
    pub candidate_count: usize,
    pub by_extension: Vec<InventoryFacet>,
    pub by_source: Vec<InventoryFacet>,
    pub by_type: Vec<InventoryFacet>,
    pub by_parser_lane: Vec<InventoryFacet>,
    pub by_validation_state: Vec<InventoryFacet>,
    pub by_review_state: Vec<InventoryFacet>,
    pub by_report_state: Vec<InventoryFacet>,
    pub by_hash_state: Vec<InventoryFacet>,
}

#[derive(Debug, Clone)]
pub struct BulkPreviewRequest {
    pub file_ids: Vec<String>,
    pub action: String,
    pub operator: String,
    pub filters_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BulkPreview {
    pub preview_id: String,
    pub selected_count: usize,
    pub missing_ids: Vec<String>,
    pub warnings: Vec<String>,
    pub expected_mutation: String,
    pub audit_path: String,
    pub audit_event_json: String,
}

#[derive(Debug, Clone)]
pub struct ExportManifestRequest {
    pub file_ids: Vec<String>,
    pub operator: String,
    pub filters_json: Option<String>,
    pub output_path: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportManifestResult {
    pub selected_count: usize,
    pub missing_ids: Vec<String>,
    pub output_path: std::path::PathBuf,
    pub output_sha256: String,
    pub audit_event_json: String,
}
