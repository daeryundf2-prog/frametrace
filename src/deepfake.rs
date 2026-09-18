//! deepfake-lens sidecar lane.
//!
//! deepfake-lens (Python, local-first) screens a file for synthetic-media
//! signals — images, audio, video, text/documents, archives. It is invoked
//! as a subprocess exactly like ffprobe: `deepfake-lens forensic <file>
//! --format json` prints a stable JSON report on stdout.
//!
//! IMPORTANT CONTRACT: deepfake-lens scores are review-priority signals,
//! not authenticity verdicts. Any UI/record surface that shows them must
//! keep that framing — "high" means "review first", never "confirmed
//! synthetic".
//!
//! Resolution order for the tool binary:
//!   1. `FRAMETRACE_DEEPFAKE_LENS` env var (full path or bare name)
//!   2. `deepfake-lens` on PATH (i.e. `pip install deepfake-lens`)
//!
//! Install: `pip install deepfake-lens` (or `pip install -e D:\devin\deepfake-lens`
//! for the local checkout). Optional model weights live under the
//! package's models/ dir; missing weights degrade to heuristic scoring.

use crate::tool_policy::resolve_tool_binary;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Neural-model analysis can run for minutes on CPU-only machines.
pub const DEEPFAKE_TIMEOUT_SECS: u64 = 900;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeepfakeSummary {
    pub ok: bool,
    pub score: Option<i64>,
    pub band: Option<String>,
    pub band_label: Option<String>,
    pub verdict: Option<String>,
    pub signal_titles: Vec<String>,
    pub limitation_count: usize,
    pub has_c2pa: bool,
    pub has_synthid: bool,
    pub has_watermark: bool,
    pub raw_json: Option<String>,
    pub error: Option<String>,
}

impl DeepfakeSummary {
    fn failed(error: impl Into<String>) -> Self {
        Self {
            ok: false,
            error: Some(error.into()),
            ..Self::default()
        }
    }
}

#[derive(Debug, Deserialize)]
struct ForensicSignal {
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ForensicReport {
    score: Option<i64>,
    band: Option<String>,
    band_label: Option<String>,
    verdict: Option<String>,
    signals: Option<Vec<ForensicSignal>>,
    limitations: Option<Vec<serde_json::Value>>,
    has_c2pa: Option<bool>,
    has_synthid: Option<bool>,
    has_watermark: Option<bool>,
}

/// Screen `path` through the deepfake-lens forensic report.
pub fn screen(path: &Path) -> DeepfakeSummary {
    let configured =
        std::env::var("FRAMETRACE_DEEPFAKE_LENS").unwrap_or_else(|_| "deepfake-lens".to_string());
    screen_with_binary(&configured, path)
}

pub fn screen_with_binary(binary: &str, path: &Path) -> DeepfakeSummary {
    let mut command = match resolve_tool_binary(binary, &["deepfake-lens", "deepfake-lens.exe"]) {
        Ok(binary) => {
            let mut command = Command::new(&binary);
            command
                .arg("forensic")
                .arg(path)
                .arg("--format")
                .arg("json");
            command
        }
        Err(binary_error) => {
            // No console script on PATH — fall back to the interpreter
            // entry point (`python -m deepfake_lens.cli ...`), which works
            // for source checkouts and venv installs alike.
            let python = match resolve_tool_binary("python", &["python", "python3", "py"]) {
                Ok(python) => python,
                Err(python_error) => {
                    return DeepfakeSummary::failed(format!(
                        "{binary_error}; python fallback also unavailable: {python_error}"
                    ));
                }
            };
            let mut command = Command::new(&python);
            command
                .arg("-m")
                .arg("deepfake_lens.cli")
                .arg("forensic")
                .arg(path)
                .arg("--format")
                .arg("json");
            // Source checkouts aren't importable by default; point the
            // interpreter at the repo root when the operator tells us
            // where it lives.
            if let Ok(home) = std::env::var("FRAMETRACE_DEEPFAKE_LENS_HOME") {
                let existing = std::env::var("PYTHONPATH").unwrap_or_default();
                let merged = if existing.is_empty() {
                    home
                } else {
                    let sep = if cfg!(windows) { ";" } else { ":" };
                    format!("{home}{sep}{existing}")
                };
                command.env("PYTHONPATH", merged);
            }
            command
        }
    };

    let output = crate::util::run_with_timeout(&mut command, Some(DEEPFAKE_TIMEOUT_SECS));
    let output = match output {
        Ok(output) => output,
        Err(error) => {
            return DeepfakeSummary::failed(format!("failed to run deepfake-lens: {error}"));
        }
    };
    if !output.status.success() {
        return DeepfakeSummary::failed(format!(
            "deepfake-lens exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let report: ForensicReport = match serde_json::from_str(&raw) {
        Ok(report) => report,
        Err(error) => {
            return DeepfakeSummary::failed(format!("unparseable deepfake-lens JSON: {error}"));
        }
    };

    DeepfakeSummary {
        ok: true,
        score: report.score,
        band: report.band,
        band_label: report.band_label,
        verdict: report.verdict,
        signal_titles: report
            .signals
            .unwrap_or_default()
            .into_iter()
            .filter_map(|s| s.title)
            .collect(),
        limitation_count: report.limitations.map(|v| v.len()).unwrap_or(0),
        has_c2pa: report.has_c2pa.unwrap_or(false),
        has_synthid: report.has_synthid.unwrap_or(false),
        has_watermark: report.has_watermark.unwrap_or(false),
        raw_json: Some(raw),
        error: None,
    }
}

/// Collects every `artifacts/deepfake/<id>.json` report under a case into a
/// `{video_id: report}` JSON object for the evidence-viewer data bundle.
/// Missing/unreadable/invalid reports are skipped — a partial artifact must
/// not blank the generated page.
pub fn collect_reports(case_dir: &Path) -> serde_json::Value {
    let dir = case_dir.join("artifacts/deepfake");
    let mut map = serde_json::Map::new();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(_) => return serde_json::Value::Object(map),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if let Ok(report) = serde_json::from_str::<serde_json::Value>(&text) {
            map.insert(id.to_string(), report);
        }
    }
    serde_json::Value::Object(map)
}

/// One screenable record inside a case: the viewer-facing record id plus
/// the on-disk file to analyze.
pub struct ScreenTarget {
    pub id: String,
    pub path: std::path::PathBuf,
    pub origin: &'static str,
}

/// Artifact filenames derive from record ids, which can contain
/// characters that are illegal on Windows (`inode:<offset>:<ino>`,
/// `fls:<ino>`). Sanitize once — the viewer applies the same mapping
/// when looking up reports in its data bundle.
pub fn artifact_name(id: &str) -> String {
    id.chars()
        .map(|c| {
            if matches!(c, ':' | '\\' | '/') {
                '_'
            } else {
                c
            }
        })
        .collect()
}

/// Enumerates every screenable record in a case — indexed logical files,
/// carved candidates, filesystem-recovered files — so a post-import pass
/// covers E01/carve/recover pipelines that never pass through
/// `scan-folder`.
pub fn collect_targets(case_dir: &Path) -> Vec<ScreenTarget> {
    let mut out = Vec::new();
    collect_jsonl_targets(
        &case_dir.join("db/videos.jsonl"),
        "id",
        "source_path",
        "index",
        &mut out,
    );
    collect_jsonl_targets(
        &case_dir.join("artifacts/carved/carve-log.jsonl"),
        "id",
        "output_path",
        "carve",
        &mut out,
    );
    // Filesystem recovery records use the viewer's composite id
    // (`inode:<partition_offset>:<inode>`) so the badge maps correctly.
    if let Ok(text) = fs::read_to_string(case_dir.join("evidence/logs/tsk-audit.jsonl")) {
        for line in text.lines() {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
                continue;
            };
            if v.get("event").and_then(|e| e.as_str()) != Some("recover-inode") {
                continue;
            }
            let Some(path) = v.get("output_path").and_then(|x| x.as_str()) else {
                continue;
            };
            let offset = v
                .get("partition_offset")
                .and_then(|x| x.as_u64())
                .unwrap_or(0);
            let inode = v.get("inode").and_then(|x| x.as_str()).unwrap_or(path);
            out.push(ScreenTarget {
                id: format!("inode:{offset}:{inode}"),
                path: std::path::PathBuf::from(path),
                origin: "recover",
            });
        }
    }
    out
}

fn collect_jsonl_targets(
    path: &Path,
    id_key: &str,
    path_key: &str,
    origin: &'static str,
    out: &mut Vec<ScreenTarget>,
) {
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    for line in text.lines() {
        // A torn tail line (documented crash-survivability mode) is not
        // valid JSON — skip it rather than aborting the pass.
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
            continue;
        };
        let (Some(id), Some(file)) = (
            v.get(id_key).and_then(|x| x.as_str()),
            v.get(path_key).and_then(|x| x.as_str()),
        ) else {
            continue;
        };
        out.push(ScreenTarget {
            id: id.to_string(),
            path: std::path::PathBuf::from(file),
            origin,
        });
    }
}

#[derive(Default)]
pub struct ScreenCaseStats {
    pub screened: usize,
    pub skipped_existing: usize,
    pub skipped_missing: usize,
    pub failed: usize,
}

/// Screens every case record that lacks a deepfake artifact (or all of
/// them with `force`), writing `artifacts/deepfake/<id>.json` per record.
/// Individual failures never abort the pass — a case can contain a file
/// deepfake-lens cannot parse.
pub fn screen_case(
    case_dir: &Path,
    force: bool,
    progress: &dyn Fn(usize, usize, &str),
) -> Result<ScreenCaseStats, String> {
    let targets = collect_targets(case_dir);
    let artifact_dir = case_dir.join("artifacts/deepfake");
    fs::create_dir_all(&artifact_dir)
        .map_err(|err| format!("deepfake artifact dir failed: {err}"))?;
    let total = targets.len();
    let mut stats = ScreenCaseStats::default();
    for (idx, target) in targets.into_iter().enumerate() {
        progress(idx, total, &target.id);
        let artifact = artifact_dir.join(format!("{}.json", artifact_name(&target.id)));
        if artifact.is_file() && !force {
            stats.skipped_existing += 1;
            continue;
        }
        if !target.path.is_file() {
            stats.skipped_missing += 1;
            continue;
        }
        let screening = screen(&target.path);
        let screen_error = screening.error.clone();
        let body = screening.raw_json.clone().unwrap_or_else(|| {
            format!(
                "{{\"ok\":false,\"error\":\"{}\",\"origin\":\"{}\"}}",
                crate::util::json_escape(screen_error.as_deref().unwrap_or("unknown")),
                target.origin
            )
        });
        match crate::util::write_text_atomic(&artifact, &body) {
            Ok(()) => {
                if screen_error.is_some() {
                    stats.failed += 1;
                } else {
                    stats.screened += 1;
                }
            }
            Err(_) => stats.failed += 1,
        }
    }
    progress(total, total, "");
    Ok(stats)
}
