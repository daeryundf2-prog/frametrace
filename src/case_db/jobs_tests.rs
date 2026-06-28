use super::{complete_job, fail_job, interrupt_running_jobs, running_job_ids, start_job};
use crate::util::{create_case_layout, write_text};
use crate::workstation;
use std::fs;
use std::path::Path;

#[test]
fn marks_running_jobs_interrupted_for_recovery_review() {
    let root = std::env::temp_dir().join(format!("frametrace-jobs-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);

    let job = start_job(&root, "scan-folder", Path::new("/evidence"), Some(10), "{}").unwrap();
    assert_eq!(running_job_ids(&root).unwrap(), vec![job.job_id]);

    let count = interrupt_running_jobs(&root, "process was stopped before release QA").unwrap();

    assert_eq!(count, 1);
    assert!(running_job_ids(&root).unwrap().is_empty());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn interrupt_running_jobs_does_not_overwrite_terminal_jobs() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-terminal-jobs-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);

    let job = start_job(&root, "scan-folder", Path::new("/evidence"), Some(10), "{}").unwrap();
    fail_job(&root, &job.job_id, "operator stopped failed run").unwrap();

    let count = interrupt_running_jobs(&root, "stale job review").unwrap();

    assert_eq!(count, 0);
    assert!(running_job_ids(&root).unwrap().is_empty());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn workstation_status_lists_bounded_job_runtime_readiness() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-runtime-readiness-jobs-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    create_case_layout(&root).unwrap();
    write_text(
        &root.join("case.json"),
        r#"{"schema_version":1,"case_id":"FT-JOBS","title":"jobs"}"#,
    )
    .unwrap();

    let complete = start_job(
        &root,
        "scan-folder",
        Path::new("/evidence/a"),
        Some(10),
        "{}",
    )
    .unwrap();
    complete_job(&root, &complete.job_id, 10, "scan-folder completed").unwrap();
    let _running = start_job(
        &root,
        "carve-file",
        Path::new("/evidence/b.raw"),
        Some(100),
        "{}",
    )
    .unwrap();
    let interrupted = start_job(
        &root,
        "import-e01",
        Path::new("/evidence/c.E01"),
        Some(1000),
        "{}",
    )
    .unwrap();
    interrupt_running_jobs(&root, "qa stopped stale run").unwrap();

    let status = workstation::workstation_status_json(&root).unwrap();

    assert!(status.contains(r#""runtime_readiness":{"#));
    assert!(status.contains(r#""jobs":{"#));
    assert!(status.contains(r#""completed_count":1"#));
    assert!(status.contains(r#""interrupted_count":2"#));
    assert!(status.contains(r#""running_count":0"#));
    assert!(status.contains(r#""recent_limit":20"#));
    assert!(status.contains(&format!(r#""job_id":"{}""#, interrupted.job_id)));
    assert!(status.contains(r#""progress_percent":"100.0""#));
    assert!(status.contains(r#""eta_state":"not-applicable""#));
    assert!(status.contains(r#""resume_blocker":"resume-disabled-idempotence-not-proven""#));

    let _ = fs::remove_dir_all(root);
}
