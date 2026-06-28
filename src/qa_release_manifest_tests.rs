use super::read_review_manifest;
use std::fs;

#[test]
fn review_manifest_rejects_broad_text_only_gates() {
    let root = std::env::temp_dir().join(format!("frametrace-review-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let manifest = root.join("release-review.txt");
    fs::write(
        &manifest,
        "\
technical_review=pass
security review: approved
- [x] Migration Validation
- [x] Operator Review
legal-review=done
",
    )
    .unwrap();

    let err = read_review_manifest(&manifest).unwrap_err();
    assert!(err.contains("typed JSON review manifest"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn review_manifest_rejects_done_status_without_typed_pass() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-review-done-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let artifact = root.join("technical-review.json");
    fs::write(&artifact, "{}").unwrap();
    let manifest = root.join("release-review.json");
    fs::write(
        &manifest,
        format!(
            r#"{{
  "schema_version": 1,
  "gates": [
    {{
      "key": "technical_review",
      "status": "done",
      "artifact_path": "{}",
      "tool": "manual-review-recorder",
      "evidence": "manual review artifact",
      "timestamp": "2026-06-24T00:00:00Z",
      "reviewer": "qa",
      "cleanup_status": "clean"
    }}
  ]
}}"#,
            artifact.display()
        ),
    )
    .unwrap();

    let err = read_review_manifest(&manifest).unwrap_err();
    assert!(err.contains("technical_review"));
    assert!(err.contains("status"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn review_manifest_rejects_missing_artifact_path() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-review-artifact-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let manifest = root.join("release-review.json");
    fs::write(
        &manifest,
        r#"{
  "schema_version": 1,
  "gates": [
    {
      "key": "technical_review",
      "status": "PASS",
      "artifact_path": "missing-review.json",
      "tool": "manual-review-recorder",
      "evidence": "manual review artifact",
      "timestamp": "2026-06-24T00:00:00Z",
      "reviewer": "qa",
      "cleanup_status": "clean"
    }
  ]
}"#,
    )
    .unwrap();

    let err = read_review_manifest(&manifest).unwrap_err();
    assert!(err.contains("technical_review"));
    assert!(err.contains("artifact_path"));
    assert!(err.contains("does not exist"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn review_manifest_rejects_missing_cleanup_or_reviewer_metadata() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-review-metadata-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("technical-review.json"), "{}").unwrap();
    let manifest = root.join("release-review.json");
    fs::write(
        &manifest,
        r#"{
  "schema_version": 1,
  "gates": [
    {
      "key": "technical_review",
      "status": "PASS",
      "artifact_path": "technical-review.json",
      "tool": "manual-review-recorder",
      "evidence": "manual review artifact",
      "timestamp": "2026-06-24T00:00:00Z",
      "cleanup_status": ""
    }
  ]
}"#,
    )
    .unwrap();

    let err = read_review_manifest(&manifest).unwrap_err();
    assert!(err.contains("technical_review"));
    assert!(err.contains("cleanup_status"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn review_manifest_accepts_typed_artifact_backed_pass() {
    let root = std::env::temp_dir().join(format!(
        "frametrace-review-pass-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("technical-review.json"), "{}").unwrap();
    let manifest = root.join("release-review.json");
    fs::write(
        &manifest,
        r#"{
  "schema_version": 1,
  "gates": [
    {
      "key": "technical_review",
      "status": "PASS",
      "artifact_path": "technical-review.json",
      "tool": "manual-review-recorder",
      "evidence": "manual review artifact",
      "timestamp": "2026-06-24T00:00:00Z",
      "reviewer": "qa",
      "cleanup_status": "clean"
    }
  ]
}"#,
    )
    .unwrap();

    let gates = read_review_manifest(&manifest).unwrap();
    assert_eq!(gates.get("technical_review"), Some(&true));

    let _ = fs::remove_dir_all(root);
}
