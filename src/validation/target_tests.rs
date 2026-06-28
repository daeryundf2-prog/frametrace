use super::target::{resolve_from_log, resolve_validation_target};
use crate::audit;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

#[test]
fn resolves_artifact_path_from_jsonl_log() {
    let dir = unique_temp_dir("log");
    fs::create_dir_all(&dir).unwrap();
    let log = dir.join("log.jsonl");
    audit::append_chained_jsonl(
        &log,
        r#"{"id":"carve_000001","output_path":"/tmp/out.mp4"}"#,
    )
    .unwrap();
    assert_eq!(
        resolve_from_log(&log, "carve_000001")
            .unwrap()
            .unwrap()
            .path
            .to_string_lossy(),
        "/tmp/out.mp4"
    );

    fs::remove_file(&log).unwrap();
    audit::append_chained_jsonl(
        &log,
        r#"{"derived_artifact_id":"derived-frame-capture-bbbbbbbbbbbb","output_artifact_path":"/tmp/frame.jpg"}"#,
    )
    .unwrap();
    assert_eq!(
        resolve_from_log(&log, "derived-frame-capture-bbbbbbbbbbbb")
            .unwrap()
            .unwrap()
            .path
            .to_string_lossy(),
        "/tmp/frame.jpg"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn rejects_poisoned_jsonl_records_with_provenance_error() {
    let dir = unique_temp_dir("poisoned-jsonl");
    let target = write_case_frame(&dir);
    let target_text = target.to_string_lossy().to_string();
    audit::append_chained_jsonl(
        &dir.join("artifacts/frames/frame-log.jsonl"),
        &json!({
            "derived_artifact_id": "derived-frame-capture-poison",
            "output_artifact_path": target_text,
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        dir.join("artifacts/frames/frame-log.jsonl"),
        format!(
            "{}\n{{\"derived_artifact_id\":\"derived-frame-capture-poison\",\"output_artifact_path\":\"/tmp/poison.jpg\"",
            fs::read_to_string(dir.join("artifacts/frames/frame-log.jsonl")).unwrap()
        ),
    )
    .unwrap();

    let err = resolve_validation_target(&dir, "derived-frame-capture-poison", false).unwrap_err();

    assert!(
        err.contains("audit") || err.contains("provenance"),
        "unexpected error: {err}"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn rejects_stale_audit_entry_when_signed_path_leaves_case() {
    let dir = unique_temp_dir("stale-audit");
    fs::create_dir_all(dir.join("artifacts/frames")).unwrap();
    let external = unique_temp_dir("stale-external");
    fs::create_dir_all(&external).unwrap();
    let external_target = external.join("frame.jpg");
    fs::write(&external_target, b"frame").unwrap();
    let external_target_text = external_target.to_string_lossy().to_string();
    audit::append_chained_jsonl(
        &dir.join("artifacts/frames/frame-log.jsonl"),
        &json!({
            "derived_artifact_id": "derived-frame-capture-stale",
            "output_artifact_path": external_target_text,
        })
        .to_string(),
    )
    .unwrap();

    let err = resolve_validation_target(&dir, "derived-frame-capture-stale", false).unwrap_err();

    assert!(err.contains("case directory"), "unexpected error: {err}");
    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(external);
}

#[test]
fn rejects_external_direct_path_by_default() {
    let dir = unique_temp_dir("direct-default");
    fs::create_dir_all(&dir).unwrap();
    let external = unique_temp_dir("direct-external");
    fs::create_dir_all(&external).unwrap();
    let target = external.join("external.mp4");
    fs::write(&target, b"video").unwrap();

    let err = resolve_validation_target(&dir, &target.to_string_lossy(), false).unwrap_err();

    assert!(err.contains("external-source"), "unexpected error: {err}");
    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(external);
}

#[test]
fn resolves_external_direct_path_when_explicitly_enabled() {
    let dir = unique_temp_dir("direct-explicit");
    fs::create_dir_all(&dir).unwrap();
    let external = unique_temp_dir("direct-explicit-external");
    fs::create_dir_all(&external).unwrap();
    let target = external.join("external.mp4");
    fs::write(&target, b"video").unwrap();

    let resolved = resolve_validation_target(&dir, &target.to_string_lossy(), true).unwrap();

    assert_eq!(resolved.path, target.canonicalize().unwrap());
    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(external);
}

#[test]
fn resolves_case_relative_direct_path_by_default() {
    let dir = unique_temp_dir("direct-case-relative");
    let target = write_case_frame(&dir);

    let resolved = resolve_validation_target(&dir, "artifacts/frames/frame.jpg", false).unwrap();

    assert_eq!(resolved.path, target.canonicalize().unwrap());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn resolves_case_contained_derived_artifact_with_verified_chain() {
    let dir = unique_temp_dir("contained-derived");
    let target = write_case_frame(&dir);
    let target_text = target.to_string_lossy().to_string();
    audit::append_chained_jsonl(
        &dir.join("artifacts/frames/frame-log.jsonl"),
        &json!({
            "derived_artifact_id": "derived-frame-capture-contained",
            "source_artifact_id": "source-vid_000001-aaaaaaaaaaaa",
            "source_artifact_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "output_artifact_path": target_text,
        })
        .to_string(),
    )
    .unwrap();

    let resolved =
        resolve_validation_target(&dir, "derived-frame-capture-contained", false).unwrap();

    assert_eq!(resolved.path, target.canonicalize().unwrap());
    assert_eq!(
        resolved.derived_artifact_id.as_deref(),
        Some("derived-frame-capture-contained")
    );
    let _ = fs::remove_dir_all(dir);
}

fn write_case_frame(dir: &std::path::Path) -> PathBuf {
    fs::create_dir_all(dir.join("artifacts/frames")).unwrap();
    let target = dir.join("artifacts/frames/frame.jpg");
    fs::write(&target, b"frame").unwrap();
    target
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("frametrace-validation-target-{name}-{nanos}"))
}
