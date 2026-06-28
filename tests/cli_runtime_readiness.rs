use serde_json::Value;
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

fn run_with_empty_path(args: &[&str], path_dir: &Path) -> Output {
    Command::new(frametrace())
        .args(args)
        .env("PATH", path_dir)
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

fn status_json(case_dir: &Path) -> Value {
    let output = run(&["workstation-status", path(case_dir)]);
    assert_success(&output);
    serde_json::from_slice(&output.stdout).expect("workstation-status stdout should parse")
}

#[test]
fn workstation_status_reports_feature_gates_and_disk_preflight() {
    let root = unique_temp_dir("runtime-readiness");
    let case_dir = root.join("case");
    assert_success(&run(&[
        "init-case",
        path(&case_dir),
        "--title",
        "Runtime readiness gates",
    ]));

    let status = status_json(&case_dir);
    let runtime = &status["runtime_readiness"];

    assert_eq!(runtime["schema_version"], 1);
    assert_eq!(
        runtime["disk_preflight"]["features"][0]["feature"],
        "import"
    );
    assert_eq!(
        runtime["disk_preflight"]["features"][0]["blockers"][0],
        "required-bytes-unknown"
    );
    assert!(
        runtime["disk_preflight"]["features"]
            .as_array()
            .expect("disk features should be an array")
            .iter()
            .any(|feature| feature["feature"] == "package")
    );
    assert!(
        runtime["feature_gates"]
            .as_array()
            .expect("feature gates should be an array")
            .iter()
            .any(|feature| {
                feature["feature"] == "validate-artifact"
                    && feature["required_tools"].as_array().is_some()
            })
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn workstation_status_reports_feature_specific_missing_tool_blockers() {
    let root = unique_temp_dir("runtime-readiness-missing-tools");
    let case_dir = root.join("case");
    let empty_path = root.join("empty-path");
    fs::create_dir_all(&empty_path).expect("empty PATH dir should be created");
    assert_success(&run(&[
        "init-case",
        path(&case_dir),
        "--title",
        "Runtime readiness missing tools",
    ]));

    let output = run_with_empty_path(&["workstation-status", path(&case_dir)], &empty_path);
    assert_success(&output);
    let status: Value =
        serde_json::from_slice(&output.stdout).expect("workstation-status stdout should parse");
    let gates = status["runtime_readiness"]["feature_gates"]
        .as_array()
        .expect("feature gates should be an array");

    let import_gate = gates
        .iter()
        .find(|gate| gate["feature"] == "import")
        .expect("import gate should exist");
    assert_eq!(import_gate["status"], "blocked");
    assert!(
        import_gate["blockers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|blocker| blocker == "missing-tool:ewfexport")
    );

    let proxy_gate = gates
        .iter()
        .find(|gate| gate["feature"] == "proxy")
        .expect("proxy gate should exist");
    assert_eq!(proxy_gate["status"], "blocked");
    assert!(
        proxy_gate["blockers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|blocker| blocker == "missing-tool:ffmpeg")
    );

    let _ = fs::remove_dir_all(root);
}

fn path(path: &Path) -> &str {
    path.to_str().expect("test paths should be UTF-8")
}
