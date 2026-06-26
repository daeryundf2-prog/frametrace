use frametrace::audit;
use frametrace::playback::{PlaybackConfirmationOptions, confirm_playback};
use frametrace::report::{ReportInputs, render_case_report};
use std::fs;
use std::path::PathBuf;

#[test]
fn report_discloses_derived_provenance_and_validation_failures() {
    let html = render_case_report(&ReportInputs {
        manifest_json: r#"{"case_id":"FT-MEDIA","title":"Media Contract"}"#,
        index_json: r#"{"videos":[],"warnings":[]}"#,
        export_log_jsonl: r#"{"event":"export-video","format":"mp4","operator":"qa-operator","method":"ffmpeg-clip-export","source_artifact_id":"source-vid_000001-aaaaaaaaaaaa","derived_artifact_id":"derived-clip-bbbbbbbbbbbb","source_artifact_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","output_artifact_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","source_path":"/case/source/clip.mp4","output_path":"/case/artifacts/clips/clip.mp4","entry_sha256":"chain"}"#,
        proxy_log_jsonl: r#"{"event":"make-proxy","kind":"proxy","operator":"qa-operator","method":"ffmpeg-proxy","source_artifact_id":"source-vid_000001-aaaaaaaaaaaa","derived_artifact_id":"derived-proxy-cccccccccccc","source_artifact_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","output_artifact_sha256":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc","source_path":"/case/source/clip.mp4","output_path":"/case/artifacts/proxies/clip.mp4","entry_sha256":"chain"}"#,
        thumbnail_log_jsonl: "",
        frame_log_jsonl: r#"{"event":"make-frame-capture","kind":"frame-capture","operator":"qa-operator","method":"ffmpeg-frame-capture","source_artifact_id":"source-vid_000001-aaaaaaaaaaaa","derived_artifact_id":"derived-frame-capture-dddddddddddd","output_artifact_sha256":"dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd","source_path":"/case/source/clip.mp4","output_path":"/case/artifacts/frames/frame.jpg","entry_sha256":"chain"}"#,
        carve_log_jsonl: "",
        filesystem_log_jsonl: "",
        validation_log_jsonl: r#"{"event":"validate-artifact","operator":"qa-operator","method":"ffprobe-container-video-stream","source_artifact_id":"source-carve_000001-dddddddddddd","target_path":"/case/artifacts/carved/bad.mp4","target_sha256":"dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd","validation_status":"validation-failed","validation_note":"ffprobe could not parse the file","entry_sha256":"chain"}"#,
        audit_chain_status_json: r#"[{"name":"validation","relative_path":"evidence/logs/validation-log.jsonl","status":"valid","entries":1,"last_entry_sha256":"chain","error":null}]"#,
    });

    assert!(html.contains("qa-operator"));
    assert!(html.contains("ffmpeg-clip-export"));
    assert!(html.contains("ffmpeg-proxy"));
    assert!(html.contains("ffmpeg-frame-capture"));
    assert!(html.contains("source-vid_000001-aaaaaaaaaaaa"));
    assert!(html.contains("derived-clip-bbbbbbbbbbbb"));
    assert!(html.contains("source_artifact_sha256"));
    assert!(html.contains("validation-failed"));
    assert!(html.contains("ffprobe could not parse the file"));
    assert!(html.contains("감사 체인 검증"));
    assert!(html.contains("valid"));
}

#[test]
fn playback_confirmation_requires_prior_ffprobe_validation_and_records_separate_state() {
    let case_dir = unique_temp_dir("playback-confirmation");
    fs::create_dir_all(case_dir.join("evidence/logs")).unwrap();
    audit::append_chained_jsonl(
        &case_dir.join("evidence/logs/validation-log.jsonl"),
        r#"{"schema_version":3,"event":"validate-artifact","selector":"vid_000001","operator":"qa-operator","method":"ffprobe-container-video-stream","source_artifact_id":"source-vid_000001-aaaaaaaaaaaa","source_artifact_path":"/case/source/clip.mp4","source_artifact_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","target_artifact_id":"source-vid_000001-aaaaaaaaaaaa","target_path":"/case/source/clip.mp4","target_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","validation_status":"ffprobe-video-stream-confirmed","validation_note":"ffprobe parsed a video stream"}"#,
    )
    .unwrap();

    let result = confirm_playback(
        &case_dir,
        "vid_000001",
        &PlaybackConfirmationOptions {
            operator: Some("qa-operator".to_string()),
            playback_tool: Some("Windows Media Player".to_string()),
            notes: Some("examiner observed normal playback".to_string()),
        },
    )
    .unwrap();

    assert_eq!(result.validation_status, "playback-confirmed");
    assert_eq!(
        result.prior_validation_status,
        "ffprobe-video-stream-confirmed"
    );
    let log = fs::read_to_string(case_dir.join("evidence/logs/validation-log.jsonl")).unwrap();
    assert!(log.contains(r#""validation_status":"ffprobe-video-stream-confirmed""#));
    assert!(log.contains(r#""validation_status":"playback-confirmed""#));
    assert!(log.contains(r#""method":"manual-playback-confirmation""#));
    assert!(log.contains(r#""playback_verified":true"#));
    assert!(log.contains(r#""playback_tool":"Windows Media Player""#));

    let _ = fs::remove_dir_all(case_dir);
}

#[test]
fn playback_confirmation_rejects_missing_ffprobe_validation() {
    let case_dir = unique_temp_dir("playback-missing-validation");
    fs::create_dir_all(case_dir.join("evidence/logs")).unwrap();

    let err = confirm_playback(
        &case_dir,
        "vid_000001",
        &PlaybackConfirmationOptions {
            operator: Some("qa-operator".to_string()),
            playback_tool: Some("Windows Media Player".to_string()),
            notes: None,
        },
    )
    .unwrap_err();

    assert!(err.contains("requires prior ffprobe-video-stream-confirmed validation"));
    let _ = fs::remove_dir_all(case_dir);
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("frametrace-{name}-{}-{nanos}", std::process::id()))
}
