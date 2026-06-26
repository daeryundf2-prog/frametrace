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

fn run_with_path(args: &[&str], path_env: &str) -> Output {
    Command::new(frametrace())
        .args(args)
        .env("PATH", path_env)
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
fn export_video_rejects_symlinked_default_clip_directory_without_writing_outside() {
    let root = unique_temp_dir("cli-export-default-dir-symlink");
    let case_dir = root.join("case");
    let media_dir = root.join("media");
    let fake_bin = root.join("fake-bin");
    seed_indexed_case(&case_dir, &media_dir);
    write_fake_ffmpeg(&fake_bin);
    fs::remove_dir_all(case_dir.join("artifacts/clips")).expect("clips dir should be removed");
    let outside = root.join("outside-clips");
    fs::create_dir_all(&outside).expect("outside clips dir should exist");
    symlink(&outside, case_dir.join("artifacts/clips")).expect("symlink should be created");

    let output = run_with_path(
        &[
            "export-video",
            path(&case_dir),
            "vid_000001",
            "--format",
            "mp4",
        ],
        &test_path_with_fake_bin(&fake_bin),
    );

    assert_failure_contains(&output, "inside the case directory");
    assert_no_outside_files(&outside);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn derived_media_commands_reject_symlinked_default_directories_without_writing_outside() {
    let root = unique_temp_dir("cli-derived-default-dir-symlink");
    let case_dir = root.join("case");
    let media_dir = root.join("media");
    let fake_bin = root.join("fake-bin");
    seed_indexed_case(&case_dir, &media_dir);
    write_fake_ffmpeg(&fake_bin);

    for (command, rel_dir) in [
        ("make-proxy", "artifacts/proxies"),
        ("make-thumbnail", "artifacts/thumbnails"),
        ("capture-frame", "artifacts/frames"),
    ] {
        let case_artifact_dir = case_dir.join(rel_dir);
        let outside = root.join(format!("outside-{command}"));
        fs::remove_dir_all(&case_artifact_dir).expect("artifact dir should be removed");
        fs::create_dir_all(&outside).expect("outside artifact dir should exist");
        symlink(&outside, &case_artifact_dir).expect("symlink should be created");

        let output = run_with_path(
            &[command, path(&case_dir), "vid_000001"],
            &test_path_with_fake_bin(&fake_bin),
        );

        assert_failure_contains(&output, "inside the case directory");
        assert_no_outside_files(&outside);
        fs::remove_file(&case_artifact_dir).expect("symlink should be removed");
        fs::create_dir_all(&case_artifact_dir).expect("artifact dir should be restored");
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn derived_media_commands_require_policy_approved_ffmpeg_and_log_resolved_tool_metadata() {
    let root = unique_temp_dir("cli-derived-tool-policy");
    let case_dir = root.join("case");
    let media_dir = root.join("media");
    let fake_bin = root.join("fake-bin");
    seed_indexed_case(&case_dir, &media_dir);
    write_fake_ffmpeg(&fake_bin);
    write_executable(
        &fake_bin.join("fake-ffmpeg"),
        "#!/bin/sh\nprintf 'should not run' > \"$2\"\n",
    );
    let approved_ffmpeg = fake_bin.join("ffmpeg");
    let approved_ffmpeg = approved_ffmpeg
        .canonicalize()
        .expect("approved fake ffmpeg should canonicalize");

    for (command, log_path) in [
        ("make-proxy", "artifacts/proxies/proxy-log.jsonl"),
        ("make-thumbnail", "artifacts/thumbnails/thumbnail-log.jsonl"),
        ("capture-frame", "artifacts/frames/frame-log.jsonl"),
    ] {
        let output = run(&[
            command,
            path(&case_dir),
            "vid_000001",
            "--ffmpeg",
            path(&approved_ffmpeg),
            "--operator",
            "qa-tool-policy",
        ]);
        assert_success(&output);
        let log = fs::read_to_string(case_dir.join(log_path)).expect("derived log should exist");
        assert!(log.contains(&format!(
            r#""resolved_tool_path":"{}""#,
            approved_ffmpeg.display()
        )));
        assert!(log.contains(r#""tool_version":"ffmpeg fake 1.0""#));
        assert!(log.contains(r#""command_args":["#));
        assert!(log.contains(r#""operator":"qa-tool-policy""#));
        assert!(log.contains(r#""output_artifact_sha256":"#));
        assert!(log.contains(r#""entry_sha256":"#));
    }

    let disallowed_output = case_dir.join("artifacts/clips/rejected.mp4");
    let rejected = run(&[
        "export-video",
        path(&case_dir),
        "vid_000001",
        "--format",
        "mp4",
        "--ffmpeg",
        path(&fake_bin.join("fake-ffmpeg")),
        "--output",
        path(&disallowed_output),
    ]);

    assert_failure_contains(&rejected, "unsupported tool binary");
    assert!(
        !disallowed_output.exists(),
        "rejected output must not be written"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn carve_file_rejects_symlinked_default_carved_directory_without_writing_outside() {
    let root = unique_temp_dir("cli-carve-default-dir-symlink");
    let case_dir = root.join("case");
    let source_dir = root.join("source-evidence");
    fs::create_dir_all(&source_dir).expect("source evidence dir should exist");
    let source_file = source_dir.join("image.bin");
    fs::write(&source_file, b"\0\0\0\x18ftypmp42payload").expect("source image should be written");
    init_case(&case_dir);
    fs::remove_dir_all(case_dir.join("artifacts/carved")).expect("carved dir should be removed");
    symlink(&source_dir, case_dir.join("artifacts/carved")).expect("symlink should be created");

    let output = run(&["carve-file", path(&case_dir), path(&source_file)]);

    assert_failure_contains(&output, "inside the case directory");
    assert!(!source_dir.join("carve_000001_000000000000.mp4").exists());
    assert!(!source_dir.join("carve-log.jsonl").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn package_case_rejects_symlinked_default_reports_directory_without_writing_outside() {
    let root = unique_temp_dir("cli-package-default-dir-symlink");
    let case_dir = root.join("case");
    init_case(&case_dir);
    seed_minimal_package_case(&case_dir);
    fs::remove_dir_all(case_dir.join("reports")).expect("reports dir should be removed");
    let outside = root.join("outside-reports");
    fs::create_dir_all(&outside).expect("outside reports dir should exist");
    symlink(&outside, case_dir.join("reports")).expect("symlink should be created");

    let output = run(&["package-case", path(&case_dir)]);

    assert_failure_contains(&output, "inside the case directory");
    assert_no_outside_files(&outside);
    let _ = fs::remove_dir_all(root);
}

fn seed_indexed_case(case_dir: &Path, media_dir: &Path) {
    fs::create_dir_all(media_dir).expect("media dir should exist");
    fs::write(media_dir.join("clip.mp4"), b"\0\0\0\x18ftypmp42payload")
        .expect("fixture video should be written");
    init_case(case_dir);
    assert_success(&run(&[
        "scan-folder",
        path(case_dir),
        path(media_dir),
        "--no-ffprobe",
    ]));
}

fn init_case(case_dir: &Path) {
    assert_success(&run(&[
        "init-case",
        path(case_dir),
        "--title",
        "Output Policy",
    ]));
}

fn write_fake_ffmpeg(fake_bin: &Path) {
    fs::create_dir_all(fake_bin).expect("fake bin should be created");
    write_executable(
        &fake_bin.join("ffmpeg"),
        "#!/bin/sh\ncase \"$1\" in -version|--version) echo 'ffmpeg fake 1.0'; exit 0 ;; esac\nout=\"\"\nfor arg in \"$@\"; do out=\"$arg\"; done\ncase \"$out\" in ''|-*) echo 'missing output' >&2; exit 2 ;; esac\nprintf 'derived media' > \"$out\"\n",
    );
}

fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).expect("fake executable should be written");
    let mut permissions = fs::metadata(path)
        .expect("fake executable metadata should exist")
        .permissions();
    use std::os::unix::fs::PermissionsExt;
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("fake executable should be executable");
}

fn test_path_with_fake_bin(fake_bin: &Path) -> String {
    let current = std::env::var_os("PATH")
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    format!("{}:{current}", fake_bin.display())
}

fn assert_no_outside_files(outside: &Path) {
    let mut entries = fs::read_dir(outside)
        .expect("outside dir should be readable")
        .collect::<Result<Vec<_>, _>>()
        .expect("outside entries should be readable");
    entries.sort_by_key(|entry| entry.path());
    assert!(
        entries.is_empty(),
        "outside directory should remain empty: {:?}",
        entries.iter().map(|entry| entry.path()).collect::<Vec<_>>()
    );
}

fn seed_minimal_package_case(case_dir: &Path) {
    fs::create_dir_all(case_dir.join("db")).expect("db dir should exist");
    fs::write(case_dir.join("db/case.db"), b"sqlite placeholder").expect("db should be written");
    fs::write(case_dir.join("db/video_index.json"), b"{}").expect("index should be written");
    fs::write(case_dir.join("db/videos.jsonl"), b"").expect("jsonl should be written");
    fs::write(case_dir.join("db/video_paths.tsv"), b"id\tsource_path\n")
        .expect("paths should be written");
}

fn path(path: &Path) -> &str {
    path.to_str().expect("test paths should be UTF-8")
}
