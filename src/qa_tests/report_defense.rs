use super::super::report_defense_check;
use super::helpers::{assert_finding_status, read_json, seed_report_defense_case};
use crate::{audit, case_db};
use std::fs;
use std::path::Path;

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
    let machine_report = read_json(&output_dir.join("report-defense-report.json"));
    assert_finding_status(&machine_report, "banned_legal_wording", "failed");

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
fn report_defense_displays_optional_audit_chain_states_without_pass_labels() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-report-optional-audit-chain-test-{}",
        std::process::id()
    ));
    let case_dir = root.join("case");
    let output_dir = root.join("qa");
    let _ = fs::remove_dir_all(&root);
    seed_report_defense_case(&case_dir, "<html><body>scan-only case</body></html>");

    let report = report_defense_check(&case_dir, &output_dir).unwrap();

    assert!(report.passed);
    let checklist = fs::read_to_string(output_dir.join("report-defense-checklist.md")).unwrap();
    assert!(checklist.contains("[not-applicable] proxy"));
    assert!(checklist.contains("[unsupported] filesystem recovery"));
    assert!(!checklist.contains("[PASS] proxy"));
    assert!(!checklist.contains("[PASS] filesystem recovery"));
    let machine_report = read_json(&output_dir.join("report-defense-report.json"));
    assert_eq!(machine_report["qa_type"], "report_defense");
    assert_finding_status(
        &machine_report,
        "audit_chain_filesystem_recovery",
        "unsupported",
    );
    assert_finding_status(&machine_report, "audit_chain_proxy", "not-applicable");

    let _ = fs::remove_dir_all(root);
}
#[test]
fn report_defense_allows_valid_required_proxy_audit_log() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-report-valid-audit-chain-test-{}",
        std::process::id()
    ));
    let case_dir = root.join("case");
    let output_dir = root.join("qa");
    let _ = fs::remove_dir_all(&root);
    seed_report_defense_case(
        &case_dir,
        "<html><body>derived artifact artifacts/proxies/proxy_000001.mp4</body></html>",
    );
    fs::create_dir_all(case_dir.join("artifacts/proxies")).unwrap();
    fs::write(
        case_dir.join("artifacts/proxies/proxy_000001.mp4"),
        "proxy bytes",
    )
    .unwrap();
    audit::append_chained_jsonl(
        &case_dir.join("artifacts/proxies/proxy-log.jsonl"),
        r#"{"event":"make-proxy","kind":"proxy"}"#,
    )
    .unwrap();

    let report = report_defense_check(&case_dir, &output_dir).unwrap();

    assert!(report.passed);
    let checklist = fs::read_to_string(output_dir.join("report-defense-checklist.md")).unwrap();
    assert!(checklist.contains("[valid] proxy"));
    assert!(checklist.contains("required=yes"));

    let _ = fs::remove_dir_all(root);
}
