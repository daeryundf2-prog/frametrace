use super::qa_test_fixtures::seed_repro_case;
use super::{
    accuracy_report, performance_report, read_review_manifest, report_defense_check,
    reproducibility_report,
};
use crate::audit;
use crate::case_db;
use std::fs;
use std::path::Path;

#[test]
fn accuracy_report_passes_for_matching_manifest() {
    let root = std::env::temp_dir().join(format!("frametrace-qa-test-{}", std::process::id()));
    let case_dir = root.join("case");
    let output_dir = root.join("qa");
    let manifest = root.join("corpus.tsv");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(case_dir.join("db")).unwrap();
    let source_path = "/evidence/a.mp4";
    fs::write(
        case_dir.join("db/videos.jsonl"),
        format!(
            "{{\"source_path\":\"{}\",\"sha256\":\"abc\"}}\n",
            source_path
        ),
    )
    .unwrap();
    fs::write(&manifest, format!("{source_path}\tabc\n")).unwrap();

    let report = accuracy_report(&case_dir, &manifest, &output_dir).unwrap();
    assert!(report.passed);
    assert!(report.report_path.is_file());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn accuracy_report_includes_recovery_artifacts() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-qa-recovery-test-{}",
        std::process::id()
    ));
    let case_dir = root.join("case");
    let output_dir = root.join("qa");
    let manifest = root.join("corpus.tsv");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(case_dir.join("db")).unwrap();
    fs::create_dir_all(case_dir.join("artifacts/carved")).unwrap();
    fs::create_dir_all(case_dir.join("evidence/logs")).unwrap();
    fs::write(case_dir.join("db/videos.jsonl"), "").unwrap();
    let carved_path = root.join("case/artifacts/carved/carve_000001.mp4");
    fs::write(
        case_dir.join("artifacts/carved/carve-log.jsonl"),
        format!(
            "{{\"id\":\"carve_000001\",\"output_path\":\"{}\",\"sha256\":\"abc\",\"validation_status\":\"candidate-unvalidated\"}}\n",
            carved_path.display()
        ),
    )
    .unwrap();
    fs::write(
        case_dir.join("evidence/logs/validation-log.jsonl"),
        format!(
            "{{\"selector\":\"carve_000001\",\"target_path\":\"{}\",\"target_sha256\":\"abc\",\"validation_status\":\"ffprobe-video-stream-confirmed\"}}\n",
            carved_path.display()
        ),
    )
    .unwrap();
    fs::write(&manifest, format!("{}\tabc\n", carved_path.display())).unwrap();

    let report = accuracy_report(&case_dir, &manifest, &output_dir).unwrap();
    assert!(report.passed);
    assert!(report.report_path.is_file());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn report_defense_rejects_disallowed_legal_claims() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-report-claims-test-{}",
        std::process::id()
    ));
    let case_dir = root.join("case");
    let output_dir = root.join("qa");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(case_dir.join("db")).unwrap();
    fs::create_dir_all(case_dir.join("reports")).unwrap();
    fs::create_dir_all(case_dir.join("review")).unwrap();
    fs::write(case_dir.join("case.json"), "{}").unwrap();
    fs::write(case_dir.join("db/case.db"), "").unwrap();
    fs::write(case_dir.join("db/video_index.json"), "{}").unwrap();
    fs::write(case_dir.join("db/videos.jsonl"), "").unwrap();
    fs::write(case_dir.join("db/video_paths.tsv"), "id\tsource_path\n").unwrap();
    fs::write(
        case_dir.join("reports/case-report.html"),
        "<html>court-ready recovery</html>",
    )
    .unwrap();
    fs::write(
        case_dir.join("review/evidence-viewer.html"),
        "<html></html>",
    )
    .unwrap();

    let err = report_defense_check(&case_dir, &output_dir).unwrap_err();
    assert!(err.contains("disallowed claim"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn report_defense_rejects_cases_with_running_jobs() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-active-jobs-test-{}",
        std::process::id()
    ));
    let case_dir = root.join("case");
    let output_dir = root.join("qa");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(case_dir.join("db")).unwrap();
    fs::create_dir_all(case_dir.join("reports")).unwrap();
    fs::write(case_dir.join("case.json"), "{}").unwrap();
    fs::write(case_dir.join("db/video_index.json"), "{}").unwrap();
    fs::write(case_dir.join("db/videos.jsonl"), "").unwrap();
    fs::write(case_dir.join("db/video_paths.tsv"), "id\tsource_path\n").unwrap();
    fs::write(case_dir.join("reports/case-report.html"), "<html></html>").unwrap();
    case_db::start_job(
        &case_dir,
        "scan-folder",
        Path::new("/evidence"),
        Some(1),
        "{}",
    )
    .unwrap();

    let err = report_defense_check(&case_dir, &output_dir).unwrap_err();

    assert!(err.contains("active job"));
    let checklist = fs::read_to_string(output_dir.join("report-defense-checklist.md")).unwrap();
    assert!(checklist.contains("running SQLite job"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn report_defense_rejects_tampered_media_audit_logs() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-report-audit-chain-test-{}",
        std::process::id()
    ));
    let case_dir = root.join("case");
    let output_dir = root.join("qa");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(case_dir.join("db")).unwrap();
    fs::create_dir_all(case_dir.join("reports")).unwrap();
    fs::create_dir_all(case_dir.join("artifacts/proxies")).unwrap();
    fs::write(case_dir.join("case.json"), "{}").unwrap();
    fs::write(case_dir.join("db/video_index.json"), "{}").unwrap();
    fs::write(case_dir.join("db/videos.jsonl"), "").unwrap();
    fs::write(case_dir.join("db/video_paths.tsv"), "id\tsource_path\n").unwrap();
    fs::write(case_dir.join("reports/case-report.html"), "<html></html>").unwrap();
    let conn = case_db::open_case_db(&case_dir).unwrap();
    case_db::init_schema(&conn).unwrap();
    let proxy_log = case_dir.join("artifacts/proxies/proxy-log.jsonl");
    audit::append_chained_jsonl(&proxy_log, r#"{"event":"make-proxy","kind":"proxy"}"#).unwrap();
    let tampered = fs::read_to_string(&proxy_log)
        .unwrap()
        .replace("make-proxy", "make-proxy-tampered");
    fs::write(&proxy_log, tampered).unwrap();

    let err = report_defense_check(&case_dir, &output_dir).unwrap_err();

    assert!(err.contains("audit chain"));
    let checklist = fs::read_to_string(output_dir.join("report-defense-checklist.md")).unwrap();
    assert!(checklist.contains("proxy"));
    assert!(checklist.contains("tampered"));

    let _ = fs::remove_dir_all(root);
}

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
fn performance_report_records_query_latency_metrics() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-performance-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);

    let report = performance_report(&root, 1_000).unwrap();
    let text = fs::read_to_string(&report.report_path).unwrap();

    assert!(report.passed);
    assert!(text.contains("\"query_count\": 8"));
    assert!(text.contains("\"max_query_ms\""));
    assert!(text.contains("\"query_latency_target_ms\": 2000"));
    assert!(text.contains("\"query_rows_returned\""));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn review_manifest_accepts_key_value_and_markdown_gates() {
    let root = std::env::temp_dir().join(format!("frametrace-review-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let manifest = root.join("release-review.txt");
    fs::write(
        &manifest,
        "\
technical_review=pass
security review: approved
- [x] Migration Validation
- [x] Operator Review
legal-review=done
",
    )
    .unwrap();

    let gates = read_review_manifest(&manifest).unwrap();
    assert_eq!(gates.get("technical_review"), Some(&true));
    assert_eq!(gates.get("security_review"), Some(&true));
    assert_eq!(gates.get("migration_validation"), Some(&true));
    assert_eq!(gates.get("operator_review"), Some(&true));
    assert_eq!(gates.get("legal_review"), Some(&true));

    let _ = fs::remove_dir_all(root);
}
