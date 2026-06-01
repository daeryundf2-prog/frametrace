use super::benchmark_case_db;
use std::fs;

#[test]
fn benchmark_records_indexed_query_latency() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-db-benchmark-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);

    let result = benchmark_case_db(&root, 64).unwrap();

    assert_eq!(result.rows, 64);
    assert_eq!(result.query_count, 4);
    assert_eq!(result.query_rows_returned, 130);
    assert_eq!(result.query_plans.len(), 4);
    assert!(
        result
            .query_plans
            .iter()
            .all(|plan| !plan.detail.is_empty())
    );
    assert!(result.path.is_file());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn benchmark_rejects_zero_rows() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-db-benchmark-zero-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);

    let err = benchmark_case_db(&root, 0).unwrap_err();

    assert!(err.contains("greater than 0"));
    let _ = fs::remove_dir_all(root);
}
