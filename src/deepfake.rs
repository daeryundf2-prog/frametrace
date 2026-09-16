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
    let configured = std::env::var("FRAMETRACE_DEEPFAKE_LENS").unwrap_or_else(|_| "deepfake-lens".to_string());
    screen_with_binary(&configured, path)
}

pub fn screen_with_binary(binary: &str, path: &Path) -> DeepfakeSummary {
    let mut command = match resolve_tool_binary(binary, &["deepfake-lens", "deepfake-lens.exe"]) {
        Ok(binary) => {
            let mut command = Command::new(&binary);
            command.arg("forensic").arg(path).arg("--format").arg("json");
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
