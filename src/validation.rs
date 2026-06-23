use crate::audit;
use crate::ffprobe;
use crate::media_contract;
use crate::model::ProbeSummary;
use crate::tool_policy::{command_version, require_case_output_path};
use crate::util::{json_escape, now_unix, read_to_string};
use crate::video_export::resolve_video_source;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ValidationOptions {
    pub ffprobe_bin: String,
    pub operator: Option<String>,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            ffprobe_bin: "ffprobe".to_string(),
            operator: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub selector: String,
    pub target_path: PathBuf,
    pub target_sha256: String,
    pub source_artifact_id: Option<String>,
    pub source_artifact_path: Option<PathBuf>,
    pub source_artifact_sha256: Option<String>,
    pub derived_artifact_id: Option<String>,
    pub target_artifact_id: Option<String>,
    pub validation_status: String,
    pub validation_note: String,
    pub probe: ProbeSummary,
    pub validated_unix: u64,
}

#[derive(Debug, Clone)]
struct ValidationTarget {
    path: PathBuf,
    source_artifact_id: Option<String>,
    source_artifact_path: Option<PathBuf>,
    source_artifact_sha256: Option<String>,
    derived_artifact_id: Option<String>,
    target_artifact_id: Option<String>,
}

pub fn validate_artifact(
    case_dir: &Path,
    selector: &str,
    options: &ValidationOptions,
) -> Result<ValidationResult, String> {
    let target = resolve_validation_target(case_dir, selector)?;
    let validated_unix = now_unix()?;
    let target_sha256 = audit::digest_file(&target.path)?;
    let probe = ffprobe::probe_with_binary(&options.ffprobe_bin, &target.path);
    let (validation_status, validation_note) = validation_status(&probe);
    let result = ValidationResult {
        selector: selector.to_string(),
        target_path: target.path,
        target_sha256,
        source_artifact_id: target.source_artifact_id,
        source_artifact_path: target.source_artifact_path,
        source_artifact_sha256: target.source_artifact_sha256,
        derived_artifact_id: target.derived_artifact_id,
        target_artifact_id: target.target_artifact_id,
        validation_status: validation_status.to_string(),
        validation_note: validation_note.to_string(),
        probe,
        validated_unix,
    };
    append_validation_log(case_dir, &result, options)?;
    Ok(result)
}

fn resolve_validation_target(case_dir: &Path, selector: &str) -> Result<ValidationTarget, String> {
    let direct = PathBuf::from(selector);
    if direct.is_file() {
        let path = direct
            .canonicalize()
            .map_err(|err| format!("failed to canonicalize validation target: {err}"))?;
        return Ok(ValidationTarget {
            path,
            source_artifact_id: None,
            source_artifact_path: None,
            source_artifact_sha256: None,
            derived_artifact_id: None,
            target_artifact_id: None,
        });
    }

    if let Ok(path) = resolve_video_source(case_dir, selector) {
        let path = path
            .canonicalize()
            .map_err(|err| format!("failed to canonicalize validation target: {err}"))?;
        let source_artifact_sha256 = audit::indexed_source_hash(case_dir, selector, &path);
        let source_artifact_id = source_artifact_sha256
            .as_deref()
            .map(|sha256| media_contract::source_artifact_id(selector, sha256));
        return Ok(ValidationTarget {
            source_artifact_path: Some(path.clone()),
            path,
            source_artifact_id: source_artifact_id.clone(),
            source_artifact_sha256,
            derived_artifact_id: None,
            target_artifact_id: source_artifact_id,
        });
    }

    for rel_log in [
        "artifacts/carved/carve-log.jsonl",
        "artifacts/clips/export-log.jsonl",
        "artifacts/proxies/proxy-log.jsonl",
        "artifacts/thumbnails/thumbnail-log.jsonl",
        "artifacts/frames/frame-log.jsonl",
        "evidence/logs/tsk-audit.jsonl",
    ] {
        let Some(mut target) = resolve_from_log(&case_dir.join(rel_log), selector) else {
            continue;
        };
        if target.path.is_file() {
            target.path = target
                .path
                .canonicalize()
                .map_err(|err| format!("failed to canonicalize validation target: {err}"))?;
            return Ok(target);
        }
    }

    Err(format!(
        "validation target not found: {selector} (use an indexed video id, artifact id, inode recovery path, or direct file path)"
    ))
}

fn resolve_from_log(log_path: &Path, selector: &str) -> Option<ValidationTarget> {
    let text = read_to_string(log_path).ok()?;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let id = extract_json_string(line, "id");
        let inode = extract_json_string(line, "inode");
        let output_path = extract_json_string(line, "output_path");
        let output_artifact_path = extract_json_string(line, "output_artifact_path");
        let selector_field = extract_json_string(line, "selector");
        let derived_artifact_id = extract_json_string(line, "derived_artifact_id");
        let source_artifact_id = extract_json_string(line, "source_artifact_id");
        let source_artifact_path = extract_json_string(line, "source_artifact_path")
            .or_else(|| extract_json_string(line, "source_path"));
        let source_artifact_sha256 = extract_json_string(line, "source_artifact_sha256")
            .or_else(|| extract_json_string(line, "source_index_sha256"));
        let matches = id.as_deref() == Some(selector)
            || inode.as_deref() == Some(selector)
            || selector_field.as_deref() == Some(selector)
            || derived_artifact_id.as_deref() == Some(selector)
            || source_artifact_id.as_deref() == Some(selector)
            || output_path.as_deref() == Some(selector)
            || output_artifact_path.as_deref() == Some(selector);
        if matches {
            let target_artifact_id = derived_artifact_id
                .clone()
                .or_else(|| id.clone())
                .or_else(|| inode.clone())
                .or_else(|| selector_field.clone());
            return output_path
                .or(output_artifact_path)
                .map(|path| ValidationTarget {
                    path: PathBuf::from(path),
                    source_artifact_id,
                    source_artifact_path: source_artifact_path.map(PathBuf::from),
                    source_artifact_sha256,
                    derived_artifact_id,
                    target_artifact_id,
                });
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

fn append_validation_log(
    case_dir: &Path,
    result: &ValidationResult,
    options: &ValidationOptions,
) -> Result<(), String> {
    let operator = media_contract::resolve_operator(case_dir, options.operator.as_deref())?;
    let line = validation_log_body_json(result, options, &operator);
    let log_path = case_dir.join("evidence/logs/validation-log.jsonl");
    require_case_output_path(case_dir, &log_path, "validation log")?;
    audit::append_chained_jsonl(&log_path, &line)
}

fn validation_log_body_json(
    result: &ValidationResult,
    options: &ValidationOptions,
    operator: &str,
) -> String {
    let tool_version = command_version(&options.ffprobe_bin, &["ffprobe"], "-version");
    let command_args = ffprobe::probe_command_args(&result.target_path);
    let source_artifact_sha256 = result
        .source_artifact_sha256
        .as_deref()
        .unwrap_or(&result.target_sha256);
    let source_artifact_id = result.source_artifact_id.clone().unwrap_or_else(|| {
        media_contract::source_artifact_id(&result.selector, source_artifact_sha256)
    });
    let source_artifact_path = result
        .source_artifact_path
        .as_ref()
        .unwrap_or(&result.target_path);
    let target_artifact_id = result
        .target_artifact_id
        .as_deref()
        .or(result.derived_artifact_id.as_deref())
        .unwrap_or(&source_artifact_id);
    format!(
        "{{\"schema_version\":3,\"event\":\"validate-artifact\",\"validated_unix\":{},\"operator\":\"{}\",\"method\":\"ffprobe-container-video-stream\",\"tool\":\"ffprobe\",\"tool_version\":\"{}\",\"command\":\"{}\",\"command_args\":{},\"selector\":\"{}\",\"source_artifact_id\":\"{}\",\"source_artifact_path\":\"{}\",\"source_artifact_sha256\":\"{}\",\"derived_artifact_id\":{},\"target_artifact_id\":\"{}\",\"target_path\":\"{}\",\"target_sha256\":\"{}\",\"validation_artifact_path\":\"evidence/logs/validation-log.jsonl\",\"validation_status\":\"{}\",\"validation_note\":\"{}\",\"duration_seconds\":{},\"format_name\":{},\"video_codec\":{},\"audio_codec\":{},\"width\":{},\"height\":{},\"ffprobe_ok\":{},\"ffprobe_error\":{},\"ffprobe_version\":\"{}\"}}",
        result.validated_unix,
        json_escape(operator),
        json_escape(&tool_version),
        json_escape(&options.ffprobe_bin),
        audit::json_string_array(&command_args),
        json_escape(&result.selector),
        json_escape(&source_artifact_id),
        json_escape(&source_artifact_path.to_string_lossy()),
        json_escape(source_artifact_sha256),
        audit::optional_string(result.derived_artifact_id.as_deref()),
        json_escape(target_artifact_id),
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
        json_escape(&tool_version)
    )
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
    use super::{
        ValidationOptions, ValidationResult, extract_json_string, resolve_from_log,
        validation_log_body_json, validation_status,
    };
    use crate::model::ProbeSummary;
    use std::fs;
    use std::path::PathBuf;

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
                .path
                .to_string_lossy(),
            "/tmp/out.mp4"
        );
        fs::write(
            &log,
            r#"{"derived_artifact_id":"derived-frame-capture-bbbbbbbbbbbb","output_artifact_path":"/tmp/frame.jpg"}"#,
        )
        .unwrap();
        assert_eq!(
            resolve_from_log(&log, "derived-frame-capture-bbbbbbbbbbbb")
                .unwrap()
                .path
                .to_string_lossy(),
            "/tmp/frame.jpg"
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

    #[test]
    fn validation_log_body_records_forensic_promotion_contract() {
        let result = ValidationResult {
            selector: "carve_000001".to_string(),
            target_path: PathBuf::from("/case/artifacts/carved/carve_000001.mp4"),
            target_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
            source_artifact_id: None,
            source_artifact_path: None,
            source_artifact_sha256: None,
            derived_artifact_id: None,
            target_artifact_id: None,
            validation_status: "ffprobe-video-stream-confirmed".to_string(),
            validation_note: "ffprobe parsed a video stream".to_string(),
            probe: ProbeSummary {
                ok: true,
                raw_json: Some("{}".to_string()),
                error: None,
                duration_seconds: Some(12.0),
                format_name: Some("mov,mp4".to_string()),
                video_codec: Some("h264".to_string()),
                audio_codec: None,
                width: Some(1920),
                height: Some(1080),
            },
            validated_unix: 1_789_000_000,
        };
        let options = ValidationOptions {
            ffprobe_bin: "ffprobe".to_string(),
            operator: Some("qa-operator".to_string()),
        };

        let body = validation_log_body_json(&result, &options, "qa-operator");

        assert!(body.contains(r#""event":"validate-artifact""#));
        assert!(body.contains(r#""operator":"qa-operator""#));
        assert!(body.contains(r#""method":"ffprobe-container-video-stream""#));
        assert!(body.contains(r#""tool":"ffprobe""#));
        assert!(body.contains(r#""tool_version":"#));
        assert!(body.contains(
            r#""command_args":["-v","error","-print_format","json","-show_format","-show_streams""#
        ));
        assert!(body.contains(r#""source_artifact_id":"source-carve_000001-aaaaaaaaaaaa""#));
        assert!(body.contains(r#""source_artifact_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa""#));
        assert!(
            body.contains(r#""validation_artifact_path":"evidence/logs/validation-log.jsonl""#)
        );
    }

    #[test]
    fn validation_log_preserves_source_relation_for_derived_artifact_targets() {
        let result = ValidationResult {
            selector: "derived-frame-capture-bbbbbbbbbbbb".to_string(),
            target_path: PathBuf::from("/case/artifacts/frames/frame.jpg"),
            target_sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                .to_string(),
            source_artifact_id: Some("source-vid_000001-aaaaaaaaaaaa".to_string()),
            source_artifact_path: Some(PathBuf::from("/case/source/original.mp4")),
            source_artifact_sha256: Some(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            ),
            derived_artifact_id: Some("derived-frame-capture-bbbbbbbbbbbb".to_string()),
            target_artifact_id: Some("derived-frame-capture-bbbbbbbbbbbb".to_string()),
            validation_status: "ffprobe-video-stream-confirmed".to_string(),
            validation_note: "ffprobe parsed a video stream".to_string(),
            probe: ProbeSummary {
                ok: true,
                raw_json: Some("{}".to_string()),
                error: None,
                duration_seconds: Some(1.0),
                format_name: Some("image2".to_string()),
                video_codec: Some("mjpeg".to_string()),
                audio_codec: None,
                width: Some(640),
                height: Some(360),
            },
            validated_unix: 1_789_000_001,
        };
        let options = ValidationOptions {
            ffprobe_bin: "ffprobe".to_string(),
            operator: Some("qa-operator".to_string()),
        };

        let body = validation_log_body_json(&result, &options, "qa-operator");

        assert!(body.contains(r#""source_artifact_id":"source-vid_000001-aaaaaaaaaaaa""#));
        assert!(body.contains(r#""source_artifact_path":"/case/source/original.mp4""#));
        assert!(body.contains(
            r#""source_artifact_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa""#
        ));
        assert!(body.contains(r#""derived_artifact_id":"derived-frame-capture-bbbbbbbbbbbb""#));
        assert!(body.contains(r#""target_artifact_id":"derived-frame-capture-bbbbbbbbbbbb""#));
    }
}
