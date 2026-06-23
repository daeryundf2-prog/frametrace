#![cfg(unix)]

use crate::artifacts::{
    FrameCaptureOptions, ProxyOptions, ThumbnailOptions, capture_frame, generate_proxy,
    generate_thumbnail,
};
use crate::tsk::{TskRecoverOptions, recover_inode};
use crate::video_export::{ExportFormat, ExportOptions, export_video};
use std::fmt::Debug;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_root(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("frametrace-{name}-{}-{nanos}", std::process::id()))
}

fn case_with_source(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = temp_root(name);
    let case_dir = root.join("case");
    fs::create_dir_all(&case_dir).unwrap();
    let source = root.join("source.mp4");
    fs::write(&source, b"synthetic source").unwrap();
    (root, case_dir, source)
}

fn dangling_case_output(case_dir: &Path, file_name: &str) -> (PathBuf, PathBuf) {
    let output = case_dir.join("artifacts").join(file_name);
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    let outside = case_dir
        .parent()
        .unwrap()
        .join(format!("{file_name}.outside"));
    let _ = fs::remove_file(&outside);
    symlink(&outside, &output).unwrap();
    (output, outside)
}

fn source_selector(source: &Path) -> String {
    source.to_string_lossy().to_string()
}

fn assert_symlink_rejected<T: Debug>(result: Result<T, String>, outside: &Path, root: &Path) {
    let outside_created = outside.exists();
    let _ = fs::remove_dir_all(root);
    match result {
        Ok(value) => panic!("expected dangling symlink output rejection, got {value:?}"),
        Err(err) => {
            assert!(
                err.contains("cannot be a symlink"),
                "expected symlink rejection, got: {err}"
            );
            assert!(
                !outside_created,
                "dangling symlink target was created before rejection: {}",
                outside.display()
            );
        }
    }
}

#[test]
fn proxy_rejects_dangling_symlink_output_before_ffmpeg() {
    let (root, case_dir, source) = case_with_source("proxy-symlink");
    let (output, outside) = dangling_case_output(&case_dir, "proxy.mp4");

    let result = generate_proxy(
        &case_dir,
        &source_selector(&source),
        &ProxyOptions {
            output_path: Some(output),
            max_width: 640,
            operator: Some("qa".to_string()),
        },
    );

    assert_symlink_rejected(result, &outside, &root);
}

#[test]
fn thumbnail_rejects_dangling_symlink_output_before_ffmpeg() {
    let (root, case_dir, source) = case_with_source("thumbnail-symlink");
    let (output, outside) = dangling_case_output(&case_dir, "thumbnail.jpg");

    let result = generate_thumbnail(
        &case_dir,
        &source_selector(&source),
        &ThumbnailOptions {
            output_path: Some(output),
            time_seconds: 0.0,
            operator: Some("qa".to_string()),
        },
    );

    assert_symlink_rejected(result, &outside, &root);
}

#[test]
fn frame_capture_rejects_dangling_symlink_output_before_ffmpeg() {
    let (root, case_dir, source) = case_with_source("frame-symlink");
    let (output, outside) = dangling_case_output(&case_dir, "frame.jpg");

    let result = capture_frame(
        &case_dir,
        &source_selector(&source),
        &FrameCaptureOptions {
            output_path: Some(output),
            time_seconds: 0.0,
            operator: Some("qa".to_string()),
        },
    );

    assert_symlink_rejected(result, &outside, &root);
}

#[test]
fn video_export_rejects_dangling_symlink_output_before_ffmpeg() {
    let (root, case_dir, source) = case_with_source("video-export-symlink");
    let (output, outside) = dangling_case_output(&case_dir, "clip.mp4");

    let result = export_video(
        &case_dir,
        &source_selector(&source),
        &ExportOptions {
            format: ExportFormat::Mp4,
            start_seconds: None,
            duration_seconds: None,
            output_path: Some(output),
            operator: Some("qa".to_string()),
        },
    );

    assert_symlink_rejected(result, &outside, &root);
}

#[test]
fn inode_recovery_rejects_dangling_symlink_output_before_icat() {
    let (root, case_dir, source) = case_with_source("inode-recovery-symlink");
    let (output, outside) = dangling_case_output(&case_dir, "inode.bin");

    let result = recover_inode(
        &case_dir,
        &source,
        &TskRecoverOptions {
            partition_offset: 0,
            inode: "128".to_string(),
            output_path: Some(output),
            recover_deleted: false,
            include_slack: false,
            skip_sparse_holes: false,
            icat_bin: "icat".to_string(),
        },
    );

    assert_symlink_rejected(result, &outside, &root);
}
