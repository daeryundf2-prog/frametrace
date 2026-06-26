use frametrace::case_db::write_scan_index;
use frametrace::model::{ProbeSummary, ScanOptions, ScanResult, SourceProfile, VideoRecord};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn frametrace() -> &'static str {
    env!("CARGO_BIN_EXE_frametrace")
}

fn run(args: &[&str]) -> Output {
    Command::new(frametrace())
        .args(args)
        .output()
        .expect("frametrace binary should run")
}

#[test]
fn make_report_preserves_small_case_video_rows_when_sqlite_is_available() {
    let root = unique_temp_dir("bounded-report-small");
    let case_dir = root.join("case");
    fs::create_dir_all(case_dir.join("db")).expect("db dir should be created");
    write_manifest(&case_dir, "FT-SMALL");
    let scan = scan_result(vec![
        video("vid_000001", "/evidence/small/tiny-a.mp4", 10),
        video("vid_000002", "/evidence/small/tiny-b.mov", 20),
    ]);
    write_scan_index(&case_dir, &scan, &[]).expect("SQLite index should be written");
    fs::write(case_dir.join("db/video_index.json"), scan.to_json())
        .expect("legacy JSON index should be written");

    let output = run(&["make-report", path(&case_dir)]);
    assert_success(&output);
    let html = fs::read_to_string(case_dir.join("reports/case-report.html"))
        .expect("case report should be readable");
    assert!(html.contains("FT-SMALL"));
    assert!(html.contains("tiny-a.mp4"));
    assert!(html.contains("tiny-b.mov"));
    assert!(html.contains("색인 영상"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn make_report_uses_bounded_sqlite_summary_when_legacy_json_index_is_absent() {
    let root = unique_temp_dir("bounded-report-large");
    let case_dir = root.join("case");
    fs::create_dir_all(case_dir.join("db")).expect("db dir should be created");
    write_manifest(&case_dir, "FT-LARGE");
    let videos = (0..700)
        .map(|index| {
            video(
                &format!("vid_{index:06}"),
                &format!("/evidence/large/{index:06}.mp4"),
                1,
            )
        })
        .collect::<Vec<_>>();
    let scan = scan_result(videos);
    write_scan_index(&case_dir, &scan, &[]).expect("SQLite index should be written");

    let output = run(&["make-report", path(&case_dir)]);
    assert_success(&output);
    let html = fs::read_to_string(case_dir.join("reports/case-report.html"))
        .expect("case report should be readable");
    assert!(html.contains("FT-LARGE"));
    assert!(html.contains("700"));
    assert!(html.contains("bounded"));
    assert!(html.contains("inventory"));
    assert!(html.contains("000000.mp4"));
    assert!(!html.contains("000699.mp4"));
    assert!(
        html.matches("<tr>").count() < 200,
        "report should not embed every video row"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn make_report_rejects_legacy_json_only_case_with_migration_guidance() {
    let root = unique_temp_dir("bounded-report-legacy");
    let case_dir = root.join("case");
    fs::create_dir_all(case_dir.join("db")).expect("db dir should be created");
    write_manifest(&case_dir, "FT-LEGACY");
    fs::write(
        case_dir.join("db/video_index.json"),
        r#"{"schema_version":1,"video_count":1,"total_bytes":1,"warnings":[],"options":{"hash_files":false,"use_ffprobe":false,"max_depth":null},"videos":[]}"#,
    )
    .expect("legacy JSON index should be written");

    let output = run(&["make-report", path(&case_dir)]);
    assert_failure_contains(&output, "SQLite case db required");
    assert_failure_contains(&output, "scan-folder");
    assert!(!case_dir.join("reports/case-report.html").exists());

    let _ = fs::remove_dir_all(root);
}

fn write_manifest(case_dir: &Path, case_id: &str) {
    fs::write(
        case_dir.join("case.json"),
        format!(
            r#"{{
  "schema_version": 1,
  "case_id": "{case_id}",
  "title": "{case_id}",
  "created_unix": 1,
  "tool_name": "frametrace",
  "tool_version": "test",
  "platform": "test",
  "operator": null,
  "host": null,
  "device_id": null,
  "device_serial": null,
  "write_protect": null,
  "acquisition_tool": null,
  "evidence_hash": null,
  "notes": null
}}
"#
        ),
    )
    .expect("case manifest should be written");
}

fn scan_result(records: Vec<VideoRecord>) -> ScanResult {
    ScanResult {
        source_path: PathBuf::from("/evidence"),
        scanned_unix: 1,
        video_count: records.len(),
        total_bytes: records.iter().map(|record| record.size_bytes).sum(),
        warnings: Vec::new(),
        options: ScanOptions {
            hash_files: false,
            use_ffprobe: false,
            max_depth: None,
        },
        records,
    }
}

fn video(id: &str, source_path: &str, size_bytes: u64) -> VideoRecord {
    VideoRecord {
        id: id.to_string(),
        source_path: PathBuf::from(source_path),
        relative_path: PathBuf::from(source_path)
            .file_name()
            .expect("fixture path should have file name")
            .to_string_lossy()
            .to_string(),
        extension: PathBuf::from(source_path)
            .extension()
            .expect("fixture path should have extension")
            .to_string_lossy()
            .to_string(),
        size_bytes,
        modified_unix: Some(1),
        sha256: None,
        hash_status: "skipped".to_string(),
        probe: ProbeSummary::skipped(),
        confidence: "extension-candidate".to_string(),
        source_profile: SourceProfile::generic_media("test fixture"),
    }
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

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after UNIX epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("frametrace-{name}-{}-{nanos}", std::process::id()))
}

fn path(path: &Path) -> &str {
    path.to_str().expect("test paths should be UTF-8")
}
