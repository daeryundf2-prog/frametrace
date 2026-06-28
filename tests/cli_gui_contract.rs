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

fn path(path: &Path) -> &str {
    path.to_str().expect("test paths should be UTF-8")
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
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains(expected),
        "expected output to contain {expected:?}\nactual:\n{combined}"
    );
}

fn json_stdout(output: &Output) -> Value {
    assert_success(output);
    serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON")
}

#[test]
fn gui_adapter_contract_exposes_engine_owned_surfaces_when_case_opened() {
    let root = unique_temp_dir("gui-contract");
    let case_dir = root.join("case");
    let source_dir = root.join("source");
    fs::create_dir_all(&source_dir).expect("source dir should be created");
    fs::write(source_dir.join("clip.mp4"), b"\0\0\0\x18ftypmp42payload")
        .expect("fixture video should be written");

    assert_success(&run(&[
        "init-case",
        path(&case_dir),
        "--title",
        "GUI contract",
    ]));
    assert_success(&run(&[
        "scan-folder",
        path(&case_dir),
        path(&source_dir),
        "--no-ffprobe",
    ]));

    let status = json_stdout(&run(&["workstation-status", path(&case_dir)]));
    let adapter = status
        .get("gui_data_adapter")
        .expect("workstation-status should expose gui_data_adapter contract");
    assert_eq!(adapter["state_owner"], "rust-engine-sqlite-audit");
    assert_eq!(adapter["gui_durable_state_allowed"], false);
    assert_eq!(adapter["max_page_size"], 500);
    assert_eq!(
        adapter["surfaces"]["case_open"]["command"],
        "workstation-status"
    );
    assert_eq!(
        adapter["surfaces"]["inventory_page"]["command"],
        "inventory"
    );
    assert_eq!(
        adapter["surfaces"]["inventory_search"]["command"],
        "inventory"
    );
    assert_eq!(
        adapter["surfaces"]["inventory_facets"]["command"],
        "inventory --facets"
    );
    assert_eq!(
        adapter["surfaces"]["inventory_detail"]["command"],
        "inventory --file-id"
    );
    assert_eq!(
        adapter["surfaces"]["bulk_preview"]["command"],
        "inventory-bulk-preview"
    );
    assert_eq!(
        adapter["surfaces"]["export_manifest"]["command"],
        "inventory-export-manifest"
    );
    assert_eq!(
        adapter["surfaces"]["validation_playback_state"]["command"],
        "workstation-status"
    );
    assert_eq!(
        adapter["surfaces"]["report_package_status"]["command"],
        "workstation-status"
    );

    let page = json_stdout(&run(&[
        "inventory",
        path(&case_dir),
        "--limit",
        "1000",
        "--extension",
        "mp4",
        "--validation-state",
        "candidate-unvalidated",
        "--sort",
        "risk-timestamp-asc",
    ]));
    assert_eq!(page["view"], "inventory");
    assert_eq!(page["page_size"], 500);
    assert_eq!(page["total_rows"], 1);
    assert_eq!(page["truncated"], false);
    assert_eq!(page["rows"][0]["file_id"], "vid_000001");
    assert_eq!(page["rows"][0]["validation_state"], "candidate-unvalidated");

    let facets = json_stdout(&run(&["inventory", path(&case_dir), "--facets"]));
    assert_eq!(facets["view"], "facets");
    assert_eq!(facets["facets"]["candidate_count"], 1);

    let detail = json_stdout(&run(&[
        "inventory",
        path(&case_dir),
        "--file-id",
        "vid_000001",
    ]));
    assert_eq!(detail["view"], "detail");
    assert_eq!(detail["row"]["file_id"], "vid_000001");

    let preview = json_stdout(&run(&[
        "inventory-bulk-preview",
        path(&case_dir),
        "--action",
        "add-to-report",
        "--operator",
        "qa",
        "vid_000001",
    ]));
    assert_eq!(preview["view"], "bulk-preview");
    assert_eq!(preview["selected_count"], 1);
    assert_eq!(preview["audit_event"]["mutation_committed"], false);
    assert_eq!(preview["expected_mutation"], "report_state -> included");

    let manifest_path = case_dir.join("reports/inventory-export.json");
    let manifest_result = json_stdout(&run(&[
        "inventory-export-manifest",
        path(&case_dir),
        "--operator",
        "qa",
        "--output",
        path(&manifest_path),
        "vid_000001",
    ]));
    assert_eq!(manifest_result["view"], "inventory-export-manifest");
    assert_eq!(manifest_result["selected_count"], 1);
    let manifest_text =
        fs::read_to_string(&manifest_path).expect("inventory manifest should be written");
    let manifest: Value =
        serde_json::from_str(&manifest_text).expect("manifest should be valid JSON");
    assert_eq!(manifest["source_of_truth"], "case.db/videos");
    assert_eq!(manifest["browser_large_case_policy"], "paged-query-only");
    assert_eq!(manifest["rows"][0]["file_id"], "vid_000001");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn inventory_adapter_rejects_missing_case_clearly() {
    let bad_case = unique_temp_dir("not-case");
    fs::create_dir_all(&bad_case).expect("bad case dir should be created");

    let output = run(&["inventory", path(&bad_case), "--limit", "10"]);

    assert_failure_contains(&output, "not a FrameTrace case");
    let _ = fs::remove_dir_all(bad_case);
}
