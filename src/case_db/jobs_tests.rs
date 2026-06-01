use super::{fail_job, interrupt_running_jobs, running_job_ids, start_job};
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
