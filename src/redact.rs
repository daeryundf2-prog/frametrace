//! Deterministic path redaction for distributable reports (security review
//! finding: reports and viewer payloads expose full source paths by default).
//!
//! `redact_json_text` rewrites every absolute path embedded in a JSON
//! document (or a JSONL stream, one object per line) into a stable token:
//!
//! - paths under the case directory become `<case>/`-relative, so report
//!   tables keep showing which artifact/output is meant;
//! - paths outside the case become `<redacted:HASH>` where HASH is the first
//!   16 hex characters of the SHA-256 over the normalized path string, so a
//!   redacted report still lets a reviewer tell "same file" from "different
//!   file" without revealing directory layout;
//! - `file://` URLs become `<redacted-url:HASH>` over the full URL string.
//!
//! Redaction is consistent by construction: the token is a pure function of
//! the input string, so the same path always maps to the same token across
//! all inputs of one report run and across runs.
//!
//! Strings that do not look like absolute paths (relative paths, ids, codec
//! names) pass through untouched — a relative path or bare filename is the
//! least-sensitive part of a recorded location and keeps tables readable.

use crate::util::strip_windows_extended_prefix;
use std::path::Path;

/// Redact every absolute path embedded in `input`. Whole-input JSON first;
/// if that does not parse, each non-empty line is treated as one JSONL
/// record. Unparseable input is returned unchanged (same failure-soft policy
/// as the report's `jsonl_to_array`, which skips torn lines downstream).
pub fn redact_json_text(input: &str, case_dir: &Path) -> String {
    if input.trim().is_empty() {
        return input.to_string();
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(input) {
        return redact_value(&value, case_dir).to_string();
    }
    let mut out = String::with_capacity(input.len());
    for (index, line) in input.lines().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(value) => out.push_str(&redact_value(&value, case_dir).to_string()),
            // A torn final line (documented crash-survivability mode) passes
            // through verbatim; downstream consumers already skip it.
            Err(_) => out.push_str(line),
        }
    }
    if input.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// The redaction note rendered into reports/review pages so the reader can
/// tell the paths were deliberately rewritten.
pub const REDACTION_NOTE: &str = "path redaction applied: absolute source paths were rewritten as <case>/… or <redacted:hash> tokens";

fn redact_value(value: &serde_json::Value, case_dir: &Path) -> serde_json::Value {
    match value {
        serde_json::Value::String(text) => serde_json::Value::String(redact_string(text, case_dir)),
        serde_json::Value::Array(items) => serde_json::Value::Array(
            items
                .iter()
                .map(|item| redact_value(item, case_dir))
                .collect(),
        ),
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.iter()
                .map(|(key, item)| (key.clone(), redact_value(item, case_dir)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn redact_string(text: &str, case_dir: &Path) -> String {
    if text.starts_with("file://") {
        return format!("<redacted-url:{}>", stable_token(text));
    }
    if !looks_like_absolute_path(text) {
        return text.to_string();
    }
    let normalized = strip_windows_extended_prefix(Path::new(text));
    let case_root = case_dir
        .canonicalize()
        .unwrap_or_else(|_| case_dir.to_path_buf());
    let normalized = normalized.canonicalize().unwrap_or(normalized);
    if let Ok(relative) = normalized.strip_prefix(&case_root) {
        let mut rendered = String::from("<case>");
        for component in relative.components() {
            rendered.push('/');
            rendered.push_str(&component.as_os_str().to_string_lossy());
        }
        return rendered;
    }
    format!("<redacted:{}>", stable_token(text))
}

/// First 16 hex chars of the SHA-256 over the raw input — deterministic, so
/// the same path always yields the same token, and short enough to keep
/// report tables readable.
fn stable_token(input: &str) -> String {
    crate::sha256::digest_bytes(input.as_bytes())[..16].to_string()
}

/// Absolute-path shape only: POSIX root, Windows drive (`C:\`/`C:/`), or UNC.
/// Relative paths like `camera/a.mp4` deliberately pass through.
fn looks_like_absolute_path(text: &str) -> bool {
    if text.starts_with('/') || text.starts_with("\\\\") {
        return true;
    }
    let bytes = text.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

#[cfg(test)]
mod tests {
    use super::{looks_like_absolute_path, redact_json_text};
    use std::fs;
    use std::path::PathBuf;

    fn temp_case(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("frametrace-redact-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn detects_absolute_path_shapes() {
        assert!(looks_like_absolute_path("/evidence/a.mp4"));
        assert!(looks_like_absolute_path(r"C:\evidence\a.mp4"));
        assert!(looks_like_absolute_path("C:/evidence/a.mp4"));
        assert!(looks_like_absolute_path(r"\\server\share\a.mp4"));
        assert!(!looks_like_absolute_path("camera/a.mp4"));
        assert!(!looks_like_absolute_path("vid_000001"));
        assert!(!looks_like_absolute_path("h264"));
    }

    #[test]
    fn paths_inside_the_case_become_case_relative() {
        let case_dir = temp_case("inside");
        let inside = case_dir.join("artifacts/clips/out.mp4");
        fs::create_dir_all(inside.parent().unwrap()).unwrap();
        fs::write(&inside, b"x").unwrap();
        let input = format!(
            "{{\"output_path\":\"{}\"}}",
            inside.to_string_lossy().replace('\\', "\\\\")
        );
        let out = redact_json_text(&input, &case_dir);
        let canonical_inside = inside.canonicalize().unwrap();
        let canonical_case = case_dir.canonicalize().unwrap();
        let relative = canonical_inside
            .strip_prefix(&canonical_case)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        assert!(
            out.contains(&format!("<case>/{relative}")),
            "unexpected: {out}"
        );
        assert!(!out.contains(&case_dir.to_string_lossy().to_string()));
        let _ = fs::remove_dir_all(case_dir);
    }

    #[test]
    fn outside_paths_become_deterministic_hash_tokens() {
        let case_dir = temp_case("outside");
        let input = r#"{"source_path":"/evidence/secret/clip.mp4","id":"vid_1"}"#;
        let first = redact_json_text(input, &case_dir);
        let second = redact_json_text(input, &case_dir);
        assert_eq!(first, second, "redaction must be deterministic");
        assert!(!first.contains("/evidence/secret"), "{first}");
        assert!(first.contains("<redacted:"), "{first}");
        // Relative values pass through.
        assert!(first.contains("\"vid_1\""));
        let _ = fs::remove_dir_all(case_dir);
    }

    #[test]
    fn same_path_across_documents_maps_to_the_same_token() {
        let case_dir = temp_case("consistent");
        let a = redact_json_text(r#"{"source_path":"/e/x.mp4"}"#, &case_dir);
        let b = redact_json_text("{\"target_path\":\"/e/x.mp4\"}\n", &case_dir);
        let token = |text: &str| {
            let start = text.find("<redacted:").unwrap();
            let end = text[start..].find('>').unwrap() + start;
            text[start..=end].to_string()
        };
        assert_eq!(token(&a), token(&b));
        let _ = fs::remove_dir_all(case_dir);
    }

    #[test]
    fn different_paths_get_different_tokens() {
        let case_dir = temp_case("distinct");
        let a = redact_json_text(r#"{"p":"/e/a.mp4"}"#, &case_dir);
        let b = redact_json_text(r#"{"p":"/e/b.mp4"}"#, &case_dir);
        assert_ne!(a, b);
        let _ = fs::remove_dir_all(case_dir);
    }

    #[test]
    fn file_urls_are_redacted_too() {
        let case_dir = temp_case("fileurl");
        let input = r#"{"file_url":"file:///evidence/secret/clip.mp4"}"#;
        let out = redact_json_text(input, &case_dir);
        assert!(!out.contains("clip.mp4"), "{out}");
        assert!(out.contains("<redacted-url:"), "{out}");
        let _ = fs::remove_dir_all(case_dir);
    }

    #[test]
    fn jsonl_lines_and_torn_tails_survive() {
        let case_dir = temp_case("jsonl");
        let input = "{\"output_path\":\"/e/a.mp4\"}\n{\"output_path\":\"/e/b.mp4\"}\n{\"torn";
        let out = redact_json_text(input, &case_dir);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(!lines[0].contains("/e/a.mp4"));
        assert!(!lines[1].contains("/e/b.mp4"));
        assert_eq!(lines[2], "{\"torn");
        let _ = fs::remove_dir_all(case_dir);
    }

    #[test]
    fn nested_and_array_paths_are_reached() {
        let case_dir = temp_case("nested");
        let input = r#"{"videos":[{"source_path":"/e/a.mp4","tags":["/e/nested.mp4"]}]}"#;
        let out = redact_json_text(input, &case_dir);
        assert!(!out.contains("/e/a.mp4"), "{out}");
        assert!(!out.contains("/e/nested.mp4"), "{out}");
        assert_eq!(out.matches("<redacted:").count(), 2);
        let _ = fs::remove_dir_all(case_dir);
    }

    #[test]
    fn relative_and_empty_inputs_pass_through() {
        let case_dir = temp_case("passthrough");
        assert_eq!(redact_json_text("", &case_dir), "");
        assert_eq!(
            redact_json_text(r#"{"relative_path":"cam/a.mp4"}"#, &case_dir),
            r#"{"relative_path":"cam/a.mp4"}"#
        );
        let _ = fs::remove_dir_all(case_dir);
    }
}
