use crate::case_db;

pub fn winui_contract_json() -> String {
    "{\"durable_mutation\":\"engine-command-only\",\
\"state_owner\":\"rust-engine-sqlite-audit\",\
\"inventory_transport\":\"paged-sqlite-query\",\
\"large_case_full_json_load_allowed\":false,\
\"candidate_promotion\":\"validate-artifact then confirm-playback\",\
\"release_language\":\"report-defensible\"}"
        .to_string()
}

pub fn gui_data_adapter_json() -> String {
    format!(
        "{{\"schema_version\":1,\"state_owner\":\"rust-engine-sqlite-audit\",\
\"gui_durable_state_allowed\":false,\"full_json_load_allowed\":false,\"max_page_size\":{},\
\"surfaces\":{{\"case_open\":{{\"command\":\"workstation-status\",\"response_view\":\"workstation-status\"}},\
\"inventory_page\":{{\"command\":\"inventory\",\"response_view\":\"inventory\"}},\
\"inventory_search\":{{\"command\":\"inventory\",\"response_view\":\"inventory\"}},\
\"inventory_facets\":{{\"command\":\"inventory --facets\",\"response_view\":\"facets\"}},\
\"inventory_detail\":{{\"command\":\"inventory --file-id\",\"response_view\":\"detail\"}},\
\"source_tree\":{{\"command\":\"inventory --facets\",\"response_view\":\"facets\"}},\
\"bulk_preview\":{{\"command\":\"inventory-bulk-preview\",\"response_view\":\"bulk-preview\"}},\
\"export_manifest\":{{\"command\":\"inventory-export-manifest\",\"response_view\":\"inventory-export-manifest\"}},\
\"validation_playback_state\":{{\"command\":\"workstation-status\",\"response_path\":\"validation\"}},\
\"report_package_status\":{{\"command\":\"workstation-status\",\"response_path\":\"generated_artifacts\"}}}}}}",
        case_db::INVENTORY_MAX_PAGE_SIZE
    )
}
