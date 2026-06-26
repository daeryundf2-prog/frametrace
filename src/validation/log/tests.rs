use super::{validation_log_body_json, validation_status};
use crate::model::ProbeSummary;
use crate::tool_policy::{ResolvedExternalTool, resolve_external_tool};
use crate::validation::{ValidationOptions, ValidationResult};
use std::fs;
use std::path::PathBuf;

#[test]
fn classifies_successful_video_probe_as_ffprobe_confirmed() {
    let probe = ProbeSummary {
        ok: true,
        raw_json: Some("{}".to_string()),
        error: None,
        duration_seconds: Some(1.0),
        format_name: Some("mov,mp4".to_string()),
        video_codec: Some("h264".to_string()),
        audio_codec: None,
        width: Some(1920),
        height: Some(1080),
    };
    assert_eq!(
        validation_status(&probe).0,
        "ffprobe-video-stream-confirmed"
    );
}

#[test]
fn classifies_missing_video_stream_as_failed() {
    let probe = ProbeSummary {
        ok: true,
        raw_json: Some("{}".to_string()),
        error: None,
        duration_seconds: Some(1.0),
        format_name: Some("mp3".to_string()),
        video_codec: None,
        audio_codec: Some("mp3".to_string()),
        width: None,
        height: None,
    };
    assert_eq!(validation_status(&probe).0, "validation-failed");
}

#[test]
fn validation_log_body_records_forensic_promotion_contract() {
    let result = ValidationResult {
        selector: "carve_000001".to_string(),
        target_path: PathBuf::from("/case/artifacts/carved/carve_000001.mp4"),
        target_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .to_string(),
        source_artifact_id: None,
        source_artifact_path: None,
        source_artifact_sha256: None,
        derived_artifact_id: None,
        target_artifact_id: None,
        validation_status: "ffprobe-video-stream-confirmed".to_string(),
        validation_note: "ffprobe parsed a video stream".to_string(),
        probe: ProbeSummary {
            ok: true,
            raw_json: Some("{}".to_string()),
            error: None,
            duration_seconds: Some(12.0),
            format_name: Some("mov,mp4".to_string()),
            video_codec: Some("h264".to_string()),
            audio_codec: None,
            width: Some(1920),
            height: Some(1080),
        },
        validated_unix: 1_789_000_000,
        ffprobe_tool: fake_ffprobe_tool(),
    };
    let options = ValidationOptions {
        ffprobe_bin: "ffprobe".to_string(),
        operator: Some("qa-operator".to_string()),
        allow_external_source: false,
    };

    let body = validation_log_body_json(&result, &options, "qa-operator");

    assert!(body.contains(r#""event":"validate-artifact""#));
    assert!(body.contains(r#""operator":"qa-operator""#));
    assert!(body.contains(r#""method":"ffprobe-container-video-stream""#));
    assert!(body.contains(r#""tool":"ffprobe""#));
    assert!(body.contains(r#""tool_version":"#));
    assert!(body.contains(
        r#""command_args":["-v","error","-print_format","json","-show_format","-show_streams""#
    ));
    assert!(body.contains(r#""source_artifact_id":"source-carve_000001-aaaaaaaaaaaa""#));
    assert!(body.contains(
        r#""source_artifact_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa""#
    ));
    assert!(body.contains(r#""validation_artifact_path":"evidence/logs/validation-log.jsonl""#));
}

#[test]
fn validation_log_preserves_source_relation_for_derived_artifact_targets() {
    let result = ValidationResult {
        selector: "derived-frame-capture-bbbbbbbbbbbb".to_string(),
        target_path: PathBuf::from("/case/artifacts/frames/frame.jpg"),
        target_sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            .to_string(),
        source_artifact_id: Some("source-vid_000001-aaaaaaaaaaaa".to_string()),
        source_artifact_path: Some(PathBuf::from("/case/source/original.mp4")),
        source_artifact_sha256: Some(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        ),
        derived_artifact_id: Some("derived-frame-capture-bbbbbbbbbbbb".to_string()),
        target_artifact_id: Some("derived-frame-capture-bbbbbbbbbbbb".to_string()),
        validation_status: "ffprobe-video-stream-confirmed".to_string(),
        validation_note: "ffprobe parsed a video stream".to_string(),
        probe: ProbeSummary {
            ok: true,
            raw_json: Some("{}".to_string()),
            error: None,
            duration_seconds: Some(1.0),
            format_name: Some("image2".to_string()),
            video_codec: Some("mjpeg".to_string()),
            audio_codec: None,
            width: Some(640),
            height: Some(360),
        },
        validated_unix: 1_789_000_001,
        ffprobe_tool: fake_ffprobe_tool(),
    };
    let options = ValidationOptions {
        ffprobe_bin: "ffprobe".to_string(),
        operator: Some("qa-operator".to_string()),
        allow_external_source: false,
    };

    let body = validation_log_body_json(&result, &options, "qa-operator");

    assert!(body.contains(r#""source_artifact_id":"source-vid_000001-aaaaaaaaaaaa""#));
    assert!(body.contains(r#""source_artifact_path":"/case/source/original.mp4""#));
    assert!(body.contains(
        r#""source_artifact_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa""#
    ));
    assert!(body.contains(r#""derived_artifact_id":"derived-frame-capture-bbbbbbbbbbbb""#));
    assert!(body.contains(r#""target_artifact_id":"derived-frame-capture-bbbbbbbbbbbb""#));
}

fn fake_ffprobe_tool() -> ResolvedExternalTool {
    let path = test_executable_named("ffprobe");
    let tool = resolve_external_tool(path.to_str().unwrap(), &["ffprobe"], "-version").unwrap();
    if let Some(parent) = path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
    tool
}

fn test_executable_named(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!(
        "frametrace-validation-tool-test-{}-{}-{name}",
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
