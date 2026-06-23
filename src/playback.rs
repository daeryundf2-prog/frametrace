use crate::audit;
use crate::media_contract;
use crate::util::{json_escape, now_unix, read_to_string};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct PlaybackConfirmationOptions {
    pub operator: Option<String>,
    pub playback_tool: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlaybackConfirmation {
    pub selector: String,
    pub target_path: PathBuf,
    pub target_sha256: String,
    pub validation_status: String,
    pub prior_validation_status: String,
    pub operator: String,
    pub confirmed_unix: u64,
}

struct PriorValidation {
    target_path: PathBuf,
    target_sha256: String,
    source_artifact_id: Option<String>,
    source_artifact_path: Option<String>,
    source_artifact_sha256: Option<String>,
    derived_artifact_id: Option<String>,
    target_artifact_id: Option<String>,
    validation_status: String,
    entry_sha256: Option<String>,
}

pub fn confirm_playback(
    case_dir: &Path,
    selector: &str,
    options: &PlaybackConfirmationOptions,
) -> Result<PlaybackConfirmation, String> {
    let prior = find_prior_ffprobe_validation(case_dir, selector)?;
    let operator = media_contract::resolve_operator(case_dir, options.operator.as_deref())?;
    let confirmed_unix = now_unix()?;
    let result = PlaybackConfirmation {
        selector: selector.to_string(),
        target_path: prior.target_path.clone(),
        target_sha256: prior.target_sha256.clone(),
        validation_status: "playback-confirmed".to_string(),
        prior_validation_status: prior.validation_status.clone(),
        operator,
        confirmed_unix,
    };
    let body = playback_log_body_json(&result, &prior, options);
    audit::append_chained_jsonl(&case_dir.join("evidence/logs/validation-log.jsonl"), &body)?;
    Ok(result)
}

fn find_prior_ffprobe_validation(
    case_dir: &Path,
    selector: &str,
) -> Result<PriorValidation, String> {
    let log_path = case_dir.join("evidence/logs/validation-log.jsonl");
    let text = read_to_string(&log_path)
        .map_err(|err| format!("playback confirmation requires prior ffprobe-video-stream-confirmed validation; failed to read {}: {err}", log_path.display()))?;

    text.lines()
        .rev()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| prior_validation_from_line(line, selector))
        .find(|prior| prior.validation_status == "ffprobe-video-stream-confirmed")
        .ok_or_else(|| {
            format!(
                "playback confirmation requires prior ffprobe-video-stream-confirmed validation for {selector}"
            )
        })
}

fn prior_validation_from_line(line: &str, selector: &str) -> Option<PriorValidation> {
    let line_selector = extract_json_string(line, "selector");
    let source_artifact_id = extract_json_string(line, "source_artifact_id");
    let derived_artifact_id = extract_json_string(line, "derived_artifact_id");
    let target_artifact_id = extract_json_string(line, "target_artifact_id");
    let target_path = extract_json_string(line, "target_path");
    let matches = line_selector.as_deref() == Some(selector)
        || source_artifact_id.as_deref() == Some(selector)
        || derived_artifact_id.as_deref() == Some(selector)
        || target_artifact_id.as_deref() == Some(selector)
        || target_path.as_deref() == Some(selector);
    if !matches {
        return None;
    }

    Some(PriorValidation {
        target_path: PathBuf::from(target_path?),
        target_sha256: extract_json_string(line, "target_sha256")?,
        source_artifact_id,
        source_artifact_path: extract_json_string(line, "source_artifact_path"),
        source_artifact_sha256: extract_json_string(line, "source_artifact_sha256"),
        derived_artifact_id,
        target_artifact_id,
        validation_status: extract_json_string(line, "validation_status")?,
        entry_sha256: extract_json_string(line, "entry_sha256"),
    })
}

fn playback_log_body_json(
    result: &PlaybackConfirmation,
    prior: &PriorValidation,
    options: &PlaybackConfirmationOptions,
) -> String {
    format!(
        "{{\"schema_version\":3,\"event\":\"confirm-playback\",\"confirmed_unix\":{},\"selector\":\"{}\",\"operator\":\"{}\",\"method\":\"manual-playback-confirmation\",\"tool\":\"manual-playback\",\"playback_tool\":{},\"playback_verified\":true,\"source_artifact_id\":{},\"source_artifact_path\":{},\"source_artifact_sha256\":{},\"derived_artifact_id\":{},\"target_artifact_id\":{},\"target_path\":\"{}\",\"target_sha256\":\"{}\",\"validation_status\":\"{}\",\"prior_validation_status\":\"{}\",\"prior_validation_entry_sha256\":{},\"validation_note\":\"examiner playback review confirmed playable media after ffprobe stream validation\",\"notes\":{}}}",
        result.confirmed_unix,
        json_escape(&result.selector),
        json_escape(&result.operator),
        audit::optional_string(options.playback_tool.as_deref()),
        audit::optional_string(prior.source_artifact_id.as_deref()),
        audit::optional_string(prior.source_artifact_path.as_deref()),
        audit::optional_string(prior.source_artifact_sha256.as_deref()),
        audit::optional_string(prior.derived_artifact_id.as_deref()),
        audit::optional_string(prior.target_artifact_id.as_deref()),
        json_escape(&result.target_path.to_string_lossy()),
        json_escape(&result.target_sha256),
        json_escape(&result.validation_status),
        json_escape(&result.prior_validation_status),
        audit::optional_string(prior.entry_sha256.as_deref()),
        audit::optional_string(options.notes.as_deref())
    )
}

fn extract_json_string(line: &str, key: &str) -> Option<String> {
    let key = format!("\"{}\":", key);
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
                'b' => out.push('\u{08}'),
                'f' => out.push('\u{0C}'),
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
