use std::fs;
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

#[test]
fn help_command_succeeds() {
    let output = run(&["--help"]);
    assert_success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("scan-folder"));
}

#[test]
fn case_lifecycle_smoke_test_uses_real_binary() {
    let root = unique_temp_dir("cli-lifecycle");
    let case_dir = root.join("case");
    let media_dir = root.join("media");
    fs::create_dir_all(&media_dir).expect("media dir should be created");
    fs::write(media_dir.join("clip.mp4"), b"\0\0\0\x18ftypmp42payload")
        .expect("fixture video should be written");

    assert_success(&run(&[
        "init-case",
        path(&case_dir),
        "--title",
        "Smoke Case",
    ]));
    assert_success(&run(&[
        "scan-folder",
        path(&case_dir),
        path(&media_dir),
        "--no-ffprobe",
    ]));
    assert_success(&run(&["make-review", path(&case_dir)]));
    assert_success(&run(&["make-report", path(&case_dir)]));
    assert_success(&run(&["package-case", path(&case_dir)]));
    assert_success(&run(&["inspect", path(&case_dir)]));

    assert!(case_dir.join("db/video_index.json").is_file());
    assert!(case_dir.join("review/index.html").is_file());
    assert!(case_dir.join("review/evidence-viewer.html").is_file());
    assert!(case_dir.join("reports/case-report.html").is_file());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn external_tool_failure_reports_install_guidance() {
    let root = unique_temp_dir("cli-external-tool");
    let case_dir = root.join("case");
    let e01_path = root.join("sample.E01");
    fs::create_dir_all(&root).expect("root dir should be created");
    fs::write(&e01_path, b"not a real image").expect("fixture E01 should be written");
    assert_success(&run(&[
        "init-case",
        path(&case_dir),
        "--title",
        "External Tool Case",
    ]));

    let output = run(&[
        "inspect-e01",
        path(&case_dir),
        path(&e01_path),
        "--ewfinfo",
        "frametrace-definitely-missing-ewfinfo",
    ]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("install libewf tools"),
        "stderr was: {stderr}"
    );

    let _ = fs::remove_dir_all(root);
}

fn path(path: &Path) -> &str {
    path.to_str().expect("test paths should be UTF-8")
}
