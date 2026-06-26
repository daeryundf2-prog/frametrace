use crate::audit;
use crate::case_db;
use crate::qa::QaReport;
use crate::util::{read_to_string, write_text};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

const DISALLOWED_REPORT_CLAIMS: &[&str] = &[
    "court-ready",
    "court-grade",
    "court-proven",
    "legal-grade",
    "legal-proof",
];
const ALLOWED_REPORT_LANGUAGE: &[&str] = &[
    "report-defensible",
    "reproducible analysis record",
    "validated against the defined QA corpus",
    "candidate-unvalidated",
    "unsupported",
    "known limitation",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReportAuditChainState {
    Missing,
    Empty,
    Valid,
    Tampered,
    Unsupported,
    NotApplicable,
}

impl ReportAuditChainState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Empty => "empty",
            Self::Valid => "valid",
            Self::Tampered => "tampered",
            Self::Unsupported => "unsupported",
            Self::NotApplicable => "not-applicable",
        }
    }
}

#[derive(Debug, Clone)]
struct ReportAuditLogStatus {
    name: String,
    relative_path: String,
    state: ReportAuditChainState,
    required: bool,
    reason: String,
    entries: Option<usize>,
    last_entry_sha256: Option<String>,
    error: Option<String>,
}

struct AuditChainSpec {
    name: &'static str,
    relative_path: &'static str,
    artifact_dirs: &'static [&'static str],
    report_claim_markers: &'static [&'static str],
    report_claim_kind: ReportClaimKind,
    report_claim_reason: &'static str,
    unsupported_when_absent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReportClaimKind {
    ArtifactPath,
    ValidationStatus,
}

const REPORT_AUDIT_CHAIN_SPECS: &[AuditChainSpec] = &[
    AuditChainSpec {
        name: "clip export",
        relative_path: "artifacts/clips/export-log.jsonl",
        artifact_dirs: &["artifacts/clips"],
        report_claim_markers: &["artifacts/clips"],
        report_claim_kind: ReportClaimKind::ArtifactPath,
        report_claim_reason: "report claims artifacts under",
        unsupported_when_absent: false,
    },
    AuditChainSpec {
        name: "proxy",
        relative_path: "artifacts/proxies/proxy-log.jsonl",
        artifact_dirs: &["artifacts/proxies"],
        report_claim_markers: &["artifacts/proxies"],
        report_claim_kind: ReportClaimKind::ArtifactPath,
        report_claim_reason: "report claims artifacts under",
        unsupported_when_absent: false,
    },
    AuditChainSpec {
        name: "thumbnail",
        relative_path: "artifacts/thumbnails/thumbnail-log.jsonl",
        artifact_dirs: &["artifacts/thumbnails"],
        report_claim_markers: &["artifacts/thumbnails"],
        report_claim_kind: ReportClaimKind::ArtifactPath,
        report_claim_reason: "report claims artifacts under",
        unsupported_when_absent: false,
    },
    AuditChainSpec {
        name: "frame capture",
        relative_path: "artifacts/frames/frame-log.jsonl",
        artifact_dirs: &["artifacts/frames"],
        report_claim_markers: &["artifacts/frames"],
        report_claim_kind: ReportClaimKind::ArtifactPath,
        report_claim_reason: "report claims artifacts under",
        unsupported_when_absent: false,
    },
    AuditChainSpec {
        name: "carving",
        relative_path: "artifacts/carved/carve-log.jsonl",
        artifact_dirs: &["artifacts/carved"],
        report_claim_markers: &["artifacts/carved"],
        report_claim_kind: ReportClaimKind::ArtifactPath,
        report_claim_reason: "report claims artifacts under",
        unsupported_when_absent: false,
    },
    AuditChainSpec {
        name: "filesystem recovery",
        relative_path: "evidence/logs/tsk-audit.jsonl",
        artifact_dirs: &["db/filesystem", "artifacts/recovered/filesystem"],
        report_claim_markers: &["db/filesystem", "artifacts/recovered/filesystem"],
        report_claim_kind: ReportClaimKind::ArtifactPath,
        report_claim_reason: "report claims artifacts under",
        unsupported_when_absent: true,
    },
    AuditChainSpec {
        name: "validation",
        relative_path: "evidence/logs/validation-log.jsonl",
        artifact_dirs: &[],
        report_claim_markers: &[
            "ffprobe-video-stream-confirmed",
            "playback-confirmed",
            "validation-failed",
        ],
        report_claim_kind: ReportClaimKind::ValidationStatus,
        report_claim_reason: "report claims validation status",
        unsupported_when_absent: false,
    },
];

pub fn report_defense_check(case_dir: &Path, output_dir: &Path) -> Result<QaReport, String> {
    let checks = [
        ("case manifest", case_dir.join("case.json")),
        ("case database", case_db::case_db_path(case_dir)),
        ("video JSON index", case_dir.join("db/video_index.json")),
        ("video JSONL index", case_dir.join("db/videos.jsonl")),
        ("video path TSV", case_dir.join("db/video_paths.tsv")),
        ("case report", case_dir.join("reports/case-report.html")),
    ];
    let missing = checks
        .iter()
        .filter(|(_, path)| !path.is_file())
        .map(|(name, path)| format!("{name}: {}", path.display()))
        .collect::<Vec<_>>();
    let active_job_count =
        case_db::summarize_case_db(case_dir)?.map_or(0, |summary| summary.active_job_count);
    let claim_violations = report_claim_violations(case_dir)?;
    let audit_statuses = report_audit_chain_statuses(case_dir);
    let audit_violations = required_audit_chain_messages(&audit_statuses);
    let passed = missing.is_empty()
        && claim_violations.is_empty()
        && audit_violations.is_empty()
        && active_job_count == 0;
    fs::create_dir_all(output_dir)
        .map_err(|err| format!("failed to create QA output directory: {err}"))?;
    let json_path = output_dir.join("report-defense-report.json");
    let report_path = output_dir.join("report-defense-checklist.md");
    let evidence = ReportDefenseEvidence {
        passed,
        checks: &checks,
        missing: &missing,
        claim_violations: &claim_violations,
        audit_statuses: &audit_statuses,
        audit_violations: &audit_violations,
        active_job_count,
    };
    write_report_defense_json(&json_path, &evidence)?;
    let mut text = String::from("# Report Defensibility Checklist\n\n");
    text.push_str(&format!(
        "Machine-readable source: `{}`\n\n",
        json_path.display()
    ));
    for (name, path) in checks {
        let status = if path.is_file() { "PASS" } else { "FAIL" };
        text.push_str(&format!("- [{status}] {name}: `{}`\n", path.display()));
    }
    if !missing.is_empty() {
        text.push_str("\n## Missing\n\n");
        for item in &missing {
            text.push_str(&format!("- {item}\n"));
        }
    }
    if !claim_violations.is_empty() {
        text.push_str("\n## Disallowed Report Claims\n\n");
        for item in &claim_violations {
            text.push_str(&format!("- {item}\n"));
        }
    }
    if active_job_count > 0 {
        text.push_str("\n## Active Jobs\n\n");
        text.push_str(&format!(
            "- {active_job_count} running SQLite job(s) must complete or be marked interrupted before report defense.\n"
        ));
    }
    text.push_str("\n## Audit Chain Validation\n\n");
    for status in &audit_statuses {
        let entries = status
            .entries
            .map(|entries| entries.to_string())
            .unwrap_or_else(|| "-".to_string());
        text.push_str(&format!(
            "- [{}] {}: `{}` required={} reason={} entries={} last={} error={}\n",
            status.state.as_str(),
            status.name,
            status.relative_path,
            if status.required { "yes" } else { "no" },
            status.reason,
            entries,
            status.last_entry_sha256.as_deref().unwrap_or("-"),
            status.error.as_deref().unwrap_or("-")
        ));
    }
    if !audit_violations.is_empty() {
        text.push_str("\n## Audit Chain Failures\n\n");
        for item in &audit_violations {
            text.push_str(&format!("- {item}\n"));
        }
    }
    write_text(&report_path, &text)
        .map_err(|err| format!("failed to write report-defense checklist: {err}"))?;
    if passed {
        Ok(QaReport {
            report_path,
            passed,
        })
    } else {
        let audit_blockers = if audit_violations.is_empty() {
            "-".to_string()
        } else {
            audit_violations.join("; ")
        };
        Err(format!(
            "report defensibility QA failed: finding_keys={}; missing {} required artifacts, {} disallowed claim(s), {} audit chain failure(s), {} active job(s); audit blockers: {}",
            report_defense_failure_keys(
                &missing,
                &claim_violations,
                &audit_violations,
                active_job_count
            )
            .join(","),
            missing.len(),
            claim_violations.len(),
            audit_violations.len(),
            active_job_count,
            audit_blockers
        ))
    }
}

pub fn privacy_review_check(case_dir: &Path, output_dir: &Path) -> Result<QaReport, String> {
    fs::create_dir_all(output_dir)
        .map_err(|err| format!("failed to create QA output directory: {err}"))?;
    let surfaces = privacy_surfaces(case_dir);
    let sensitive_markers = collect_sensitive_path_markers(case_dir);
    let mut findings = Vec::new();

    let present_surface_count = surfaces
        .iter()
        .filter(|surface| surface.text.is_some())
        .count();
    findings.push(json!({
        "key": "distributable_surfaces_present",
        "status": if present_surface_count == 0 { "skipped" } else { "pass" },
        "message": if present_surface_count == 0 {
            "no distributable report, review, or package surfaces were available for privacy review".to_string()
        } else {
            format!("{present_surface_count} distributable surface(s) inspected")
        }
    }));

    let banned_hits = collect_banned_claim_hits(&surfaces);
    findings.push(json!({
        "key": "banned_legal_wording",
        "status": if banned_hits.is_empty() { "pass" } else { "failed" },
        "message": if banned_hits.is_empty() {
            "no banned report-defense wording found".to_string()
        } else {
            banned_hits.join("; ")
        }
    }));

    let disclosure_hits = collect_full_path_disclosure_hits(case_dir, &surfaces);
    findings.push(json!({
        "key": "full_path_disclosure_mode",
        "status": if disclosure_hits.is_empty() { "pass" } else { "partial" },
        "message": if disclosure_hits.is_empty() {
            "redacted distributable mode; no local/operator full-path disclosure artifact found".to_string()
        } else {
            disclosure_hits.join("; ")
        }
    }));

    let leakage_hits = collect_full_path_leakage_hits(&surfaces, &sensitive_markers);
    findings.push(json!({
        "key": "full_path_leakage",
        "status": if leakage_hits.is_empty() { "pass" } else { "failed" },
        "message": if leakage_hits.is_empty() {
            "no absolute case/source path markers found in distributable surfaces".to_string()
        } else {
            leakage_hits.join("; ")
        }
    }));

    let passed = findings.iter().all(|finding| {
        finding
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| status == "pass")
    });
    let report_path = output_dir.join("privacy-review.json");
    let surface_json = surfaces
        .iter()
        .map(|surface| {
            json!({
                "relative_path": surface.relative_path,
                "status": if surface.text.is_some() { "pass" } else { "not-applicable" }
            })
        })
        .collect::<Vec<_>>();
    write_pretty_json(
        &report_path,
        &json!({
            "schema_version": 1,
            "qa_type": "privacy_review",
            "passed": passed,
            "allowed_language": ALLOWED_REPORT_LANGUAGE,
            "sensitive_marker_count": sensitive_markers.len(),
            "surfaces": surface_json,
            "findings": findings,
        }),
    )?;

    if passed {
        Ok(QaReport {
            report_path,
            passed,
        })
    } else {
        Err(format!(
            "privacy QA failed: finding_keys={}; see {}",
            failing_finding_keys_from_file(&report_path)?.join(","),
            report_path.display()
        ))
    }
}

struct PrivacySurface {
    relative_path: &'static str,
    text: Option<String>,
}

fn privacy_surfaces(case_dir: &Path) -> Vec<PrivacySurface> {
    [
        "reports/case-report.html",
        "review/index.html",
        "review/evidence-viewer.html",
        "reports/package/package-manifest.json",
        "reports/package/review/index.html",
        "reports/package/review/evidence-viewer.html",
    ]
    .into_iter()
    .map(|relative_path| {
        let text = read_to_string(&case_dir.join(relative_path)).ok();
        PrivacySurface {
            relative_path,
            text,
        }
    })
    .collect()
}

fn collect_banned_claim_hits(surfaces: &[PrivacySurface]) -> Vec<String> {
    let mut hits = Vec::new();
    for surface in surfaces {
        let Some(text) = &surface.text else {
            continue;
        };
        let lower = text.to_ascii_lowercase();
        for term in DISALLOWED_REPORT_CLAIMS {
            if lower.contains(term) {
                hits.push(format!("{} contains `{term}`", surface.relative_path));
            }
        }
    }
    hits
}

fn collect_full_path_disclosure_hits(case_dir: &Path, surfaces: &[PrivacySurface]) -> Vec<String> {
    let mut hits = Vec::new();
    for relative_path in [
        "reports/privacy-full-path-disclosure.json",
        "review/privacy-full-path-disclosure.json",
        "reports/package/privacy-full-path-disclosure.json",
    ] {
        if case_dir.join(relative_path).is_file() {
            hits.push(format!(
                "{relative_path} declares local/operator full-path mode"
            ));
        }
    }
    for surface in surfaces {
        let Some(text) = &surface.text else {
            continue;
        };
        if text.contains(r#""local_operator_full_path_disclosure":true"#)
            || text.contains(r#""path_disclosure_mode":"local_operator_full_paths""#)
            || text.contains("LOCAL/OPERATOR MODE")
        {
            hits.push(format!(
                "{} contains local/operator full-path disclosure metadata",
                surface.relative_path
            ));
        }
    }
    hits
}

fn collect_full_path_leakage_hits(
    surfaces: &[PrivacySurface],
    sensitive_markers: &[String],
) -> Vec<String> {
    let mut hits = Vec::new();
    for surface in surfaces {
        let Some(text) = &surface.text else {
            continue;
        };
        for marker in sensitive_markers {
            if !marker.is_empty() && text.contains(marker) {
                hits.push(format!("{} leaks `{}`", surface.relative_path, marker));
            }
        }
    }
    hits.sort();
    hits.dedup();
    hits
}

fn collect_sensitive_path_markers(case_dir: &Path) -> Vec<String> {
    let mut markers = Vec::new();
    markers.push(case_dir.to_string_lossy().to_string());
    if let Ok(canonical) = case_dir.canonicalize() {
        markers.push(canonical.to_string_lossy().to_string());
    }
    collect_tsv_path_markers(&case_dir.join("db/video_paths.tsv"), &mut markers);
    collect_json_path_markers(&case_dir.join("db/video_index.json"), &mut markers);
    collect_jsonl_path_markers(&case_dir.join("db/videos.jsonl"), &mut markers);
    markers.retain(|marker| marker.len() > 3);
    markers.sort();
    markers.dedup();
    markers
}

fn collect_tsv_path_markers(path: &Path, markers: &mut Vec<String>) {
    let Ok(text) = read_to_string(path) else {
        return;
    };
    for line in text.lines().skip(1) {
        for field in line.split('\t') {
            if is_sensitive_path_marker(field) {
                markers.push(field.to_string());
            }
        }
    }
}

fn collect_jsonl_path_markers(path: &Path, markers: &mut Vec<String>) {
    let Ok(text) = read_to_string(path) else {
        return;
    };
    for line in text.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        collect_value_path_markers(&value, markers);
    }
}

fn collect_json_path_markers(path: &Path, markers: &mut Vec<String>) {
    let Ok(text) = read_to_string(path) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return;
    };
    collect_value_path_markers(&value, markers);
}

fn collect_value_path_markers(value: &Value, markers: &mut Vec<String>) {
    match value {
        Value::String(text) if is_sensitive_path_marker(text) => markers.push(text.to_string()),
        Value::Array(items) => {
            for item in items {
                collect_value_path_markers(item, markers);
            }
        }
        Value::Object(map) => {
            for value in map.values() {
                collect_value_path_markers(value, markers);
            }
        }
        _ => {}
    }
}

fn is_sensitive_path_marker(value: &str) -> bool {
    value.starts_with("file://") || Path::new(value).is_absolute()
}

fn report_audit_chain_statuses(case_dir: &Path) -> Vec<ReportAuditLogStatus> {
    let raw_statuses = audit::media_audit_chain_statuses(case_dir);
    REPORT_AUDIT_CHAIN_SPECS
        .iter()
        .map(|spec| {
            let raw = raw_statuses
                .iter()
                .find(|status| status.relative_path == spec.relative_path);
            let has_log = case_dir.join(spec.relative_path).is_file();
            let artifact_dir = spec.artifact_dirs.iter().copied().find(|artifact_dir| {
                has_case_surface_artifact(case_dir, artifact_dir, spec.relative_path)
            });
            let report_claim_marker = spec.report_claim_markers.iter().copied().find(|marker| {
                match spec.report_claim_kind {
                    ReportClaimKind::ArtifactPath => {
                        report_claims_chain(case_dir, marker, spec.relative_path)
                    }
                    ReportClaimKind::ValidationStatus => {
                        report_claims_validation_status(case_dir, marker, spec.relative_path)
                    }
                }
            });
            let required = has_log || artifact_dir.is_some() || report_claim_marker.is_some();
            let reason = if has_log {
                "audit log present".to_string()
            } else if let Some(artifact_dir) = artifact_dir {
                format!("case surface contains artifacts under {artifact_dir}")
            } else if let Some(marker) = report_claim_marker {
                if spec.report_claim_reason.ends_with("under") {
                    format!("{} {marker}", spec.report_claim_reason)
                } else {
                    spec.report_claim_reason.to_string()
                }
            } else if spec.unsupported_when_absent {
                "optional chain unsupported for this case surface".to_string()
            } else {
                "no reported artifacts for this chain".to_string()
            };
            let state = if required {
                raw.map(|status| report_state_from_audit(&status.state))
                    .unwrap_or(ReportAuditChainState::Missing)
            } else if spec.unsupported_when_absent {
                ReportAuditChainState::Unsupported
            } else {
                ReportAuditChainState::NotApplicable
            };
            ReportAuditLogStatus {
                name: spec.name.to_string(),
                relative_path: spec.relative_path.to_string(),
                state,
                required,
                reason,
                entries: raw.and_then(|status| status.entries),
                last_entry_sha256: raw.and_then(|status| status.last_entry_sha256.clone()),
                error: raw.and_then(|status| status.error.clone()),
            }
        })
        .collect()
}

fn report_state_from_audit(state: &audit::AuditChainState) -> ReportAuditChainState {
    match state {
        audit::AuditChainState::Valid => ReportAuditChainState::Valid,
        audit::AuditChainState::Empty => ReportAuditChainState::Empty,
        audit::AuditChainState::Missing => ReportAuditChainState::Missing,
        audit::AuditChainState::Tampered => ReportAuditChainState::Tampered,
    }
}

fn has_case_surface_artifact(case_dir: &Path, relative_dir: &str, relative_log: &str) -> bool {
    let artifact_dir = case_dir.join(relative_dir);
    let Ok(entries) = fs::read_dir(&artifact_dir) else {
        return false;
    };
    let log_name = Path::new(relative_log).file_name();
    entries.filter_map(Result::ok).any(|entry| {
        let path = entry.path();
        path.is_file() && path.file_name() != log_name
    })
}

fn report_claims_chain(case_dir: &Path, marker: &str, relative_log: &str) -> bool {
    ["reports/case-report.html", "review/evidence-viewer.html"]
        .iter()
        .filter_map(|relative_report| read_to_string(&case_dir.join(relative_report)).ok())
        .any(|text| text.replace(relative_log, "").contains(marker))
}

fn report_claims_validation_status(case_dir: &Path, marker: &str, relative_log: &str) -> bool {
    ["reports/case-report.html", "review/evidence-viewer.html"]
        .iter()
        .filter_map(|relative_report| read_to_string(&case_dir.join(relative_report)).ok())
        .any(|text| {
            let report_without_log_path = text.replace(relative_log, "");
            validation_log_array_has_entry(&report_without_log_path)
                || (!report_without_log_path.contains("const validationLog = []")
                    && report_without_log_path.contains(marker))
        })
}

fn validation_log_array_has_entry(text: &str) -> bool {
    let Some((_, after_prefix)) = text.split_once("const validationLog = [") else {
        return false;
    };
    !after_prefix.trim_start().starts_with(']')
}

fn required_audit_chain_messages(statuses: &[ReportAuditLogStatus]) -> Vec<String> {
    statuses
        .iter()
        .filter(|status| status.required)
        .filter(|status| {
            matches!(
                status.state,
                ReportAuditChainState::Missing
                    | ReportAuditChainState::Empty
                    | ReportAuditChainState::Tampered
            )
        })
        .map(|status| {
            format!(
                "{}: {} [{}] ({})",
                status.name,
                status.relative_path,
                status.state.as_str(),
                status.error.as_deref().unwrap_or(status.state.as_str())
            )
        })
        .collect()
}

struct ReportDefenseEvidence<'a> {
    passed: bool,
    checks: &'a [(&'a str, std::path::PathBuf)],
    missing: &'a [String],
    claim_violations: &'a [String],
    audit_statuses: &'a [ReportAuditLogStatus],
    audit_violations: &'a [String],
    active_job_count: u64,
}

fn write_report_defense_json(
    path: &Path,
    evidence: &ReportDefenseEvidence<'_>,
) -> Result<(), String> {
    let artifact_findings = evidence
        .checks
        .iter()
        .map(|(name, artifact_path)| {
            json!({
                "key": format!("required_artifact_{}", finding_key(name)),
                "status": if artifact_path.is_file() { "pass" } else { "failed" },
                "message": artifact_path.display().to_string(),
            })
        })
        .collect::<Vec<_>>();
    let mut findings = artifact_findings;
    findings.push(json!({
        "key": "banned_legal_wording",
        "status": if evidence.claim_violations.is_empty() { "pass" } else { "failed" },
        "message": if evidence.claim_violations.is_empty() {
            "no banned report-defense wording found".to_string()
        } else {
            evidence.claim_violations.join("; ")
        },
    }));
    findings.push(json!({
        "key": "active_jobs",
        "status": if evidence.active_job_count == 0 { "pass" } else { "failed" },
        "message": if evidence.active_job_count == 0 {
            "no running SQLite jobs".to_string()
        } else {
            format!("{} running SQLite job(s)", evidence.active_job_count)
        },
    }));
    for status in evidence.audit_statuses {
        findings.push(json!({
            "key": format!("audit_chain_{}", finding_key(&status.name)),
            "status": audit_finding_status(status),
            "message": status.reason,
            "required": status.required,
            "state": status.state.as_str(),
            "relative_path": status.relative_path,
            "entries": status.entries,
            "last_entry_sha256": status.last_entry_sha256,
            "error": status.error,
        }));
    }
    write_pretty_json(
        path,
        &json!({
            "schema_version": 1,
            "qa_type": "report_defense",
            "passed": evidence.passed,
            "missing": evidence.missing,
            "claim_violations": evidence.claim_violations,
            "audit_violations": evidence.audit_violations,
            "active_job_count": evidence.active_job_count,
            "findings": findings,
            "audit_chains": evidence.audit_statuses.iter().map(audit_status_json).collect::<Vec<_>>(),
        }),
    )
}

fn audit_status_json(status: &ReportAuditLogStatus) -> Value {
    json!({
        "key": format!("audit_chain_{}", finding_key(&status.name)),
        "name": status.name,
        "relative_path": status.relative_path,
        "state": status.state.as_str(),
        "status": audit_finding_status(status),
        "required": status.required,
        "reason": status.reason,
        "entries": status.entries,
        "last_entry_sha256": status.last_entry_sha256,
        "error": status.error,
    })
}

fn audit_finding_status(status: &ReportAuditLogStatus) -> &'static str {
    match status.state {
        ReportAuditChainState::Valid => "pass",
        ReportAuditChainState::Missing
        | ReportAuditChainState::Empty
        | ReportAuditChainState::Tampered
            if status.required =>
        {
            "failed"
        }
        ReportAuditChainState::Missing
        | ReportAuditChainState::Empty
        | ReportAuditChainState::Tampered => "skipped",
        ReportAuditChainState::Unsupported => "unsupported",
        ReportAuditChainState::NotApplicable => "not-applicable",
    }
}

fn report_defense_failure_keys(
    missing: &[String],
    claim_violations: &[String],
    audit_violations: &[String],
    active_job_count: u64,
) -> Vec<String> {
    let mut keys = Vec::new();
    if !missing.is_empty() {
        keys.push("required_artifacts".to_string());
    }
    if !claim_violations.is_empty() {
        keys.push("banned_legal_wording".to_string());
    }
    if !audit_violations.is_empty() {
        keys.push("audit_chain_status".to_string());
    }
    if active_job_count > 0 {
        keys.push("active_jobs".to_string());
    }
    keys
}

fn report_claim_violations(case_dir: &Path) -> Result<Vec<String>, String> {
    let mut violations = Vec::new();
    for rel in ["reports/case-report.html", "review/evidence-viewer.html"] {
        let path = case_dir.join(rel);
        if !path.is_file() {
            continue;
        }
        let text = read_to_string(&path)
            .map_err(|err| format!("failed to read report output {}: {err}", path.display()))?;
        let lower = text.to_ascii_lowercase();
        for term in DISALLOWED_REPORT_CLAIMS {
            if lower.contains(term) {
                violations.push(format!("{rel}: contains disallowed claim `{term}`"));
            }
        }
    }
    Ok(violations)
}

fn failing_finding_keys_from_file(path: &Path) -> Result<Vec<String>, String> {
    let text = read_to_string(path)
        .map_err(|err| format!("failed to read QA report {}: {err}", path.display()))?;
    let value = serde_json::from_str::<Value>(&text)
        .map_err(|err| format!("failed to parse QA report {}: {err}", path.display()))?;
    Ok(value
        .get("findings")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|finding| {
            finding
                .get("status")
                .and_then(Value::as_str)
                .is_some_and(|status| matches!(status, "failed" | "partial" | "skipped"))
        })
        .filter_map(|finding| finding.get("key").and_then(Value::as_str))
        .map(str::to_string)
        .collect())
}

fn finding_key(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn write_pretty_json(path: &Path, value: &Value) -> Result<(), String> {
    let mut text = serde_json::to_string_pretty(value)
        .map_err(|err| format!("failed to serialize QA JSON {}: {err}", path.display()))?;
    text.push('\n');
    write_text(path, &text).map_err(|err| format!("failed to write {}: {err}", path.display()))
}
