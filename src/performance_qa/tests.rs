use super::compatibility::CompatibilityExportEvidence;
use super::render;
use super::survival::{FullJsonLoadDenial, InventoryTiming, LargeCaseSurvivalEvidence};
use super::{
    PERFORMANCE_1M_MAX_RSS_TARGET_BYTES, PERFORMANCE_100K_MAX_RSS_TARGET_BYTES,
    PERFORMANCE_QUERY_LATENCY_TARGET_MS, max_rss_target_for_rows, performance_passed,
    performance_report,
};
use crate::case_db::{DbBenchmarkQueryPlan, DbBenchmarkResult};
use crate::resource_monitor::ResourceUsage;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

#[test]
fn performance_report_writes_query_plan_evidence() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-performance-query-plan-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);

    let report = performance_report(&root, 1_000).unwrap();
    let json = fs::read_to_string(&report.report_path).unwrap();
    let markdown = fs::read_to_string(root.join("performance-report.md")).unwrap();

    assert!(report.passed);
    assert!(json.contains("\"query_plan_full_scan_count\": 0"));
    assert!(json.contains("\"query_plans\""));
    assert!(json.contains("\"max_rss_bytes\""));
    assert!(json.contains("\"max_rss_target_bytes\""));
    assert!(json.contains("\"cpu_average_percent\""));
    assert!(json.contains("\"cpu_gate_status\""));
    assert!(markdown.contains("## Query Plans"));
    assert!(markdown.contains("## Resource Metrics"));
    assert!(markdown.contains("EXPLAIN"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn performance_report_preserves_existing_output_shape_when_t10_metrics_are_added() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-performance-shape-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);

    let report = performance_report(&root, 1_000).unwrap();
    let json = fs::read_to_string(&report.report_path).unwrap();

    assert!(report.passed);
    assert!(json.contains("\"schema_version\": 1"));
    assert!(json.contains("\"qa_type\": \"performance\""));
    assert!(json.contains("\"rows\": 1000"));
    assert!(json.contains("\"rows_per_minute\""));
    assert!(json.contains("\"query_count\": 8"));
    assert!(json.contains("\"query_plans\""));
    assert!(json.contains("\"database_path\""));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn performance_report_records_t10_large_case_survival_surfaces() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-performance-t10-surfaces-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);

    let report = performance_report(&root, 1_000).unwrap();
    let json = fs::read_to_string(&report.report_path).unwrap();
    let parsed: Value = serde_json::from_str(&json).unwrap();
    let survival = parsed["large_case_survival"].as_object().unwrap();
    let compatibility = survival["compatibility_exports"].as_object().unwrap();
    let denial = survival["full_json_load_denial"].as_object().unwrap();

    assert!(report.passed);
    assert!(
        survival["max_query_ms"].as_u64().unwrap() <= PERFORMANCE_QUERY_LATENCY_TARGET_MS as u64
    );
    assert_eq!(
        survival["query_latency_target_ms"].as_u64().unwrap(),
        PERFORMANCE_QUERY_LATENCY_TARGET_MS as u64
    );
    assert_eq!(compatibility["jsonl_rows"].as_u64().unwrap(), 1_000);
    assert_eq!(compatibility["tsv_rows"].as_u64().unwrap(), 1_000);
    assert!(!denial["full_json_load_allowed"].as_bool().unwrap());
    assert!(root.join("reports/case-report.html").is_file());
    assert!(root.join("review/index.html").is_file());
    assert!(root.join("review/evidence-viewer.html").is_file());
    assert!(root.join("db/videos.jsonl").is_file());
    assert!(root.join("db/video_paths.tsv").is_file());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn performance_report_uses_plan_memory_targets_for_100k_and_1m_profiles() {
    assert_eq!(max_rss_target_for_rows(100_000), 1_610_612_736);
    assert_eq!(
        max_rss_target_for_rows(100_000),
        PERFORMANCE_100K_MAX_RSS_TARGET_BYTES
    );
    assert_eq!(max_rss_target_for_rows(1_000_000), 3_758_096_384);
    assert_eq!(
        max_rss_target_for_rows(1_000_000),
        PERFORMANCE_1M_MAX_RSS_TARGET_BYTES
    );
}

#[test]
fn performance_json_fixture_records_plan_memory_targets_for_1m() {
    let result = benchmark_result_fixture(1_000_000, 574);
    let resources = resource_usage_fixture(PERFORMANCE_1M_MAX_RSS_TARGET_BYTES);
    let survival = survival_fixture(PERFORMANCE_QUERY_LATENCY_TARGET_MS + 1);
    let json = render::performance_json(
        false,
        &result,
        &resources,
        1_002_707,
        0,
        &PathBuf::from("performance-report.md"),
        &survival,
    );
    let parsed: Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["rows"].as_u64().unwrap(), 1_000_000);
    assert_eq!(
        parsed["max_rss_target_bytes"].as_u64().unwrap(),
        3_758_096_384
    );
    assert_eq!(
        parsed["large_case_survival"]["max_query_ms"]
            .as_u64()
            .unwrap(),
        2_001
    );
    assert_eq!(
        parsed["large_case_survival"]["query_latency_target_ms"]
            .as_u64()
            .unwrap(),
        2_000
    );
    assert_eq!(
        parsed["large_case_survival"]["compatibility_exports"]["jsonl_rows"]
            .as_u64()
            .unwrap(),
        1_000_000
    );
    assert_eq!(
        parsed["large_case_survival"]["compatibility_exports"]["tsv_rows"]
            .as_u64()
            .unwrap(),
        1_000_000
    );
    assert!(
        !parsed["large_case_survival"]["full_json_load_denial"]["full_json_load_allowed"]
            .as_bool()
            .unwrap()
    );
}

#[test]
fn one_million_profile_fails_closed_when_survival_query_latency_exceeds_target() {
    let result = benchmark_result_fixture(1_000_000, 574);
    let resources = resource_usage_fixture(PERFORMANCE_1M_MAX_RSS_TARGET_BYTES);
    let survival = survival_fixture(PERFORMANCE_QUERY_LATENCY_TARGET_MS + 142);

    assert!(!performance_passed(
        &result, &resources, 1_002_707, 0, &survival
    ));
}

fn benchmark_result_fixture(rows: usize, max_query_ms: u128) -> DbBenchmarkResult {
    DbBenchmarkResult {
        path: PathBuf::from("db/case.db"),
        rows,
        elapsed_ms: 1,
        query_count: 8,
        max_query_ms,
        query_rows_returned: 500,
        query_plans: vec![DbBenchmarkQueryPlan {
            label: "fixture".to_string(),
            sql: "SELECT id FROM videos WHERE extension = ?1".to_string(),
            detail: "SEARCH videos USING INDEX idx_videos_extension".to_string(),
        }],
    }
}

fn resource_usage_fixture(max_rss_target_bytes: u64) -> ResourceUsage {
    ResourceUsage {
        max_rss_bytes: Some(27_033_600),
        max_rss_target_bytes,
        average_cpu_percent: Some(94.0),
        cpu_target_percent: 95.0,
        cpu_target_enforced: false,
        sample_count: 1,
    }
}

fn survival_fixture(max_query_ms: u128) -> LargeCaseSurvivalEvidence {
    LargeCaseSurvivalEvidence {
        report_generation_ms: 2_237,
        review_bundle_generation_ms: 49,
        inventory_timings: vec![InventoryTiming {
            operation: "inventory-sort-size-desc",
            duration_ms: max_query_ms,
            total_rows: 1_000_000,
            returned_rows: 500,
            truncated: true,
        }],
        compatibility_exports: CompatibilityExportEvidence {
            jsonl_path: "db/videos.jsonl".to_string(),
            jsonl_rows: 1_000_000,
            jsonl_elapsed_ms: 5_231,
            jsonl_rows_per_minute: 11_470_082,
            tsv_path: "db/video_paths.tsv".to_string(),
            tsv_rows: 1_000_000,
            tsv_elapsed_ms: 4_394,
            tsv_rows_per_minute: 13_654_984,
            manifest_path: "reports/inventory-export.json".to_string(),
            manifest_selected_count: 3,
            manifest_elapsed_ms: 13_893,
        },
        full_json_load_denial: FullJsonLoadDenial {
            status: "DENIED",
            full_json_load_allowed: false,
            evidence: "fixture denial".to_string(),
        },
        max_query_ms,
    }
}
