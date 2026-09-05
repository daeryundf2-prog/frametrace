//! Candidate anomaly findings for examiner review (ROADMAP M4).
//!
//! Labels are `candidate-finding` only — never claim tampering, authenticity
//! failure, or legal proof. Findings are written to
//! `evidence/logs/anomaly-log.jsonl` and surfaced in the case report.

use crate::audit;
use crate::util::{json_escape, now_unix, read_to_string};
use std::path::{Path, PathBuf};

pub const LABEL: &str = "candidate-finding";
const GAP_SECS: u64 = 7 * 24 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub kind: &'static str,
    pub selector: String,
    pub source_path: String,
    pub detail: String,
}

#[derive(Debug, Clone)]
struct IndexedRow {
    id: String,
    source_path: String,
    sha256: Option<String>,
    modified_unix: Option<u64>,
    duration_seconds: Option<f64>,
    format_name: Option<String>,
    video_codec: Option<String>,
    ffprobe_ok: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct AnomalyScanResult {
    pub findings: Vec<Finding>,
    pub log_path: PathBuf,
    pub scanned_unix: u64,
}

/// Scan the case index for candidate anomalies and append a chained log.
pub fn scan_case(case_dir: &Path) -> Result<AnomalyScanResult, String> {
    let rows = read_indexed_rows(case_dir)?;
    let mut findings = Vec::new();
    findings.extend(hash_revalidation_findings(&rows));
    findings.extend(timestamp_findings(&rows));
    findings.extend(container_stream_findings(&rows));

    let scanned_unix = now_unix()?;
    let log_path = case_dir.join("evidence/logs/anomaly-log.jsonl");
    for finding in &findings {
        let line = format!(
            "{{\"schema_version\":1,\"event\":\"anomaly-scan\",\"scanned_unix\":{},\"label\":\"{}\",\"kind\":\"{}\",\"selector\":\"{}\",\"source_path\":\"{}\",\"detail\":\"{}\"}}",
            scanned_unix,
            LABEL,
            json_escape(finding.kind),
            json_escape(&finding.selector),
            json_escape(&finding.source_path),
            json_escape(&finding.detail),
        );
        audit::append_chained_jsonl(&log_path, &line)?;
    }
    if findings.is_empty() {
        let line = format!(
            "{{\"schema_version\":1,\"event\":\"anomaly-scan\",\"scanned_unix\":{},\"label\":\"none\",\"kind\":\"none\",\"selector\":\"\",\"source_path\":\"\",\"detail\":\"no candidate findings on indexed videos\"}}",
            scanned_unix
        );
        audit::append_chained_jsonl(&log_path, &line)?;
    }
    Ok(AnomalyScanResult {
        findings,
        log_path,
        scanned_unix,
    })
}

/// Compare a live digest against the indexed hash for one selector, if known.
pub fn hash_mismatch_finding(
    case_dir: &Path,
    selector: &str,
    live_sha256: &str,
) -> Result<Option<Finding>, String> {
    let rows = read_indexed_rows(case_dir)?;
    let Some(row) = rows.iter().find(|row| row.id == selector) else {
        return Ok(None);
    };
    let Some(indexed) = row.sha256.as_deref() else {
        return Ok(None);
    };
    if indexed.eq_ignore_ascii_case(live_sha256) {
        return Ok(None);
    }
    Ok(Some(Finding {
        kind: "hash-revalidation-mismatch",
        selector: row.id.clone(),
        source_path: row.source_path.clone(),
        detail: format!("indexed sha256 {indexed} != live sha256 {live_sha256}"),
    }))
}

fn hash_revalidation_findings(rows: &[IndexedRow]) -> Vec<Finding> {
    let mut out = Vec::new();
    for row in rows {
        let Some(indexed) = row.sha256.as_deref() else {
            continue;
        };
        let path = Path::new(&row.source_path);
        if !path.is_file() {
            continue;
        }
        let Ok(live) = audit::digest_file(path) else {
            continue;
        };
        if !indexed.eq_ignore_ascii_case(&live) {
            out.push(Finding {
                kind: "hash-revalidation-mismatch",
                selector: row.id.clone(),
                source_path: row.source_path.clone(),
                detail: format!("indexed sha256 {indexed} != live sha256 {live}"),
            });
        }
    }
    out
}

fn timestamp_findings(rows: &[IndexedRow]) -> Vec<Finding> {
    let mut by_parent: std::collections::BTreeMap<String, Vec<&IndexedRow>> =
        std::collections::BTreeMap::new();
    for row in rows {
        if row.modified_unix.is_none() {
            continue;
        }
        let parent = Path::new(&row.source_path)
            .parent()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default();
        by_parent.entry(parent).or_default().push(row);
    }

    let mut out = Vec::new();
    for (_parent, mut group) in by_parent {
        group.sort_by(|a, b| a.source_path.cmp(&b.source_path));
        for window in group.windows(2) {
            let left = window[0];
            let right = window[1];
            let left_ts = left.modified_unix.unwrap_or(0);
            let right_ts = right.modified_unix.unwrap_or(0);
            if right_ts < left_ts {
                out.push(Finding {
                    kind: "timestamp-regression",
                    selector: right.id.clone(),
                    source_path: right.source_path.clone(),
                    detail: format!(
                        "path-sorted after {} but mtime {} < {}",
                        left.source_path, right_ts, left_ts
                    ),
                });
            } else if right_ts.saturating_sub(left_ts) > GAP_SECS {
                out.push(Finding {
                    kind: "timestamp-gap",
                    selector: right.id.clone(),
                    source_path: right.source_path.clone(),
                    detail: format!(
                        "mtime gap {}s from {} (threshold {}s)",
                        right_ts - left_ts,
                        left.source_path,
                        GAP_SECS
                    ),
                });
            }
        }
    }
    out
}

fn container_stream_findings(rows: &[IndexedRow]) -> Vec<Finding> {
    let mut out = Vec::new();
    for row in rows {
        if row.ffprobe_ok != Some(true) {
            continue;
        }
        let format = row
            .format_name
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase();
        let codec = row
            .video_codec
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase();
        if (format.contains("image2") || format == "png_pipe" || format == "jpeg_pipe")
            && !codec.is_empty()
        {
            out.push(Finding {
                kind: "container-stream-mismatch",
                selector: row.id.clone(),
                source_path: row.source_path.clone(),
                detail: format!(
                    "format `{format}` looks like still image but video codec is `{codec}`"
                ),
            });
        }
        if let Some(duration) = row.duration_seconds
            && duration == 0.0
            && !codec.is_empty()
        {
            out.push(Finding {
                kind: "container-stream-mismatch",
                selector: row.id.clone(),
                source_path: row.source_path.clone(),
                detail: format!("duration 0 with video codec `{codec}`"),
            });
        }
    }
    out
}

fn read_indexed_rows(case_dir: &Path) -> Result<Vec<IndexedRow>, String> {
    let path = case_dir.join("db/videos.jsonl");
    let text = match read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(format!(
                "failed to read indexed videos {}: {err}",
                path.display()
            ));
        }
    };
    Ok(text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| IndexedRow {
            id: extract_json_string(line, "id").unwrap_or_default(),
            source_path: extract_json_string(line, "source_path").unwrap_or_default(),
            sha256: extract_json_string(line, "sha256"),
            modified_unix: extract_json_u64(line, "modified_unix"),
            duration_seconds: extract_json_f64(line, "duration_seconds"),
            format_name: extract_json_string(line, "format_name"),
            video_codec: extract_json_string(line, "video_codec"),
            ffprobe_ok: extract_json_bool(line, "ffprobe_ok"),
        })
        .filter(|row| !row.id.is_empty())
        .collect())
}

fn extract_json_string(line: &str, key: &str) -> Option<String> {
    let key = format!("\"{key}\":");
    let start = line.find(&key)? + key.len();
    let value = line[start..].trim_start();
    if value.starts_with("null") {
        return None;
    }
    let value = value.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                other => out.push(other),
            },
            other => out.push(other),
        }
    }
    None
}

fn extract_json_u64(line: &str, key: &str) -> Option<u64> {
    let key = format!("\"{key}\":");
    let start = line.find(&key)? + key.len();
    let value = line[start..].trim_start();
    if value.starts_with("null") {
        return None;
    }
    value
        .split(|ch: char| !ch.is_ascii_digit())
        .next()
        .and_then(|digits| digits.parse().ok())
}

fn extract_json_f64(line: &str, key: &str) -> Option<f64> {
    let key = format!("\"{key}\":");
    let start = line.find(&key)? + key.len();
    let value = line[start..].trim_start();
    if value.starts_with("null") {
        return None;
    }
    let end = value
        .find(|ch: char| !(ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == 'e' || ch == 'E'))
        .unwrap_or(value.len());
    value[..end].parse().ok()
}

fn extract_json_bool(line: &str, key: &str) -> Option<bool> {
    let key = format!("\"{key}\":");
    let start = line.find(&key)? + key.len();
    let value = line[start..].trim_start();
    if value.starts_with("true") {
        Some(true)
    } else if value.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_timestamp_regression_within_folder() {
        let rows = vec![
            IndexedRow {
                id: "vid_1".into(),
                source_path: "C:/ev/a.mp4".into(),
                sha256: None,
                modified_unix: Some(200),
                duration_seconds: None,
                format_name: None,
                video_codec: None,
                ffprobe_ok: None,
            },
            IndexedRow {
                id: "vid_2".into(),
                source_path: "C:/ev/b.mp4".into(),
                sha256: None,
                modified_unix: Some(100),
                duration_seconds: None,
                format_name: None,
                video_codec: None,
                ffprobe_ok: None,
            },
        ];
        let findings = timestamp_findings(&rows);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "timestamp-regression");
        assert_eq!(findings[0].selector, "vid_2");
    }

    #[test]
    fn detects_zero_duration_with_video_codec() {
        let rows = vec![IndexedRow {
            id: "vid_1".into(),
            source_path: "C:/ev/a.mp4".into(),
            sha256: None,
            modified_unix: None,
            duration_seconds: Some(0.0),
            format_name: Some("mov,mp4,m4a,3gp,3g2,mj2".into()),
            video_codec: Some("h264".into()),
            ffprobe_ok: Some(true),
        }];
        let findings = container_stream_findings(&rows);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "container-stream-mismatch");
    }

    #[test]
    fn label_is_candidate_finding_not_legal_claim() {
        assert_eq!(LABEL, "candidate-finding");
        assert!(!LABEL.contains("tamper"));
        assert!(!LABEL.contains("authentic"));
    }
}
