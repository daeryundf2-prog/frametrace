use super::super::accuracy_report;
use super::helpers::read_json;
use serde_json::{Value, json};
use std::fs;

#[test]
fn accuracy_report_passes_for_matching_manifest() {
    let root = std::env::temp_dir().join(format!("frametrace-qa-test-{}", std::process::id()));
    let case_dir = root.join("case");
    let output_dir = root.join("qa");
    let manifest = root.join("corpus.tsv");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(case_dir.join("db")).unwrap();
    let source_path = "/evidence/a.mp4";
    fs::write(
        case_dir.join("db/videos.jsonl"),
        format!(
            "{{\"source_path\":\"{}\",\"sha256\":\"abc\"}}\n",
            source_path
        ),
    )
    .unwrap();
    fs::write(&manifest, format!("{source_path}\tabc\n")).unwrap();

    let report = accuracy_report(&case_dir, &manifest, &output_dir).unwrap();
    assert!(report.passed);
    assert!(report.report_path.is_file());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn accuracy_report_includes_recovery_artifacts() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-qa-recovery-test-{}",
        std::process::id()
    ));
    let case_dir = root.join("case");
    let output_dir = root.join("qa");
    let manifest = root.join("corpus.tsv");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(case_dir.join("db")).unwrap();
    fs::create_dir_all(case_dir.join("artifacts/carved")).unwrap();
    fs::create_dir_all(case_dir.join("evidence/logs")).unwrap();
    fs::write(case_dir.join("db/videos.jsonl"), "").unwrap();
    let carved_path = root.join("case/artifacts/carved/carve_000001.mp4");
    fs::write(
        case_dir.join("artifacts/carved/carve-log.jsonl"),
        format!(
            "{{\"id\":\"carve_000001\",\"output_path\":\"{}\",\"sha256\":\"abc\",\"validation_status\":\"candidate-unvalidated\"}}\n",
            carved_path.display()
        ),
    )
    .unwrap();
    fs::write(
        case_dir.join("evidence/logs/validation-log.jsonl"),
        format!(
            "{{\"selector\":\"carve_000001\",\"target_path\":\"{}\",\"target_sha256\":\"abc\",\"validation_status\":\"ffprobe-video-stream-confirmed\"}}\n",
            carved_path.display()
        ),
    )
    .unwrap();
    fs::write(&manifest, format!("{}\tabc\n", carved_path.display())).unwrap();

    let report = accuracy_report(&case_dir, &manifest, &output_dir).unwrap();
    assert!(report.passed);
    assert!(report.report_path.is_file());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn accuracy_report_accepts_typed_corpus_manifest_and_records_metric_shape() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-qa-typed-corpus-test-{}",
        std::process::id()
    ));
    let case_dir = root.join("case");
    let output_dir = root.join("qa");
    let manifest = root.join("corpus.json");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(case_dir.join("db")).unwrap();
    fs::write(
        case_dir.join("db/videos.jsonl"),
        "{\"source_path\":\"/non-client/synthetic/video-a.mp4\",\"sha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}\n",
    )
    .unwrap();
    fs::write(
        &manifest,
        typed_manifest_text(
            "/non-client/synthetic/video-a.mp4",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
    )
    .unwrap();

    let report = accuracy_report(&case_dir, &manifest, &output_dir).unwrap();
    let json = read_json(&report.report_path);

    assert!(report.passed);
    assert_eq!(json["precision"], 1.0);
    assert_eq!(json["recall"], 1.0);
    assert_eq!(json["false_positives"], 0);
    assert_eq!(json["false_negatives"], 0);
    assert_eq!(
        json["expected"][0]["source_sha256"],
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    assert_eq!(
        json["expected"][0]["ground_truth"]["expected_state"],
        "ffprobe-video-stream-confirmed"
    );
    assert_eq!(
        json["domains"][1]["status"], "unsupported",
        "unsupported domains must be recorded as unsupported, not pass"
    );
    assert_eq!(json["release_keys"]["mixed_real_world_like"], "unsupported");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn accuracy_report_rejects_manifest_missing_required_ground_truth_contract_fields() {
    for field in [
        "source_artifact_id",
        "expected_hash",
        "negative_controls",
        "notes",
    ] {
        let root = std::env::temp_dir().join(format!(
            "frametrace-qa-missing-{field}-test-{}",
            std::process::id()
        ));
        let case_dir = root.join("case");
        let output_dir = root.join("qa");
        let manifest = root.join("corpus.json");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(case_dir.join("db")).unwrap();
        fs::write(
            case_dir.join("db/videos.jsonl"),
            "{\"source_path\":\"/non-client/synthetic/video-a.mp4\",\"sha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}\n",
        )
        .unwrap();
        let mut value = typed_manifest_value(
            "/non-client/synthetic/video-a.mp4",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        value["cases"][0]["ground_truth"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        fs::write(&manifest, serde_json::to_string_pretty(&value).unwrap()).unwrap();

        let err = accuracy_report(&case_dir, &manifest, &output_dir).unwrap_err();
        assert!(
            err.contains(field),
            "expected missing {field} error, got {err}"
        );

        let _ = fs::remove_dir_all(root);
    }
}

#[test]
fn accuracy_report_rejects_synthetic_only_mixed_real_world_release_key() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-qa-synthetic-release-key-test-{}",
        std::process::id()
    ));
    let case_dir = root.join("case");
    let output_dir = root.join("qa");
    let manifest = root.join("corpus.json");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(case_dir.join("db")).unwrap();
    fs::write(case_dir.join("db/videos.jsonl"), "").unwrap();
    fs::write(
        &manifest,
        serde_json::to_string_pretty(&typed_manifest_value_with_release_key("pass")).unwrap(),
    )
    .unwrap();

    let err = accuracy_report(&case_dir, &manifest, &output_dir).unwrap_err();
    assert!(err.contains("mixed_real_world_like"));
    assert!(err.contains("synthetic"));

    let _ = fs::remove_dir_all(root);
}

fn typed_manifest_text(source_path: &str, source_sha256: &str) -> String {
    serde_json::to_string_pretty(&typed_manifest_value(source_path, source_sha256)).unwrap()
}

fn typed_manifest_value(source_path: &str, source_sha256: &str) -> Value {
    json!({
      "schema_version": 1,
      "corpus_id": "synthetic-video-corpus",
      "corpus_kind": "synthetic",
      "release_keys": {
        "mixed_real_world_like": "unsupported"
      },
      "domains": [
        {
          "key": "video_recovery",
          "status": "supported",
          "ground_truth_schema": required_ground_truth_fields(),
          "expected_outputs_schema": ["db/videos.jsonl"]
        },
        {
          "key": "browser_artifacts",
          "status": "unsupported",
          "reason": "parser PRD is not approved for this release"
        }
      ],
      "cases": [
        {
          "case_id": "SYN-VID-001",
          "domain": "video_recovery",
          "source_path": source_path,
          "source_sha256": source_sha256,
          "ground_truth": {
            "corpus_id": "synthetic-video-corpus",
            "source_artifact_id": "source-video-a",
            "source_sha256": source_sha256,
            "expected_artifact_type": "source-video",
            "expected_path_pattern": source_path,
            "expected_hash": source_sha256,
            "expected_timestamp_range": {
              "start_unix": 1782470000,
              "end_unix": 1782470003
            },
            "expected_state": "ffprobe-video-stream-confirmed",
            "negative_controls": ["/non-client/synthetic/not-video.txt"],
            "notes": "non-client synthetic fixture"
          },
          "expected_outputs": {
            "indexed": true,
            "validation_status": "ffprobe-video-stream-confirmed"
          }
        }
      ],
      "external_references": []
    })
}

fn typed_manifest_value_with_release_key(value: &str) -> Value {
    let mut manifest = typed_manifest_value(
        "/non-client/synthetic/video-a.mp4",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    manifest["corpus_id"] = json!("synthetic-only");
    manifest["release_keys"] = json!({"mixed_real_world_like": value});
    manifest["cases"] = json!([]);
    manifest
}

fn required_ground_truth_fields() -> Vec<&'static str> {
    vec![
        "corpus_id",
        "source_artifact_id",
        "source_sha256",
        "expected_artifact_type",
        "expected_path_pattern",
        "expected_hash",
        "expected_timestamp_range",
        "expected_state",
        "negative_controls",
        "notes",
    ]
}
