use super::{
    BulkPreviewRequest, ExportManifestRequest, INVENTORY_MAX_PAGE_SIZE, InventoryListQuery,
    benchmark_case_db, bulk_preview, export_manifest, get_file_detail, inventory_facets,
    list_inventory, search_inventory,
};
use rusqlite::Connection;
use std::fs;

#[test]
fn lists_inventory_with_bounded_sqlite_pagination() {
    let root = test_root("frametrace-inventory-list-test");
    benchmark_case_db(&root, 64).unwrap();

    let page = list_inventory(
        &root,
        &InventoryListQuery {
            page_offset: 0,
            page_size: 10,
            extension: Some("mp4".to_string()),
            validation_state: Some("candidate-unvalidated".to_string()),
            sort: None,
        },
    )
    .unwrap();

    assert_eq!(page.total_rows, 64);
    assert_eq!(page.rows.len(), 10);
    assert_eq!(page.next_cursor, Some(10));
    assert!(page.truncated);
    assert!(page.query_id.starts_with("inventory-"));
    assert_eq!(page.rows[0].file_id, "bench_00000000");
    assert_eq!(page.rows[0].validation_state, "candidate-unvalidated");
    assert_eq!(page.rows[0].parser_lane, "benchmark");
    assert_eq!(page.rows[0].report_state, "not-selected");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn caps_inventory_page_size_for_large_viewer_safety() {
    let root = test_root("frametrace-inventory-cap-test");
    benchmark_case_db(&root, 600).unwrap();

    let page = list_inventory(
        &root,
        &InventoryListQuery {
            page_size: INVENTORY_MAX_PAGE_SIZE + 100,
            ..InventoryListQuery::default()
        },
    )
    .unwrap();

    assert_eq!(page.total_rows, 600);
    assert_eq!(page.page_size, INVENTORY_MAX_PAGE_SIZE);
    assert_eq!(page.rows.len(), INVENTORY_MAX_PAGE_SIZE);
    assert_eq!(page.next_cursor, Some(INVENTORY_MAX_PAGE_SIZE));
    assert!(page.truncated);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn inventory_sort_contract_is_explicit_and_stable() {
    let root = test_root("frametrace-inventory-sort-test");
    benchmark_case_db(&root, 8).unwrap();

    let newest = list_inventory(
        &root,
        &InventoryListQuery {
            page_size: 3,
            sort: Some("timestamp-desc".to_string()),
            ..InventoryListQuery::default()
        },
    )
    .unwrap();
    let largest = list_inventory(
        &root,
        &InventoryListQuery {
            page_size: 3,
            sort: Some("size-desc".to_string()),
            ..InventoryListQuery::default()
        },
    )
    .unwrap();
    let bad_sort = list_inventory(
        &root,
        &InventoryListQuery {
            sort: Some("unknown-sort".to_string()),
            ..InventoryListQuery::default()
        },
    )
    .unwrap_err();

    assert_eq!(newest.rows[0].file_id, "bench_00000007");
    assert_eq!(newest.rows[1].file_id, "bench_00000006");
    assert_eq!(largest.rows[0].file_id, "bench_00000007");
    assert_eq!(largest.rows[1].file_id, "bench_00000006");
    assert_eq!(newest.next_cursor, Some(3));
    assert!(bad_sort.contains("unsupported inventory sort"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn searches_facets_and_loads_detail_from_sqlite() {
    let root = test_root("frametrace-inventory-query-test");
    benchmark_case_db(&root, 64).unwrap();

    let search = search_inventory(&root, "clip_0000000", 20).unwrap();
    let facets = inventory_facets(&root).unwrap();
    let detail = get_file_detail(&root, "bench_00000003").unwrap().unwrap();

    assert_eq!(search.total_rows, 10);
    assert_eq!(search.rows.len(), 10);
    assert_eq!(facets.total_rows, 64);
    assert_eq!(facets.candidate_count, 64);
    assert_eq!(facets.confirmed_count, 0);
    assert_eq!(facets.by_extension[0].value, "mp4");
    assert_eq!(facets.by_extension[0].count, 64);
    assert_eq!(facets.by_type[0].value, "video");
    assert_eq!(facets.by_parser_lane[0].value, "benchmark");
    assert_eq!(facets.by_validation_state[0].value, "candidate-unvalidated");
    assert_eq!(facets.by_review_state[0].value, "unreviewed");
    assert_eq!(facets.by_report_state[0].value, "not-selected");
    assert_eq!(facets.by_hash_state[0].value, "benchmark");
    assert_eq!(detail.file_id, "bench_00000003");
    assert_eq!(detail.display_name, "clip_00000003.mp4");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn bulk_preview_is_auditable_and_does_not_mutate_database() {
    let root = test_root("frametrace-inventory-preview-test");
    benchmark_case_db(&root, 8).unwrap();
    let before = video_count(&root);

    let preview = bulk_preview(
        &root,
        &BulkPreviewRequest {
            file_ids: vec![
                "bench_00000000".to_string(),
                "bench_00000001".to_string(),
                "missing".to_string(),
            ],
            action: "exclude-from-report".to_string(),
            operator: "qa".to_string(),
            filters_json: Some("{\"extension\":\"mp4\"}".to_string()),
        },
    )
    .unwrap();
    let after = video_count(&root);

    assert_eq!(preview.selected_count, 2);
    assert!(preview.preview_id.starts_with("bulk-preview-"));
    assert_eq!(preview.expected_mutation, "report_state -> excluded");
    assert!(preview.audit_path.starts_with("evidence/logs/"));
    assert_eq!(preview.missing_ids, vec!["missing".to_string()]);
    assert_eq!(preview.warnings, vec!["1 requested file ID was not found"]);
    assert_eq!(before, after);
    assert!(
        preview
            .audit_event_json
            .contains("\"event\":\"bulk-preview\"")
    );
    assert!(
        preview
            .audit_event_json
            .contains("\"mutation_committed\":false")
    );
    assert!(preview.audit_event_json.contains("\"operator\":\"qa\""));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn export_manifest_writes_selected_rows_with_output_hash() {
    let root = test_root("frametrace-inventory-export-test");
    benchmark_case_db(&root, 8).unwrap();
    let output_path = root.join("reports/custom-inventory-export.json");

    let manifest = export_manifest(
        &root,
        &ExportManifestRequest {
            file_ids: vec![
                "bench_00000000".to_string(),
                "bench_00000001".to_string(),
                "missing".to_string(),
            ],
            operator: "qa".to_string(),
            filters_json: Some("{\"hash_state\":\"benchmark\"}".to_string()),
            output_path: Some(output_path.clone()),
        },
    )
    .unwrap();
    let text = fs::read_to_string(&output_path).unwrap();

    assert_eq!(manifest.selected_count, 2);
    assert_eq!(manifest.missing_ids, vec!["missing".to_string()]);
    assert_eq!(manifest.output_path, output_path);
    assert_eq!(manifest.output_sha256.len(), 64);
    assert!(text.contains("\"manifest_kind\":\"inventory-export\""));
    assert!(text.contains("\"source_of_truth\":\"case.db/videos\""));
    assert!(text.contains("\"browser_large_case_policy\":\"paged-query-only\""));
    assert!(text.contains("\"file_id\":\"bench_00000000\""));
    assert!(text.contains("\"missing_ids\":[\"missing\"]"));

    let _ = fs::remove_dir_all(root);
}

fn test_root(prefix: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("{prefix}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    root
}

fn video_count(case_dir: &std::path::Path) -> i64 {
    let db_path = case_dir.join("db/case.db");
    let conn = Connection::open(db_path).unwrap();
    conn.query_row("SELECT COUNT(*) FROM videos", [], |row| row.get(0))
        .unwrap()
}
