use super::super::report_defense_check;
use super::helpers::seed_report_defense_case;
use std::fs;

#[test]
fn report_defense_blocks_missing_required_proxy_audit_log() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-report-missing-audit-chain-test-{}",
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

    let err = report_defense_check(&case_dir, &output_dir).unwrap_err();

    assert!(err.contains("artifacts/proxies/proxy-log.jsonl"));
    assert!(err.contains("missing"));
    let checklist = fs::read_to_string(output_dir.join("report-defense-checklist.md")).unwrap();
    assert!(checklist.contains("[missing] proxy"));
    assert!(checklist.contains("required=yes"));
    assert!(checklist.contains("artifacts/proxies/proxy-log.jsonl"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn report_defense_blocks_missing_audit_log_for_report_claimed_proxy() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-report-claimed-audit-chain-test-{}",
        std::process::id()
    ));
    let case_dir = root.join("case");
    let output_dir = root.join("qa");
    let _ = fs::remove_dir_all(&root);
    seed_report_defense_case(
        &case_dir,
        "<html><body>reported derived artifact artifacts/proxies/proxy_000001.mp4</body></html>",
    );

    let err = report_defense_check(&case_dir, &output_dir).unwrap_err();

    assert!(err.contains("artifacts/proxies/proxy-log.jsonl"));
    let checklist = fs::read_to_string(output_dir.join("report-defense-checklist.md")).unwrap();
    assert!(checklist.contains("[missing] proxy"));
    assert!(checklist.contains("reason=report claims artifacts under artifacts/proxies"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn report_defense_blocks_missing_audit_log_for_report_claimed_validation() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-report-claimed-validation-chain-test-{}",
        std::process::id()
    ));
    let case_dir = root.join("case");
    let output_dir = root.join("qa");
    let _ = fs::remove_dir_all(&root);
    seed_report_defense_case(
        &case_dir,
        r#"<html><body>{"validation_status":"ffprobe-video-stream-confirmed"}</body></html>"#,
    );

    let err = report_defense_check(&case_dir, &output_dir).unwrap_err();

    assert!(err.contains("evidence/logs/validation-log.jsonl"));
    assert!(err.contains("missing"));
    let checklist = fs::read_to_string(output_dir.join("report-defense-checklist.md")).unwrap();
    assert!(checklist.contains("[missing] validation"));
    assert!(checklist.contains("required=yes"));
    assert!(checklist.contains("reason=report claims validation status"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn report_defense_blocks_missing_audit_log_for_recovered_filesystem_artifact() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-report-recovered-filesystem-chain-test-{}",
        std::process::id()
    ));
    let case_dir = root.join("case");
    let output_dir = root.join("qa");
    let _ = fs::remove_dir_all(&root);
    seed_report_defense_case(
        &case_dir,
        "<html><body>recovered artifact artifacts/recovered/filesystem/inode_1304.bin</body></html>",
    );
    fs::create_dir_all(case_dir.join("artifacts/recovered/filesystem")).unwrap();
    fs::write(
        case_dir.join("artifacts/recovered/filesystem/inode_1304.bin"),
        "filesystem bytes",
    )
    .unwrap();

    let err = report_defense_check(&case_dir, &output_dir).unwrap_err();

    assert!(err.contains("evidence/logs/tsk-audit.jsonl"));
    assert!(err.contains("missing"));
    let checklist = fs::read_to_string(output_dir.join("report-defense-checklist.md")).unwrap();
    assert!(checklist.contains("[missing] filesystem recovery"));
    assert!(checklist.contains("required=yes"));
    assert!(
        checklist.contains(
            "reason=case surface contains artifacts under artifacts/recovered/filesystem"
        )
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn report_defense_blocks_empty_required_proxy_audit_log() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-report-empty-audit-chain-test-{}",
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
    fs::write(case_dir.join("artifacts/proxies/proxy-log.jsonl"), "").unwrap();

    let err = report_defense_check(&case_dir, &output_dir).unwrap_err();

    assert!(err.contains("artifacts/proxies/proxy-log.jsonl"));
    assert!(err.contains("empty"));
    let checklist = fs::read_to_string(output_dir.join("report-defense-checklist.md")).unwrap();
    assert!(checklist.contains("[empty] proxy"));
    assert!(checklist.contains("required=yes"));

    let _ = fs::remove_dir_all(root);
}
