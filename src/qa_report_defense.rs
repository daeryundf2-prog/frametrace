use crate::audit;
use crate::case_db;
use crate::qa::QaReport;
use crate::util::{read_to_string, write_text};
use std::fs;
use std::path::Path;

const DISALLOWED_REPORT_CLAIMS: &[&str] =
    &["court-ready", "court-grade", "court-proven", "legal-grade"];

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
    let audit_statuses = audit::media_audit_chain_statuses(case_dir);
    let audit_violations = audit::tampered_audit_chain_messages(&audit_statuses);
    let passed = missing.is_empty()
        && claim_violations.is_empty()
        && audit_violations.is_empty()
        && active_job_count == 0;
    fs::create_dir_all(output_dir)
        .map_err(|err| format!("failed to create QA output directory: {err}"))?;
    let report_path = output_dir.join("report-defense-checklist.md");
    let mut text = String::from("# Report Defensibility Checklist\n\n");
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
            "- [{}] {}: `{}` entries={} last={} error={}\n",
            status.state.as_str(),
            status.name,
            status.relative_path,
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
        Err(format!(
            "report defensibility QA failed: missing {} required artifacts, {} disallowed claim(s), {} audit chain failure(s), {} active job(s)",
            missing.len(),
            claim_violations.len(),
            audit_violations.len(),
            active_job_count
        ))
    }
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
