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

#[test]
fn release_gate_reports_typed_review_manifest_blockers() {
    let root = unique_temp_dir("cli-release-gate");
    let case_dir = root.join("case");
    let output_dir = root.join("qa");
    let manifest = root.join("release-review.json");
    let artifact = root.join("technical-review.json");
    fs::create_dir_all(&case_dir).expect("case dir should be created");
    fs::create_dir_all(&output_dir).expect("output dir should be created");
    fs::write(&artifact, "{}").expect("review artifact should be written");
    fs::write(
        &manifest,
        format!(
            r#"{{
  "schema_version": 1,
  "gates": [
    {{
      "key": "technical_review",
      "status": "done",
      "artifact_path": "{}",
      "tool": "manual-review-recorder",
      "evidence": "manual review artifact",
      "timestamp": "2026-06-24T00:00:00Z",
      "reviewer": "qa",
      "cleanup_status": "clean"
    }}
  ]
}}"#,
            artifact.display()
        ),
    )
    .expect("manifest should be written");

    let output = run(&[
        "qa",
        "release",
        path(&case_dir),
        "--review-manifest",
        path(&manifest),
        "--output-dir",
        path(&output_dir),
    ]);

    assert!(!output.status.success());
    let readiness = fs::read_to_string(output_dir.join("release-readiness.json"))
        .expect("release readiness JSON should be written");
    assert!(readiness.contains(r#""name":"review_gate_technical_review""#));
    assert!(readiness.contains(r#""status":"FAIL""#));
    assert!(readiness.contains("unsupported status"));
    assert!(readiness.contains(r#""name":"privacy_review""#));

    fs::write(
        &manifest,
        format!(
            r#"{{
  "schema_version": 1,
  "gates": [
    {{
      "key": "technical_review",
      "status": "PASS",
      "artifact_path": "{}",
      "tool": "manual-review-recorder",
      "evidence": "manual review artifact",
      "timestamp": "2026-06-24T00:00:00Z",
      "reviewer": "qa",
      "cleanup_status": "clean"
    }}
  ]
}}"#,
            artifact.display()
        ),
    )
    .expect("valid manifest should be written");

    let output = run(&[
        "qa",
        "release",
        path(&case_dir),
        "--review-manifest",
        path(&manifest),
        "--output-dir",
        path(&output_dir),
    ]);

    assert!(!output.status.success());
    let readiness = fs::read_to_string(output_dir.join("release-readiness.json"))
        .expect("release readiness JSON should be written");
    assert!(readiness.contains(r#"{"name":"review_gate_technical_review","status":"PASS""#));
    assert!(readiness.contains(r#"{"name":"review_gate_privacy_review","status":"BLOCKED""#));

    let _ = fs::remove_dir_all(root);
}

fn path(path: &Path) -> &str {
    path.to_str().expect("test paths should be UTF-8")
}
