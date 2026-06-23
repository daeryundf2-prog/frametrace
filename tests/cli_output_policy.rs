#![cfg(unix)]

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn frametrace() -> &'static str {
    env!("CARGO_BIN_EXE_frametrace")
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after UNIX epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("frametrace-{name}-{}-{nanos}", std::process::id()))
}

fn run(args: &[&str]) -> Output {
    Command::new(frametrace())
        .args(args)
        .output()
        .expect("frametrace binary should run")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_failure_contains(output: &Output, expected: &str) {
    assert!(
        !output.status.success(),
        "command unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains(expected),
        "expected output to contain {expected:?}\nactual:\n{combined}"
    );
}

#[test]
fn make_review_rejects_symlinked_review_outputs_without_writing_target() {
    let root = unique_temp_dir("cli-review-output-symlink");
    let case_dir = root.join("case");
    assert_success(&run(&[
        "init-case",
        path(&case_dir),
        "--title",
        "Output Policy",
    ]));
    fs::create_dir_all(case_dir.join("review")).expect("review dir should exist");
    let outside = root.join("outside-review.html");
    symlink(&outside, case_dir.join("review/index.html")).expect("symlink should be created");

    let output = run(&["make-review", path(&case_dir)]);

    assert_failure_contains(&output, "cannot be a symlink");
    assert!(
        !outside.exists(),
        "outside review target should not be written"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn make_review_rejects_symlinked_review_directory_without_writing_target() {
    let root = unique_temp_dir("cli-review-dir-symlink");
    let case_dir = root.join("case");
    assert_success(&run(&[
        "init-case",
        path(&case_dir),
        "--title",
        "Output Policy",
    ]));
    fs::remove_dir_all(case_dir.join("review")).expect("review dir should be removed");
    let outside = root.join("outside-review-dir");
    fs::create_dir_all(&outside).expect("outside dir should exist");
    symlink(&outside, case_dir.join("review")).expect("symlink should be created");

    let output = run(&["make-review", path(&case_dir)]);

    assert_failure_contains(&output, "inside the case directory");
    assert!(!outside.join("index.html").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn make_report_rejects_symlinked_report_outputs_without_writing_target() {
    let root = unique_temp_dir("cli-report-output-symlink");
    let case_dir = root.join("case");
    let media_dir = root.join("media");
    fs::create_dir_all(&media_dir).expect("media dir should exist");
    fs::write(media_dir.join("clip.mp4"), b"\0\0\0\x18ftypmp42payload")
        .expect("fixture video should be written");
    assert_success(&run(&[
        "init-case",
        path(&case_dir),
        "--title",
        "Output Policy",
    ]));
    assert_success(&run(&[
        "scan-folder",
        path(&case_dir),
        path(&media_dir),
        "--no-ffprobe",
    ]));
    fs::create_dir_all(case_dir.join("reports")).expect("reports dir should exist");
    let outside = root.join("outside-report.html");
    symlink(&outside, case_dir.join("reports/case-report.html"))
        .expect("symlink should be created");

    let output = run(&["make-report", path(&case_dir)]);

    assert_failure_contains(&output, "cannot be a symlink");
    assert!(
        !outside.exists(),
        "outside report target should not be written"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn make_report_rejects_symlinked_reports_directory_without_writing_target() {
    let root = unique_temp_dir("cli-report-dir-symlink");
    let case_dir = root.join("case");
    let media_dir = root.join("media");
    fs::create_dir_all(&media_dir).expect("media dir should exist");
    fs::write(media_dir.join("clip.mp4"), b"\0\0\0\x18ftypmp42payload")
        .expect("fixture video should be written");
    assert_success(&run(&[
        "init-case",
        path(&case_dir),
        "--title",
        "Output Policy",
    ]));
    assert_success(&run(&[
        "scan-folder",
        path(&case_dir),
        path(&media_dir),
        "--no-ffprobe",
    ]));
    fs::remove_dir_all(case_dir.join("reports")).expect("reports dir should be removed");
    let outside = root.join("outside-reports-dir");
    fs::create_dir_all(&outside).expect("outside dir should exist");
    symlink(&outside, case_dir.join("reports")).expect("symlink should be created");

    let output = run(&["make-report", path(&case_dir)]);

    assert_failure_contains(&output, "inside the case directory");
    assert!(!outside.join("case-report.html").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn scan_folder_rejects_symlinked_db_directory_without_writing_case_state_outside() {
    let root = unique_temp_dir("cli-scan-db-dir-symlink");
    let case_dir = root.join("case");
    let media_dir = root.join("media");
    fs::create_dir_all(&media_dir).expect("media dir should exist");
    fs::write(media_dir.join("clip.mp4"), b"\0\0\0\x18ftypmp42payload")
        .expect("fixture video should be written");
    assert_success(&run(&[
        "init-case",
        path(&case_dir),
        "--title",
        "Output Policy",
    ]));
    fs::remove_dir_all(case_dir.join("db")).expect("db dir should be removed");
    let outside = root.join("outside-db");
    fs::create_dir_all(&outside).expect("outside db dir should exist");
    symlink(&outside, case_dir.join("db")).expect("symlink should be created");

    let output = run(&[
        "scan-folder",
        path(&case_dir),
        path(&media_dir),
        "--no-ffprobe",
    ]);

    assert_failure_contains(&output, "inside the case directory");
    assert!(!outside.join("case.db").exists());
    assert!(!outside.join("video_index.json").exists());
    assert!(!outside.join("videos.jsonl").exists());
    assert!(!outside.join("video_paths.tsv").exists());
    assert!(!outside.join("scan_runs").exists());
    let _ = fs::remove_dir_all(root);
}

fn path(path: &Path) -> &str {
    path.to_str().expect("test paths should be UTF-8")
}
