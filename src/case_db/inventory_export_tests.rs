use super::{ExportManifestRequest, benchmark_case_db, export_manifest, get_file_detail};
use rusqlite::Connection;
use std::fs;

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

#[test]
fn export_manifest_rejects_outputs_outside_case_or_over_existing_files() {
    let root = test_root("frametrace-inventory-export-policy-test");
    benchmark_case_db(&root, 1).unwrap();

    let outside = root
        .parent()
        .unwrap_or_else(|| std::path::Path::new("/tmp"))
        .join(format!(
            "frametrace-inventory-export-outside-{}.json",
            std::process::id()
        ));
    let outside_err = export_manifest(
        &root,
        &ExportManifestRequest {
            file_ids: vec!["bench_00000000".to_string()],
            operator: "qa".to_string(),
            filters_json: None,
            output_path: Some(outside),
        },
    )
    .unwrap_err();
    assert!(outside_err.contains("inside the case directory"));

    let existing = root.join("reports/existing-inventory-export.json");
    fs::create_dir_all(existing.parent().unwrap()).unwrap();
    fs::write(&existing, b"existing").unwrap();
    let existing_err = export_manifest(
        &root,
        &ExportManifestRequest {
            file_ids: vec!["bench_00000000".to_string()],
            operator: "qa".to_string(),
            filters_json: None,
            output_path: Some(existing),
        },
    )
    .unwrap_err();
    assert!(existing_err.contains("output already exists"));

    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn export_manifest_rejects_dangling_symlink_output_without_creating_target() {
    let root = test_root("frametrace-inventory-symlink-output-policy-test");
    benchmark_case_db(&root, 1).unwrap();

    let outside_target = root
        .parent()
        .unwrap_or_else(|| std::path::Path::new("/tmp"))
        .join(format!(
            "frametrace-inventory-export-dangling-target-{}.json",
            std::process::id()
        ));
    let output_path = root.join("reports/dangling-inventory-export.json");
    let _ = fs::remove_file(&outside_target);
    fs::create_dir_all(output_path.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&outside_target, &output_path).unwrap();

    let err = export_manifest(
        &root,
        &ExportManifestRequest {
            file_ids: vec!["bench_00000000".to_string()],
            operator: "qa".to_string(),
            filters_json: None,
            output_path: Some(output_path),
        },
    )
    .unwrap_err();

    assert!(err.contains("cannot be a symlink"));
    assert!(!outside_target.exists());

    let _ = fs::remove_file(outside_target);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn export_manifest_rejects_registered_source_evidence_output() {
    let root = test_root("frametrace-inventory-source-output-policy-test");
    benchmark_case_db(&root, 1).unwrap();
    let source_path = root.join("source-media/source.mp4");
    fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    fs::write(&source_path, b"source evidence").unwrap();
    let conn = Connection::open(root.join("db/case.db")).unwrap();
    conn.execute(
        "UPDATE videos SET source_path = ?1 WHERE id = ?2",
        rusqlite::params![source_path.to_string_lossy(), "bench_00000000"],
    )
    .unwrap();
    let source_path = get_file_detail(&root, "bench_00000000")
        .unwrap()
        .unwrap()
        .full_path;

    let err = export_manifest(
        &root,
        &ExportManifestRequest {
            file_ids: vec!["bench_00000000".to_string()],
            operator: "qa".to_string(),
            filters_json: None,
            output_path: Some(std::path::PathBuf::from(source_path)),
        },
    )
    .unwrap_err();

    assert!(err.contains("registered source evidence path"));

    let _ = fs::remove_dir_all(root);
}

fn test_root(prefix: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("{prefix}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    root
}
