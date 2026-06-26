# T4 Review Context

Goal: Make missing required audit chains block report-defense with typed states missing, empty, valid, tampered, unsupported, not-applicable.
Scope constraints: primary src/qa_report_defense.rs, src/audit.rs if needed, tests/fixtures; avoid video_export/tool_policy T6 scope.
Changed by T4: src/audit.rs, src/qa_report_defense.rs, src/qa_tests.rs, tests/media_contract.rs.
Pre-existing target diff before T4 is captured in pre-task-target-diff.patch; qa_tests.rs already had unrelated review-manifest test removal.
Evidence: red-report-defense-tests.log, baseline-current-missing-log-cli.log, green/focused logs, manual-qa-happy.log, manual-qa-failure.log, fmt/clippy/cargo-test-full/git-diff-check logs.
Oversized-file constraint: src/audit.rs, src/qa_report_defense.rs, src/qa_tests.rs exceed 250 pure LOC already/in current dirty plan; T11 owns module splits, T4 keeps scoped behavior.

## T4 Diff
diff --git a/src/audit.rs b/src/audit.rs
index 097d965..28bca5e 100644
--- a/src/audit.rs
+++ b/src/audit.rs
@@ -13,7 +13,8 @@ pub struct AuditChainVerification {
 
 #[derive(Debug, Clone, PartialEq, Eq)]
 pub enum AuditChainState {
-    Verified,
+    Valid,
+    Empty,
     Missing,
     Tampered,
 }
@@ -21,7 +22,8 @@ pub enum AuditChainState {
 impl AuditChainState {
     pub const fn as_str(&self) -> &'static str {
         match self {
-            Self::Verified => "verified",
+            Self::Valid => "valid",
+            Self::Empty => "empty",
             Self::Missing => "missing",
             Self::Tampered => "tampered",
         }
@@ -247,10 +249,18 @@ fn audit_log_status(case_dir: &Path, name: &str, relative_path: &str) -> AuditLo
     }
 
     match verify_chained_jsonl(&path) {
+        Ok(verification) if verification.entries == 0 => AuditLogStatus {
+            name: name.to_string(),
+            relative_path: relative_path.to_string(),
+            state: AuditChainState::Empty,
+            entries: Some(verification.entries),
+            last_entry_sha256: Some(verification.last_entry_sha256),
+            error: Some("audit log has no entries".to_string()),
+        },
         Ok(verification) => AuditLogStatus {
             name: name.to_string(),
             relative_path: relative_path.to_string(),
-            state: AuditChainState::Verified,
+            state: AuditChainState::Valid,
             entries: Some(verification.entries),
             last_entry_sha256: Some(verification.last_entry_sha256),
             error: None,
@@ -372,7 +382,7 @@ mod tests {
     }
 
     #[test]
-    fn media_audit_statuses_report_verified_missing_and_tampered_logs() {
+    fn media_audit_statuses_report_valid_empty_missing_and_tampered_logs() {
         let dir = std::env::temp_dir().join(format!(
             "frametrace-audit-status-test-{}",
             std::process::id()
@@ -390,6 +400,8 @@ mod tests {
             r#"{"event":"make-proxy"}"#,
         )
         .unwrap();
+        fs::create_dir_all(dir.join("artifacts/clips")).unwrap();
+        fs::write(dir.join("artifacts/clips/export-log.jsonl"), "").unwrap();
 
         let statuses = media_audit_chain_statuses(&dir);
 
@@ -398,7 +410,14 @@ mod tests {
                 .iter()
                 .find(|status| status.relative_path == "evidence/logs/validation-log.jsonl")
                 .map(|status| &status.state),
-            Some(&AuditChainState::Verified)
+            Some(&AuditChainState::Valid)
+        );
+        assert_eq!(
+            statuses
+                .iter()
+                .find(|status| status.relative_path == "artifacts/clips/export-log.jsonl")
+                .map(|status| &status.state),
+            Some(&AuditChainState::Empty)
         );
         assert_eq!(
             statuses
@@ -414,7 +433,8 @@ mod tests {
                 .map(|status| &status.state),
             Some(&AuditChainState::Missing)
         );
-        assert!(audit_chain_statuses_json(&statuses).contains(r#""status":"verified""#));
+        assert!(audit_chain_statuses_json(&statuses).contains(r#""status":"valid""#));
+        assert!(audit_chain_statuses_json(&statuses).contains(r#""status":"empty""#));
         assert_eq!(tampered_audit_chain_messages(&statuses).len(), 1);
 
         let _ = fs::remove_dir_all(dir);
diff --git a/src/qa_report_defense.rs b/src/qa_report_defense.rs
index c34ed36..a85d0b5 100644
--- a/src/qa_report_defense.rs
+++ b/src/qa_report_defense.rs
@@ -8,6 +8,93 @@ use std::path::Path;
 const DISALLOWED_REPORT_CLAIMS: &[&str] =
     &["court-ready", "court-grade", "court-proven", "legal-grade"];
 
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+enum ReportAuditChainState {
+    Missing,
+    Empty,
+    Valid,
+    Tampered,
+    Unsupported,
+    NotApplicable,
+}
+
+impl ReportAuditChainState {
+    const fn as_str(self) -> &'static str {
+        match self {
+            Self::Missing => "missing",
+            Self::Empty => "empty",
+            Self::Valid => "valid",
+            Self::Tampered => "tampered",
+            Self::Unsupported => "unsupported",
+            Self::NotApplicable => "not-applicable",
+        }
+    }
+}
+
+#[derive(Debug, Clone)]
+struct ReportAuditLogStatus {
+    name: String,
+    relative_path: String,
+    state: ReportAuditChainState,
+    required: bool,
+    reason: String,
+    entries: Option<usize>,
+    last_entry_sha256: Option<String>,
+    error: Option<String>,
+}
+
+struct AuditChainSpec {
+    name: &'static str,
+    relative_path: &'static str,
+    artifact_dir: Option<&'static str>,
+    unsupported_when_absent: bool,
+}
+
+const REPORT_AUDIT_CHAIN_SPECS: &[AuditChainSpec] = &[
+    AuditChainSpec {
+        name: "clip export",
+        relative_path: "artifacts/clips/export-log.jsonl",
+        artifact_dir: Some("artifacts/clips"),
+        unsupported_when_absent: false,
+    },
+    AuditChainSpec {
+        name: "proxy",
+        relative_path: "artifacts/proxies/proxy-log.jsonl",
+        artifact_dir: Some("artifacts/proxies"),
+        unsupported_when_absent: false,
+    },
+    AuditChainSpec {
+        name: "thumbnail",
+        relative_path: "artifacts/thumbnails/thumbnail-log.jsonl",
+        artifact_dir: Some("artifacts/thumbnails"),
+        unsupported_when_absent: false,
+    },
+    AuditChainSpec {
+        name: "frame capture",
+        relative_path: "artifacts/frames/frame-log.jsonl",
+        artifact_dir: Some("artifacts/frames"),
+        unsupported_when_absent: false,
+    },
+    AuditChainSpec {
+        name: "carving",
+        relative_path: "artifacts/carved/carve-log.jsonl",
+        artifact_dir: Some("artifacts/carved"),
+        unsupported_when_absent: false,
+    },
+    AuditChainSpec {
+        name: "filesystem recovery",
+        relative_path: "evidence/logs/tsk-audit.jsonl",
+        artifact_dir: Some("db/filesystem"),
+        unsupported_when_absent: true,
+    },
+    AuditChainSpec {
+        name: "validation",
+        relative_path: "evidence/logs/validation-log.jsonl",
+        artifact_dir: None,
+        unsupported_when_absent: false,
+    },
+];
+
 pub fn report_defense_check(case_dir: &Path, output_dir: &Path) -> Result<QaReport, String> {
     let checks = [
         ("case manifest", case_dir.join("case.json")),
@@ -25,8 +112,8 @@ pub fn report_defense_check(case_dir: &Path, output_dir: &Path) -> Result<QaRepo
     let active_job_count =
         case_db::summarize_case_db(case_dir)?.map_or(0, |summary| summary.active_job_count);
     let claim_violations = report_claim_violations(case_dir)?;
-    let audit_statuses = audit::media_audit_chain_statuses(case_dir);
-    let audit_violations = audit::tampered_audit_chain_messages(&audit_statuses);
+    let audit_statuses = report_audit_chain_statuses(case_dir);
+    let audit_violations = required_audit_chain_messages(&audit_statuses);
     let passed = missing.is_empty()
         && claim_violations.is_empty()
         && audit_violations.is_empty()
@@ -64,10 +151,12 @@ pub fn report_defense_check(case_dir: &Path, output_dir: &Path) -> Result<QaRepo
             .map(|entries| entries.to_string())
             .unwrap_or_else(|| "-".to_string());
         text.push_str(&format!(
-            "- [{}] {}: `{}` entries={} last={} error={}\n",
+            "- [{}] {}: `{}` required={} reason={} entries={} last={} error={}\n",
             status.state.as_str(),
             status.name,
             status.relative_path,
+            if status.required { "yes" } else { "no" },
+            status.reason,
             entries,
             status.last_entry_sha256.as_deref().unwrap_or("-"),
             status.error.as_deref().unwrap_or("-")
@@ -87,16 +176,117 @@ pub fn report_defense_check(case_dir: &Path, output_dir: &Path) -> Result<QaRepo
             passed,
         })
     } else {
+        let audit_blockers = if audit_violations.is_empty() {
+            "-".to_string()
+        } else {
+            audit_violations.join("; ")
+        };
         Err(format!(
-            "report defensibility QA failed: missing {} required artifacts, {} disallowed claim(s), {} audit chain failure(s), {} active job(s)",
+            "report defensibility QA failed: missing {} required artifacts, {} disallowed claim(s), {} audit chain failure(s), {} active job(s); audit blockers: {}",
             missing.len(),
             claim_violations.len(),
             audit_violations.len(),
-            active_job_count
+            active_job_count,
+            audit_blockers
         ))
     }
 }
 
+fn report_audit_chain_statuses(case_dir: &Path) -> Vec<ReportAuditLogStatus> {
+    let raw_statuses = audit::media_audit_chain_statuses(case_dir);
+    REPORT_AUDIT_CHAIN_SPECS
+        .iter()
+        .map(|spec| {
+            let raw = raw_statuses
+                .iter()
+                .find(|status| status.relative_path == spec.relative_path);
+            let has_log = case_dir.join(spec.relative_path).is_file();
+            let has_artifact = spec
+                .artifact_dir
+                .map(|artifact_dir| {
+                    has_case_surface_artifact(case_dir, artifact_dir, spec.relative_path)
+                })
+                .unwrap_or(false);
+            let required = has_log || has_artifact;
+            let reason = if has_log {
+                "audit log present".to_string()
+            } else if has_artifact {
+                format!(
+                    "case surface contains artifacts under {}",
+                    spec.artifact_dir.unwrap_or("-")
+                )
+            } else if spec.unsupported_when_absent {
+                "optional chain unsupported for this case surface".to_string()
+            } else {
+                "no reported artifacts for this chain".to_string()
+            };
+            let state = if required {
+                raw.map(|status| report_state_from_audit(&status.state))
+                    .unwrap_or(ReportAuditChainState::Missing)
+            } else if spec.unsupported_when_absent {
+                ReportAuditChainState::Unsupported
+            } else {
+                ReportAuditChainState::NotApplicable
+            };
+            ReportAuditLogStatus {
+                name: spec.name.to_string(),
+                relative_path: spec.relative_path.to_string(),
+                state,
+                required,
+                reason,
+                entries: raw.and_then(|status| status.entries),
+                last_entry_sha256: raw.and_then(|status| status.last_entry_sha256.clone()),
+                error: raw.and_then(|status| status.error.clone()),
+            }
+        })
+        .collect()
+}
+
+fn report_state_from_audit(state: &audit::AuditChainState) -> ReportAuditChainState {
+    match state {
+        audit::AuditChainState::Valid => ReportAuditChainState::Valid,
+        audit::AuditChainState::Empty => ReportAuditChainState::Empty,
+        audit::AuditChainState::Missing => ReportAuditChainState::Missing,
+        audit::AuditChainState::Tampered => ReportAuditChainState::Tampered,
+    }
+}
+
+fn has_case_surface_artifact(case_dir: &Path, relative_dir: &str, relative_log: &str) -> bool {
+    let artifact_dir = case_dir.join(relative_dir);
+    let Ok(entries) = fs::read_dir(&artifact_dir) else {
+        return false;
+    };
+    let log_name = Path::new(relative_log).file_name();
+    entries.filter_map(Result::ok).any(|entry| {
+        let path = entry.path();
+        path.is_file() && path.file_name() != log_name
+    })
+}
+
+fn required_audit_chain_messages(statuses: &[ReportAuditLogStatus]) -> Vec<String> {
+    statuses
+        .iter()
+        .filter(|status| status.required)
+        .filter(|status| {
+            matches!(
+                status.state,
+                ReportAuditChainState::Missing
+                    | ReportAuditChainState::Empty
+                    | ReportAuditChainState::Tampered
+            )
+        })
+        .map(|status| {
+            format!(
+                "{}: {} [{}] ({})",
+                status.name,
+                status.relative_path,
+                status.state.as_str(),
+                status.error.as_deref().unwrap_or(status.state.as_str())
+            )
+        })
+        .collect()
+}
+
 fn report_claim_violations(case_dir: &Path) -> Result<Vec<String>, String> {
     let mut violations = Vec::new();
     for rel in ["reports/case-report.html", "review/evidence-viewer.html"] {
diff --git a/src/qa_tests.rs b/src/qa_tests.rs
index 61d5ddd..089ef9c 100644
--- a/src/qa_tests.rs
+++ b/src/qa_tests.rs
@@ -1,8 +1,5 @@
 use super::qa_test_fixtures::seed_repro_case;
-use super::{
-    accuracy_report, performance_report, read_review_manifest, report_defense_check,
-    reproducibility_report,
-};
+use super::{accuracy_report, performance_report, report_defense_check, reproducibility_report};
 use crate::audit;
 use crate::case_db;
 use std::fs;
@@ -178,6 +175,140 @@ fn report_defense_rejects_tampered_media_audit_logs() {
     let _ = fs::remove_dir_all(root);
 }
 
+#[test]
+fn report_defense_blocks_missing_required_proxy_audit_log() {
+    let root = std::env::temp_dir().join(format!(
+        "frametrace-report-missing-audit-chain-test-{}",
+        std::process::id()
+    ));
+    let case_dir = root.join("case");
+    let output_dir = root.join("qa");
+    let _ = fs::remove_dir_all(&root);
+    seed_report_defense_case(
+        &case_dir,
+        "<html><body>derived artifact artifacts/proxies/proxy_000001.mp4</body></html>",
+    );
+    fs::create_dir_all(case_dir.join("artifacts/proxies")).unwrap();
+    fs::write(
+        case_dir.join("artifacts/proxies/proxy_000001.mp4"),
+        "proxy bytes",
+    )
+    .unwrap();
+
+    let err = report_defense_check(&case_dir, &output_dir).unwrap_err();
+
+    assert!(err.contains("artifacts/proxies/proxy-log.jsonl"));
+    assert!(err.contains("missing"));
+    let checklist = fs::read_to_string(output_dir.join("report-defense-checklist.md")).unwrap();
+    assert!(checklist.contains("[missing] proxy"));
+    assert!(checklist.contains("required=yes"));
+    assert!(checklist.contains("artifacts/proxies/proxy-log.jsonl"));
+
+    let _ = fs::remove_dir_all(root);
+}
+
+#[test]
+fn report_defense_blocks_empty_required_proxy_audit_log() {
+    let root = std::env::temp_dir().join(format!(
+        "frametrace-report-empty-audit-chain-test-{}",
+        std::process::id()
+    ));
+    let case_dir = root.join("case");
+    let output_dir = root.join("qa");
+    let _ = fs::remove_dir_all(&root);
+    seed_report_defense_case(
+        &case_dir,
+        "<html><body>derived artifact artifacts/proxies/proxy_000001.mp4</body></html>",
+    );
+    fs::create_dir_all(case_dir.join("artifacts/proxies")).unwrap();
+    fs::write(
+        case_dir.join("artifacts/proxies/proxy_000001.mp4"),
+        "proxy bytes",
+    )
+    .unwrap();
+    fs::write(case_dir.join("artifacts/proxies/proxy-log.jsonl"), "").unwrap();
+
+    let err = report_defense_check(&case_dir, &output_dir).unwrap_err();
+
+    assert!(err.contains("artifacts/proxies/proxy-log.jsonl"));
+    assert!(err.contains("empty"));
+    let checklist = fs::read_to_string(output_dir.join("report-defense-checklist.md")).unwrap();
+    assert!(checklist.contains("[empty] proxy"));
+    assert!(checklist.contains("required=yes"));
+
+    let _ = fs::remove_dir_all(root);
+}
+
+#[test]
+fn report_defense_displays_optional_audit_chain_states_without_pass_labels() {
+    let root = std::env::temp_dir().join(format!(
+        "frametrace-report-optional-audit-chain-test-{}",
+        std::process::id()
+    ));
+    let case_dir = root.join("case");
+    let output_dir = root.join("qa");
+    let _ = fs::remove_dir_all(&root);
+    seed_report_defense_case(&case_dir, "<html><body>scan-only case</body></html>");
+
+    let report = report_defense_check(&case_dir, &output_dir).unwrap();
+
+    assert!(report.passed);
+    let checklist = fs::read_to_string(output_dir.join("report-defense-checklist.md")).unwrap();
+    assert!(checklist.contains("[not-applicable] proxy"));
+    assert!(checklist.contains("[unsupported] filesystem recovery"));
+    assert!(!checklist.contains("[PASS] proxy"));
+    assert!(!checklist.contains("[PASS] filesystem recovery"));
+
+    let _ = fs::remove_dir_all(root);
+}
+
+#[test]
+fn report_defense_allows_valid_required_proxy_audit_log() {
+    let root = std::env::temp_dir().join(format!(
+        "frametrace-report-valid-audit-chain-test-{}",
+        std::process::id()
+    ));
+    let case_dir = root.join("case");
+    let output_dir = root.join("qa");
+    let _ = fs::remove_dir_all(&root);
+    seed_report_defense_case(
+        &case_dir,
+        "<html><body>derived artifact artifacts/proxies/proxy_000001.mp4</body></html>",
+    );
+    fs::create_dir_all(case_dir.join("artifacts/proxies")).unwrap();
+    fs::write(
+        case_dir.join("artifacts/proxies/proxy_000001.mp4"),
+        "proxy bytes",
+    )
+    .unwrap();
+    audit::append_chained_jsonl(
+        &case_dir.join("artifacts/proxies/proxy-log.jsonl"),
+        r#"{"event":"make-proxy","kind":"proxy"}"#,
+    )
+    .unwrap();
+
+    let report = report_defense_check(&case_dir, &output_dir).unwrap();
+
+    assert!(report.passed);
+    let checklist = fs::read_to_string(output_dir.join("report-defense-checklist.md")).unwrap();
+    assert!(checklist.contains("[valid] proxy"));
+    assert!(checklist.contains("required=yes"));
+
+    let _ = fs::remove_dir_all(root);
+}
+
+fn seed_report_defense_case(case_dir: &Path, report_body: &str) {
+    fs::create_dir_all(case_dir.join("db")).unwrap();
+    fs::create_dir_all(case_dir.join("reports")).unwrap();
+    fs::write(case_dir.join("case.json"), "{}").unwrap();
+    fs::write(case_dir.join("db/video_index.json"), "{}").unwrap();
+    fs::write(case_dir.join("db/videos.jsonl"), "").unwrap();
+    fs::write(case_dir.join("db/video_paths.tsv"), "id\tsource_path\n").unwrap();
+    fs::write(case_dir.join("reports/case-report.html"), report_body).unwrap();
+    let conn = case_db::open_case_db(case_dir).unwrap();
+    case_db::init_schema(&conn).unwrap();
+}
+
 #[test]
 fn reproducibility_normalizes_case_paths_timestamps_and_package_times() {
     let root = std::env::temp_dir().join(format!(
@@ -236,31 +367,3 @@ fn performance_report_records_query_latency_metrics() {
 
     let _ = fs::remove_dir_all(root);
 }
-
-#[test]
-fn review_manifest_accepts_key_value_and_markdown_gates() {
-    let root = std::env::temp_dir().join(format!("frametrace-review-test-{}", std::process::id()));
-    let _ = fs::remove_dir_all(&root);
-    fs::create_dir_all(&root).unwrap();
-    let manifest = root.join("release-review.txt");
-    fs::write(
-        &manifest,
-        "\
-technical_review=pass
-security review: approved
-- [x] Migration Validation
-- [x] Operator Review
-legal-review=done
-",
-    )
-    .unwrap();
-
-    let gates = read_review_manifest(&manifest).unwrap();
-    assert_eq!(gates.get("technical_review"), Some(&true));
-    assert_eq!(gates.get("security_review"), Some(&true));
-    assert_eq!(gates.get("migration_validation"), Some(&true));
-    assert_eq!(gates.get("operator_review"), Some(&true));
-    assert_eq!(gates.get("legal_review"), Some(&true));
-
-    let _ = fs::remove_dir_all(root);
-}
diff --git a/tests/media_contract.rs b/tests/media_contract.rs
index 83bd314..a708a8f 100644
--- a/tests/media_contract.rs
+++ b/tests/media_contract.rs
@@ -16,7 +16,7 @@ fn report_discloses_derived_provenance_and_validation_failures() {
         carve_log_jsonl: "",
         filesystem_log_jsonl: "",
         validation_log_jsonl: r#"{"event":"validate-artifact","operator":"qa-operator","method":"ffprobe-container-video-stream","source_artifact_id":"source-carve_000001-dddddddddddd","target_path":"/case/artifacts/carved/bad.mp4","target_sha256":"dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd","validation_status":"validation-failed","validation_note":"ffprobe could not parse the file","entry_sha256":"chain"}"#,
-        audit_chain_status_json: r#"[{"name":"validation","relative_path":"evidence/logs/validation-log.jsonl","status":"verified","entries":1,"last_entry_sha256":"chain","error":null}]"#,
+        audit_chain_status_json: r#"[{"name":"validation","relative_path":"evidence/logs/validation-log.jsonl","status":"valid","entries":1,"last_entry_sha256":"chain","error":null}]"#,
     });
 
     assert!(html.contains("qa-operator"));
@@ -29,7 +29,7 @@ fn report_discloses_derived_provenance_and_validation_failures() {
     assert!(html.contains("validation-failed"));
     assert!(html.contains("ffprobe could not parse the file"));
     assert!(html.contains("감사 체인 검증"));
-    assert!(html.contains("verified"));
+    assert!(html.contains("valid"));
 }
 
 #[test]
