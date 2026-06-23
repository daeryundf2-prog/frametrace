use crate::audit;
use crate::media_contract;
use crate::util::read_to_string;
use crate::video_export::resolve_video_source;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(super) struct ValidationTarget {
    pub(super) path: PathBuf,
    pub(super) source_artifact_id: Option<String>,
    pub(super) source_artifact_path: Option<PathBuf>,
    pub(super) source_artifact_sha256: Option<String>,
    pub(super) derived_artifact_id: Option<String>,
    pub(super) target_artifact_id: Option<String>,
}

pub(super) fn resolve_validation_target(
    case_dir: &Path,
    selector: &str,
) -> Result<ValidationTarget, String> {
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

pub(super) fn resolve_from_log(log_path: &Path, selector: &str) -> Option<ValidationTarget> {
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
    use super::{extract_json_string, resolve_from_log};
    use std::fs;

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
}
