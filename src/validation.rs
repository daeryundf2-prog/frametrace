use crate::audit;
use crate::ffprobe;
use crate::model::ProbeSummary;
use crate::tool_policy::command_version;
use crate::util::{canonicalize_display, json_escape, now_unix, read_to_string};
use crate::video_export::resolve_video_source;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ValidationOptions {
    pub ffprobe_bin: String,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            ffprobe_bin: "ffprobe".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub selector: String,
    pub target_path: PathBuf,
    pub target_sha256: String,
    pub validation_status: String,
    pub validation_note: String,
    pub probe: ProbeSummary,
    pub validated_unix: u64,
}

pub fn validate_artifact(
    case_dir: &Path,
    selector: &str,
    options: &ValidationOptions,
) -> Result<ValidationResult, String> {
    let result = compute_validation(case_dir, selector, options)?;
    append_validation_log(case_dir, &result, options)?;
    Ok(result)
}

/// Compute phase of a validation (resolve target, hash, ffprobe) with no side
/// effects on the case. Batch commands run this in parallel and then append
/// the validation log entries sequentially so the hash chain stays intact.
pub fn compute_validation(
    case_dir: &Path,
    selector: &str,
    options: &ValidationOptions,
) -> Result<ValidationResult, String> {
    let target_path = resolve_artifact_path(case_dir, selector)?;
    let validated_unix = now_unix()?;
    let target_sha256 = audit::digest_file(&target_path)?;
    let probe = ffprobe::probe_with_binary(&options.ffprobe_bin, &target_path);
    let (validation_status, validation_note) = validation_status(&probe);
    Ok(ValidationResult {
        selector: selector.to_string(),
        target_path,
        target_sha256,
        validation_status: validation_status.to_string(),
        validation_note: validation_note.to_string(),
        probe,
        validated_unix,
    })
}

/// Resolves an indexed video id, artifact id, inode recovery id, or direct
/// path to an existing file. Shared by validate-artifact and the batch
/// commands so carved/recovered selectors work everywhere.
pub fn resolve_artifact_path(case_dir: &Path, selector: &str) -> Result<PathBuf, String> {
    let direct = PathBuf::from(selector);
    if direct.is_file() {
        return canonicalize_display(&direct)
            .map_err(|err| format!("failed to canonicalize validation target: {err}"));
    }

    if let Ok(path) = resolve_video_source(case_dir, selector) {
        return Ok(path);
    }

    for rel_log in [
        "artifacts/carved/carve-log.jsonl",
        "artifacts/clips/export-log.jsonl",
        "artifacts/proxies/proxy-log.jsonl",
        "artifacts/thumbnails/thumbnail-log.jsonl",
        "evidence/logs/tsk-audit.jsonl",
    ] {
        let Some(path) = resolve_from_log(&case_dir.join(rel_log), selector) else {
            continue;
        };
        if path.is_file() {
            return canonicalize_display(&path)
                .map_err(|err| format!("failed to canonicalize validation target: {err}"));
        }
    }

    Err(format!(
        "validation target not found: {selector} (use an indexed video id, artifact id, inode recovery path, or direct file path)"
    ))
}

fn resolve_from_log(log_path: &Path, selector: &str) -> Option<PathBuf> {
    let text = read_to_string(log_path).ok()?;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let id = extract_json_string(line, "id");
        let inode = extract_json_string(line, "inode");
        let output_path = extract_json_string(line, "output_path");
        let selector_field = extract_json_string(line, "selector");
        let matches = id.as_deref() == Some(selector)
            || inode.as_deref() == Some(selector)
            || selector_field.as_deref() == Some(selector)
            || output_path.as_deref() == Some(selector);
        if matches {
            return output_path.map(PathBuf::from);
        }
    }
    None
}

fn validation_status(probe: &ProbeSummary) -> (&'static str, &'static str) {
    if !probe.ok {
        return (
            "validation-failed",
            "ffprobe could not parse the file; keep as candidate until manual/vendor-player validation.",
        );
    }
    if probe.video_codec.is_none() {
        return (
            "validation-failed",
            "ffprobe parsed the container but found no video stream.",
        );
    }
    (
        "ffprobe-video-stream-confirmed",
        "ffprobe parsed a video stream; examiner playback review is still required before final reporting.",
    )
}

pub fn append_validation_log(
    case_dir: &Path,
    result: &ValidationResult,
    options: &ValidationOptions,
) -> Result<(), String> {
    let line = format!(
        "{{\"schema_version\":1,\"event\":\"validate-artifact\",\"validated_unix\":{},\"selector\":\"{}\",\"target_path\":\"{}\",\"target_sha256\":\"{}\",\"validation_status\":\"{}\",\"validation_note\":\"{}\",\"duration_seconds\":{},\"format_name\":{},\"video_codec\":{},\"audio_codec\":{},\"width\":{},\"height\":{},\"ffprobe_ok\":{},\"ffprobe_error\":{},\"ffprobe_version\":\"{}\",\"command\":\"{}\"}}",
        result.validated_unix,
        json_escape(&result.selector),
        json_escape(&result.target_path.to_string_lossy()),
        json_escape(&result.target_sha256),
        json_escape(&result.validation_status),
        json_escape(&result.validation_note),
        optional_f64(result.probe.duration_seconds),
        audit::optional_string(result.probe.format_name.as_deref()),
        audit::optional_string(result.probe.video_codec.as_deref()),
        audit::optional_string(result.probe.audio_codec.as_deref()),
        optional_u32(result.probe.width),
        optional_u32(result.probe.height),
        result.probe.ok,
        audit::optional_string(result.probe.error.as_deref()),
        json_escape(&command_version(
            &options.ffprobe_bin,
            &["ffprobe"],
            "-version"
        )),
        json_escape(&options.ffprobe_bin)
    );
    audit::append_chained_jsonl(&case_dir.join("evidence/logs/validation-log.jsonl"), &line)
}

fn optional_f64(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "null".to_string())
}

fn optional_u32(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
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
                'u' => {
                    let mut code = String::new();
                    for _ in 0..4 {
                        code.push(chars.next()?);
                    }
                    let code = u32::from_str_radix(&code, 16).ok()?;
                    out.push(char::from_u32(code)?);
                }
                other => out.push(other),
            },
            other => out.push(other),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{extract_json_string, resolve_from_log, validation_status};
    use crate::model::ProbeSummary;
    use std::fs;

    #[test]
    fn classifies_successful_video_probe_as_ffprobe_confirmed() {
        let probe = ProbeSummary {
            ok: true,
            raw_json: Some("{}".to_string()),
            error: None,
            duration_seconds: Some(1.0),
            format_name: Some("mov,mp4".to_string()),
            video_codec: Some("h264".to_string()),
            audio_codec: None,
            width: Some(1920),
            height: Some(1080),
        };
        assert_eq!(
            validation_status(&probe).0,
            "ffprobe-video-stream-confirmed"
        );
    }

    #[test]
    fn classifies_missing_video_stream_as_failed() {
        let probe = ProbeSummary {
            ok: true,
            raw_json: Some("{}".to_string()),
            error: None,
            duration_seconds: Some(1.0),
            format_name: Some("mp3".to_string()),
            video_codec: None,
            audio_codec: Some("mp3".to_string()),
            width: None,
            height: None,
        };
        assert_eq!(validation_status(&probe).0, "validation-failed");
    }

    #[test]
    fn resolves_artifact_path_from_jsonl_log() {
        let dir = std::env::temp_dir().join(format!(
            "frametrace-validation-log-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let log = dir.join("log.jsonl");
        fs::write(
            &log,
            r#"{"id":"carve_000001","output_path":"/tmp/out.mp4"}"#,
        )
        .unwrap();
        assert_eq!(
            resolve_from_log(&log, "carve_000001")
                .unwrap()
                .to_string_lossy(),
            "/tmp/out.mp4"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn extracts_escaped_json_strings() {
        assert_eq!(
            extract_json_string(r#"{"output_path":"C:\\Cases\\a.mp4"}"#, "output_path").as_deref(),
            Some("C:\\Cases\\a.mp4")
        );
    }
}
