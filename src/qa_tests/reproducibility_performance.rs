use super::super::qa_test_fixtures::seed_repro_case;
use super::super::{performance_report_for_test, reproducibility_report};
use super::helpers::read_json;
use std::fs;

#[test]
fn reproducibility_normalizes_case_paths_timestamps_and_package_times() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-repro-normalized-test-{}",
        std::process::id()
    ));
    let left = root.join("left-case");
    let right = root.join("right-case");
    let output_dir = root.join("qa");
    let _ = fs::remove_dir_all(&root);
    seed_repro_case(&left, 111, "ffprobe-video-stream-confirmed", "abc");
    seed_repro_case(&right, 999, "ffprobe-video-stream-confirmed", "abc");

    let report = reproducibility_report(&left, &right, &output_dir).unwrap();
    assert!(report.passed);
    assert!(report.report_path.is_file());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn reproducibility_detects_recovery_validation_drift() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-repro-drift-test-{}",
        std::process::id()
    ));
    let left = root.join("left-case");
    let right = root.join("right-case");
    let output_dir = root.join("qa");
    let _ = fs::remove_dir_all(&root);
    seed_repro_case(&left, 111, "ffprobe-video-stream-confirmed", "abc");
    seed_repro_case(&right, 999, "validation-failed", "abc");

    let err = reproducibility_report(&left, &right, &output_dir).unwrap_err();
    assert!(err.contains("normalized core outputs differ"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn reproducibility_report_records_diff_threshold_metrics() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-repro-threshold-test-{}",
        std::process::id()
    ));
    let left = root.join("left-case");
    let right = root.join("right-case");
    let output_dir = root.join("qa");
    let _ = fs::remove_dir_all(&root);
    seed_repro_case(
        &left,
        111,
        "ffprobe-video-stream-confirmed",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    seed_repro_case(
        &right,
        999,
        "ffprobe-video-stream-confirmed",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );

    let report = reproducibility_report(&left, &right, &output_dir).unwrap();
    let json = read_json(&report.report_path);

    assert!(report.passed);
    assert_eq!(
        json["allowed_diff_thresholds"]["normalized_core_differences"],
        0
    );
    assert_eq!(json["diff_metrics"]["normalized_core_differences"], 0);
    assert_eq!(
        json["diff_metrics"]["normalized_left_bytes"],
        json["normalized_left_bytes"]
    );
    assert_eq!(
        json["diff_metrics"]["normalized_right_bytes"],
        json["normalized_right_bytes"]
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn performance_report_records_query_latency_metrics() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-performance-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);

    let report = performance_report_for_test(&root, 1_000).unwrap();
    let text = fs::read_to_string(&report.report_path).unwrap();

    assert!(report.passed);
    assert!(text.contains("\"query_count\": 8"));
    assert!(text.contains("\"max_query_ms\""));
    assert!(text.contains("\"query_latency_target_ms\": 2000"));
    assert!(text.contains("\"query_rows_returned\""));

    let _ = fs::remove_dir_all(root);
}
