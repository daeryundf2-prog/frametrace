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
fn make_review_embeds_bounded_inventory_subset() {
    let root = unique_temp_dir("cli-review-bounded");
    let case_dir = root.join("case");
    assert_success(&run(&[
        "init-case",
        path(&case_dir),
        "--title",
        "Bounded Review Case",
    ]));

    let rows = (0..502)
        .map(|index| {
            format!(
                r#"{{"id":"vid_{index:06}","source_path":"/evidence/{index}.mp4","relative_path":"{index}.mp4"}}"#
            )
        })
        .collect::<Vec<_>>();
    fs::write(case_dir.join("db/videos.jsonl"), rows.join("\n"))
        .expect("video jsonl should be written");
    fs::write(
        case_dir.join("db/video_index.json"),
        format!(
            r#"{{"schema_version":1,"source_path":"/evidence","video_count":502,"videos":[{}]}}"#,
            rows.join(",")
        ),
    )
    .expect("legacy video index should be written");

    assert_success(&run(&["make-review", path(&case_dir)]));

    let html = fs::read_to_string(case_dir.join("review/evidence-viewer.html"))
        .expect("evidence viewer html should be written");
    assert!(html.contains("\"video_count\":502"));
    assert!(html.contains("\"embedded_video_count\":500"));
    assert!(html.contains("\"inventory_truncated\":true"));
    assert!(html.contains("Review HTML embeds 500 of 502 rows"));
    assert!(html.contains("\"id\":\"vid_000499\""));
    assert!(!html.contains("\"id\":\"vid_000500\""));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn make_review_uses_sqlite_inventory_when_jsonl_is_absent() {
    let root = unique_temp_dir("cli-review-sqlite");
    let case_dir = root.join("case");
    let media_dir = root.join("media");
    fs::create_dir_all(&media_dir).expect("media dir should be created");
    fs::write(media_dir.join("clip.mp4"), b"\0\0\0\x18ftypmp42payload")
        .expect("fixture video should be written");

    assert_success(&run(&[
        "init-case",
        path(&case_dir),
        "--title",
        "SQLite Review Case",
    ]));
    assert_success(&run(&[
        "scan-folder",
        path(&case_dir),
        path(&media_dir),
        "--no-ffprobe",
    ]));
    fs::remove_file(case_dir.join("db/videos.jsonl")).expect("jsonl should be removable");

    assert_success(&run(&["make-review", path(&case_dir)]));

    let html = fs::read_to_string(case_dir.join("review/evidence-viewer.html"))
        .expect("evidence viewer html should be written");
    assert!(html.contains("\"inventory_source\":\"case.db/videos\""));
    assert!(html.contains("\"video_count\":1"));
    assert!(html.contains("\"id\":\"vid_000001\""));
    assert!(html.contains("clip.mp4"));

    let _ = fs::remove_dir_all(root);
}

fn path(path: &Path) -> &str {
    path.to_str().expect("test paths should be UTF-8")
}
