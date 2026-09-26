//! Chain-of-custody statement generator.
//!
//! Produces `reports/custody-statement.html` — a print-friendly document
//! an examiner can hand to opposing counsel or a court together with the
//! case package. It records the case identity, every registered evidence
//! source (with acquisition metadata), the full job ledger, and the
//! verification status of each chained audit log — verified live at
//! generation time, never restated from a cached claim.
//!
//! The statement deliberately distinguishes original evidence from
//! derived work products (proxies, clips, thumbnails, carved candidates)
//! so nobody can read a derived artifact as an authenticated original.

use crate::audit;
use crate::case_db;
use crate::util::{html_escape, now_unix, read_to_string, write_text_atomic};
use std::path::{Path, PathBuf};

pub const CUSTODY_REL_PATH: &str = "reports/custody-statement.html";

/// Directories whose `*-log.jsonl` files are chained audit lanes worth
/// verifying in the statement. Reviewer data files (videos.jsonl,
/// timeline.jsonl) are index formats, not audit chains, and stay out.
const AUDIT_LOG_DIRS: &[&str] = &["artifacts", "evidence/logs"];

/// Generates (or refreshes) `reports/custody-statement.html`.
/// Returns the written path.
pub fn generate(case_dir: &Path) -> Result<PathBuf, String> {
    if !case_dir.join("case.json").is_file() {
        return Err(format!(
            "not a FrameTrace case directory: {}",
            case_dir.display()
        ));
    }
    let generated_unix = now_unix()?;
    let manifest = read_to_string(&case_dir.join("case.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .unwrap_or(serde_json::Value::Null);
    let sources = load_evidence_sources(case_dir);
    let jobs = load_jobs(case_dir);
    let logs = verify_audit_logs(case_dir);
    let html = render(
        case_dir,
        &manifest,
        &sources,
        &jobs,
        &logs,
        generated_unix as i64,
    );

    let out = case_dir.join(CUSTODY_REL_PATH);
    write_text_atomic(&out, &html)
        .map_err(|err| format!("failed to write custody statement {}: {err}", out.display()))?;
    Ok(out)
}

struct SourceRow {
    source_id: String,
    kind: String,
    path: String,
    registered_unix: i64,
    write_protect: Option<String>,
    acquisition_tool: Option<String>,
    evidence_hash: Option<String>,
}

struct JobRow {
    job_id: String,
    job_type: String,
    status: String,
    subject_path: String,
    started_unix: i64,
    completed_unix: Option<i64>,
    total_units: Option<i64>,
    completed_units: i64,
    error: Option<String>,
}

struct LogRow {
    rel_path: String,
    /// Ok(verification) when the log parsed and chain-checked, Err(text)
    /// when the lane could not be verified — printed verbatim so a broken
    /// lane is disclosed, never hidden.
    result: Result<audit::AuditChainVerification, String>,
}

fn load_evidence_sources(case_dir: &Path) -> Vec<SourceRow> {
    let db = case_db::case_db_path(case_dir);
    let Ok(conn) = case_db::open_readonly_case_db(&db) else {
        return Vec::new();
    };
    let mut stmt = match conn.prepare(
        "SELECT source_id, kind, path, registered_unix, write_protect,
                acquisition_tool, evidence_hash
         FROM evidence_sources ORDER BY registered_unix ASC",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return Vec::new(),
    };
    stmt.query_map([], |row| {
        Ok(SourceRow {
            source_id: row.get(0)?,
            kind: row.get(1)?,
            path: row.get(2)?,
            registered_unix: row.get(3)?,
            write_protect: row.get(4)?,
            acquisition_tool: row.get(5)?,
            evidence_hash: row.get(6)?,
        })
    })
    .map(|rows| rows.flatten().collect())
    .unwrap_or_default()
}

fn load_jobs(case_dir: &Path) -> Vec<JobRow> {
    let db = case_db::case_db_path(case_dir);
    let Ok(conn) = case_db::open_readonly_case_db(&db) else {
        return Vec::new();
    };
    let mut stmt = match conn.prepare(
        "SELECT job_id, job_type, status, subject_path, started_unix,
                completed_unix, total_units, completed_units, error
         FROM jobs ORDER BY started_unix ASC",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return Vec::new(),
    };
    stmt.query_map([], |row| {
        Ok(JobRow {
            job_id: row.get(0)?,
            job_type: row.get(1)?,
            status: row.get(2)?,
            subject_path: row.get(3)?,
            started_unix: row.get(4)?,
            completed_unix: row.get(5)?,
            total_units: row.get(6)?,
            completed_units: row.get(7)?,
            error: row.get(8)?,
        })
    })
    .map(|rows| rows.flatten().collect())
    .unwrap_or_default()
}

fn verify_audit_logs(case_dir: &Path) -> Vec<LogRow> {
    let mut paths: Vec<PathBuf> = Vec::new();
    for rel_dir in AUDIT_LOG_DIRS {
        collect_log_files(&case_dir.join(rel_dir), &mut paths, 0);
    }
    paths.sort();
    paths
        .into_iter()
        .map(|path| LogRow {
            rel_path: path
                .strip_prefix(case_dir)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/"),
            result: audit::verify_chained_jsonl(&path),
        })
        .collect()
}

fn collect_log_files(dir: &Path, out: &mut Vec<PathBuf>, depth: u32) {
    if depth > 4 || !dir.is_dir() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_log_files(&path, out, depth + 1);
        } else if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|name| name.ends_with("log.jsonl"))
        {
            out.push(path);
        }
    }
}

fn esc(value: &str) -> String {
    html_escape(value)
}

fn opt(value: &Option<String>) -> String {
    value.as_deref().map(esc).unwrap_or_else(|| "—".to_string())
}

fn ts(unix: i64) -> String {
    // UTC ISO-8601 without a date crate: days-from-civil.
    if unix <= 0 {
        return "—".to_string();
    }
    let days = unix / 86400;
    let secs = unix % 86400;
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}Z",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

fn render(
    case_dir: &Path,
    manifest: &serde_json::Value,
    sources: &[SourceRow],
    jobs: &[JobRow],
    logs: &[LogRow],
    generated_unix: i64,
) -> String {
    let field = |key: &str| -> String {
        manifest
            .get(key)
            .and_then(|v| v.as_str())
            .map(esc)
            .unwrap_or_else(|| "—".to_string())
    };
    let case_dir_disp = esc(&case_dir.display().to_string());
    let generated = ts(generated_unix);

    let mut src_rows = String::new();
    for s in sources {
        src_rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td class=path>{}</td><td>{}</td><td>{}</td><td>{}</td><td class=hash>{}</td></tr>\n",
            esc(&s.source_id),
            esc(&s.kind),
            esc(&s.path),
            ts(s.registered_unix),
            opt(&s.write_protect),
            opt(&s.acquisition_tool),
            opt(&s.evidence_hash),
        ));
    }
    if sources.is_empty() {
        src_rows
            .push_str("<tr><td colspan=7 class=empty>No evidence sources registered.</td></tr>\n");
    }

    let mut job_rows = String::new();
    for j in jobs {
        let units = match j.total_units {
            Some(t) => format!("{}/{}", j.completed_units, t),
            None => format!("{}", j.completed_units),
        };
        let status = match &j.error {
            Some(err) => format!("{}: {}", j.status, err),
            None => j.status.clone(),
        };
        job_rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td class=path>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            esc(&j.job_id),
            esc(&j.job_type),
            esc(&status),
            esc(&j.subject_path),
            ts(j.started_unix),
            j.completed_unix.map(ts).unwrap_or_else(|| "—".to_string()),
            esc(&units),
        ));
    }
    if jobs.is_empty() {
        job_rows.push_str("<tr><td colspan=7 class=empty>No jobs recorded.</td></tr>\n");
    }

    let mut log_rows = String::new();
    let mut verified = 0usize;
    let mut broken = 0usize;
    for l in logs {
        match &l.result {
            Ok(v) => {
                verified += 1;
                let caveat = if v.warnings.is_empty() {
                    String::new()
                } else {
                    format!(" — {}", esc(&v.warnings.join("; ")))
                };
                log_rows.push_str(&format!(
                    "<tr><td class=path>{}</td><td>{}</td><td class=ok>{}</td><td class=hash>{}</td><td>{}{}</td></tr>\n",
                    esc(&l.rel_path),
                    v.entries,
                    v.integrity.label(),
                    esc(&v.last_entry_sha256[..v.last_entry_sha256.len().min(16)]),
                    v.keyed_entries,
                    caveat,
                ));
            }
            Err(err) => {
                broken += 1;
                log_rows.push_str(&format!(
                    "<tr><td class=path>{}</td><td colspan=4 class=bad>verification failed: {}</td></tr>\n",
                    esc(&l.rel_path),
                    esc(err),
                ));
            }
        }
    }
    if logs.is_empty() {
        log_rows.push_str("<tr><td colspan=5 class=empty>No chained audit logs found.</td></tr>\n");
    }

    let integrity_banner = if broken > 0 {
        "<p class=banner bad>WARNING: one or more audit lanes failed verification — inspect the ledger below before relying on this package.</p>"
    } else if verified > 0 {
        "<p class=banner ok>All discovered chained audit logs verified structurally at statement generation time.</p>"
    } else {
        "<p class=banner>No chained audit logs were present to verify.</p>"
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8">
<title>Chain of Custody Statement — {case_id}</title>
<style>
body {{ font-family: -apple-system, "Segoe UI", Malgun Gothic, sans-serif; max-width: 980px; margin: 24px auto; padding: 0 20px; color: #1a1a1a; font-size: 13px; }}
h1 {{ font-size: 20px; border-bottom: 3px solid #2c5f8a; padding-bottom: 8px; }}
h2 {{ font-size: 15px; margin-top: 28px; color: #2c5f8a; }}
table {{ border-collapse: collapse; width: 100%; margin-top: 8px; }}
th, td {{ border: 1px solid #ccc; padding: 5px 8px; text-align: left; vertical-align: top; }}
th {{ background: #eef4f8; }}
td.path {{ font-family: Consolas, monospace; font-size: 11px; word-break: break-all; }}
td.hash {{ font-family: Consolas, monospace; font-size: 11px; }}
td.empty {{ color: #888; font-style: italic; }}
td.ok {{ color: #1d7a38; }}
td.bad, p.banner.bad {{ color: #b3261e; }}
p.banner {{ padding: 10px 14px; border-radius: 6px; font-weight: 600; }}
p.banner.ok {{ background: #e6f4ea; color: #1d7a38; }}
p.banner.bad {{ background: #fce8e6; }}
p.banner:not(.ok):not(.bad) {{ background: #f1f3f4; color: #444; }}
.meta td:first-child {{ font-weight: 600; width: 200px; background: #f7f9fb; }}
.note {{ font-size: 12px; color: #555; }}
@media print {{ body {{ margin: 0; }} }}
</style></head><body>
<h1>Chain of Custody Statement</h1>
<p class=note>Generated {generated} by FrameTrace — verified live at generation time. Print to PDF for submission if required.</p>
{integrity_banner}
<h2>1. Case Identity</h2>
<table class=meta>
<tr><td>Case ID</td><td>{case_id}</td></tr>
<tr><td>Title</td><td>{title}</td></tr>
<tr><td>Operator</td><td>{operator}</td></tr>
<tr><td>Host</td><td>{host}</td></tr>
<tr><td>Created</td><td>{created}</td></tr>
<tr><td>Tool</td><td>{tool} v{tool_version} ({platform})</td></tr>
<tr><td>Case directory</td><td class=path>{case_dir_disp}</td></tr>
</table>
<h2>2. Evidence Sources</h2>
<table><tr><th>Source ID</th><th>Kind</th><th>Path</th><th>Registered (UTC)</th><th>Write-protect</th><th>Acquisition tool</th><th>Evidence hash</th></tr>
{src_rows}</table>
<h2>3. Processing Ledger</h2>
<table><tr><th>Job ID</th><th>Type</th><th>Status</th><th>Subject</th><th>Started (UTC)</th><th>Completed (UTC)</th><th>Units</th></tr>
{job_rows}</table>
<h2>4. Audit Trail Integrity</h2>
<table><tr><th>Log</th><th>Entries</th><th>Integrity</th><th>Last entry SHA-256</th><th>Keyed</th></tr>
{log_rows}</table>
<h2>5. Scope Statement</h2>
<p>Original evidence files registered in section 2 were not modified by this tool: all derived
outputs (proxies, clips, thumbnails, carved candidates, recovered entries, telemetry reports)
are work products written under the case directory and logged to the chained audit lanes in
section 4. Carved and recovered items are candidates produced by signature/filesystem analysis
and must not be represented as authenticated originals. Hash verification of any packaged file
can be performed against <code>manifest.sha256</code> / <code>package-manifest.json</code>.</p>
<p class=note>Integrity labels: <code>integrity-keyed</code> = every entry HMAC-verified against a
configured key; <code>integrity-structural-only</code> = hash-chain structure verified but entries
are unsigned or unauthenticated. A failed lane is disclosed in section 4 rather than omitted.</p>
</body></html>"#,
        case_id = field("case_id"),
        title = field("title"),
        operator = field("operator"),
        host = field("host"),
        created = manifest
            .get("created_unix")
            .and_then(|v| v.as_i64())
            .map(ts)
            .unwrap_or_else(|| "—".to_string()),
        tool = field("tool_name"),
        tool_version = field("tool_version"),
        platform = field("platform"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_case_dir() {
        let dir = std::env::temp_dir().join(format!("ft-custody-{}", now_unix().unwrap()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(generate(&dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn renders_utc_timestamp() {
        assert_eq!(ts(1704110400), "2024-01-01 12:00:00Z");
        assert_eq!(ts(0), "—");
    }
}
