use super::super::{ReleaseReadinessOptions, privacy_review_check, release_readiness_report};
use super::helpers::{assert_finding_status, read_json, seed_report_defense_case};
use std::fs;

#[test]
fn privacy_review_rejects_full_path_leakage_with_exact_finding_key() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-privacy-leakage-test-{}",
        std::process::id()
    ));
    let case_dir = root.join("case");
    let output_dir = case_dir.join("reports/qa");
    let _ = fs::remove_dir_all(&root);
    seed_report_defense_case(
        &case_dir,
        &format!(
            "<html><body>leaked source path {}</body></html>",
            case_dir.display()
        ),
    );

    let err = privacy_review_check(&case_dir, &output_dir).unwrap_err();

    assert!(err.contains("full_path_leakage"));
    let machine_report = read_json(&output_dir.join("privacy-review.json"));
    assert_eq!(machine_report["qa_type"], "privacy_review");
    assert_finding_status(&machine_report, "full_path_leakage", "failed");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn privacy_review_passes_redacted_report_defensible_case() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-privacy-redacted-test-{}",
        std::process::id()
    ));
    let case_dir = root.join("case");
    let output_dir = case_dir.join("reports/qa");
    let _ = fs::remove_dir_all(&root);
    seed_report_defense_case(
        &case_dir,
        r#"<html><body>{"path_disclosure_mode":"redacted","local_operator_full_path_disclosure":false,"validation_status":"candidate-unvalidated"}</body></html>"#,
    );

    let report = privacy_review_check(&case_dir, &output_dir).unwrap();

    assert!(report.passed);
    let machine_report = read_json(&output_dir.join("privacy-review.json"));
    assert_finding_status(&machine_report, "full_path_disclosure_mode", "pass");
    assert!(
        machine_report["allowed_language"]
            .as_array()
            .unwrap()
            .iter()
            .any(|term| term == "candidate-unvalidated")
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn release_reads_executable_privacy_json_before_claiming_privacy_review() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-release-privacy-json-test-{}",
        std::process::id()
    ));
    let case_dir = root.join("case");
    let output_dir = case_dir.join("reports/qa");
    let _ = fs::remove_dir_all(&root);
    seed_report_defense_case(
        &case_dir,
        "<html><body>court-grade claim with no privacy-safe metadata</body></html>",
    );

    let err = release_readiness_report(
        &case_dir,
        &output_dir,
        &ReleaseReadinessOptions {
            corpus_manifest: None,
            comparison_case_dir: None,
            review_manifest: None,
            performance_output_dir: Some(root.join("performance")),
            performance_rows: 1_000,
        },
    )
    .unwrap_err();

    assert!(err.contains("privacy_review"));
    assert!(err.contains("banned_legal_wording"));
    let release_json = fs::read_to_string(output_dir.join("release-readiness.json")).unwrap();
    assert!(release_json.contains(r#""name":"privacy_review""#));
    assert!(release_json.contains(r#""evidence":"#));
    assert!(release_json.contains("privacy-review.json"));
    assert_finding_status(
        &read_json(&output_dir.join("privacy-review.json")),
        "banned_legal_wording",
        "failed",
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn release_rejects_stale_report_defense_json_when_current_check_errors() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-release-stale-report-defense-test-{}",
        std::process::id()
    ));
    let case_dir = root.join("case");
    let output_dir = case_dir.join("reports/qa");
    let _ = fs::remove_dir_all(&root);
    seed_report_defense_case(
        &case_dir,
        r#"<html><body>{"path_disclosure_mode":"redacted","local_operator_full_path_disclosure":false}</body></html>"#,
    );
    fs::create_dir_all(&output_dir).unwrap();
    fs::write(
        output_dir.join("report-defense-report.json"),
        r#"{
  "schema_version": 1,
  "qa_type": "report_defense",
  "passed": true,
  "findings": []
}
"#,
    )
    .unwrap();
    fs::write(case_dir.join("db/case.db"), "not sqlite").unwrap();

    let err = release_readiness_report(
        &case_dir,
        &output_dir,
        &ReleaseReadinessOptions {
            corpus_manifest: None,
            comparison_case_dir: None,
            review_manifest: None,
            performance_output_dir: Some(root.join("performance")),
            performance_rows: 1_000,
        },
    )
    .unwrap_err();

    assert!(err.contains("report_defense"));
    let release_json = fs::read_to_string(output_dir.join("release-readiness.json")).unwrap();
    assert!(release_json.contains(r#""name":"report_defense""#));
    assert!(release_json.contains(r#""status":"FAIL""#));
    assert!(!release_json.contains(r#"{"name":"report_defense","status":"PASS""#));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn release_writes_no_go_decision_when_checks_fail() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-release-decision-fail-test-{}",
        std::process::id()
    ));
    let case_dir = root.join("case");
    let output_dir = case_dir.join("reports/qa");
    let _ = fs::remove_dir_all(&root);
    seed_report_defense_case(
        &case_dir,
        r#"<html><body>{"path_disclosure_mode":"redacted","local_operator_full_path_disclosure":false,"validation_status":"candidate-unvalidated"}</body></html>"#,
    );

    let err = release_readiness_report(
        &case_dir,
        &output_dir,
        &ReleaseReadinessOptions {
            corpus_manifest: None,
            comparison_case_dir: None,
            review_manifest: None,
            performance_output_dir: Some(root.join("performance")),
            performance_rows: 1_000,
        },
    )
    .unwrap_err();

    assert!(err.contains("release readiness failed"));
    let decision = read_json(&output_dir.join("release-decision.json"));
    assert_eq!(decision["qa_type"], "release_decision");
    assert_eq!(decision["decision"], "BLOCKED");
    assert!(
        decision["blockers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|blocker| blocker["name"] == "review_gate_technical_review"
                && blocker["evidence"] == "missing --review-manifest")
    );

    let _ = fs::remove_dir_all(root);
}
