use super::{DerivedArtifact, FrameCaptureOptions, ProxyOptions, ThumbnailOptions, ffmpeg};
use crate::tool_policy::{ResolvedExternalTool, resolve_external_tool};
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn default_proxy_is_review_sized() {
    assert_eq!(ProxyOptions::default().max_width, 1280);
}

#[test]
fn default_thumbnail_starts_at_zero() {
    assert_eq!(ThumbnailOptions::default().time_seconds, 0.0);
}

#[test]
fn frame_capture_args_record_exact_time() {
    let args = ffmpeg::frame_capture_args(
        Path::new("in.mp4"),
        Path::new("frame.jpg"),
        &FrameCaptureOptions {
            time_seconds: 3.5,
            ..FrameCaptureOptions::default()
        },
    );
    assert!(args.contains(&"-ss".to_string()));
    assert!(args.contains(&"3.500".to_string()));
    assert!(args.contains(&"-frames:v".to_string()));
    assert_eq!(args.last().map(String::as_str), Some("frame.jpg"));
}

#[test]
fn derived_artifact_log_body_records_provenance_operator_and_ids() {
    let artifact = DerivedArtifact {
        source_path: PathBuf::from("/case/source/clip.mp4"),
        output_path: PathBuf::from("/case/artifacts/proxies/clip_proxy.mp4"),
        kind: "proxy".to_string(),
        created_unix: 1_789_000_000,
        selector: "vid_000001".to_string(),
        operator: "qa-operator".to_string(),
    };
    let output_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let source_sha256 = Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

    let body = ffmpeg::log_body_json(
        &artifact,
        source_sha256,
        output_sha256,
        &["-i".to_string(), "clip.mp4".to_string()],
        &fake_tool(),
    );

    assert!(body.contains(r#""event":"make-proxy""#));
    assert!(body.contains(r#""artifact_state":"derived""#));
    assert!(body.contains(r#""operator":"qa-operator""#));
    assert!(body.contains(r#""method":"ffmpeg-proxy""#));
    assert!(body.contains(r#""tool":"ffmpeg""#));
    assert!(body.contains(r#""resolved_tool_path":"#));
    assert!(body.contains(r#""selector":"vid_000001""#));
    assert!(body.contains(r#""derived_artifact_id":"derived-proxy-bbbbbbbbbbbb""#));
    assert!(body.contains(r#""source_artifact_id":"source-vid_000001-aaaaaaaaaaaa""#));
    assert!(body.contains(
        r#""source_artifact_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa""#
    ));
    assert!(body.contains(
        r#""output_artifact_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb""#
    ));
}

#[test]
fn builds_artifact_command_args() {
    let proxy_args = ffmpeg::proxy_args(
        Path::new("in.mp4"),
        Path::new("proxy.mp4"),
        &ProxyOptions::default(),
    );
    assert!(proxy_args.contains(&"libx264".to_string()));
    assert_eq!(proxy_args.last().map(String::as_str), Some("proxy.mp4"));

    let thumb_args = ffmpeg::thumbnail_args(
        Path::new("in.mp4"),
        Path::new("thumb.jpg"),
        &ThumbnailOptions::default(),
    );
    assert!(thumb_args.contains(&"-frames:v".to_string()));
    assert_eq!(thumb_args.last().map(String::as_str), Some("thumb.jpg"));
}

fn fake_tool() -> ResolvedExternalTool {
    let path = test_executable_named("ffmpeg");
    let tool = resolve_external_tool(path.to_str().unwrap(), &["ffmpeg"], "-version").unwrap();
    if let Some(parent) = path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
    tool
}

fn test_executable_named(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!(
        "frametrace-artifact-tool-test-{}-{}-{name}",
        std::process::id(),
        unique_test_suffix()
    ));
    fs::create_dir_all(&base).unwrap();
    let path = base.join(executable_name(name));
    fs::copy(std::env::current_exe().unwrap(), &path).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
    }
    path
}

fn executable_name(name: &str) -> String {
    #[cfg(windows)]
    {
        format!("{name}.exe")
    }
    #[cfg(not(windows))]
    {
        name.to_string()
    }
}

fn unique_test_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}
