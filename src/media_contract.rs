use crate::util::read_to_string;
use std::env;
use std::path::Path;

pub fn resolve_operator(case_dir: &Path, explicit: Option<&str>) -> Result<String, String> {
    resolve_operator_value(explicit, case_manifest_operator(case_dir), env_operator())
}

fn resolve_operator_value(
    explicit: Option<&str>,
    case_operator: Option<String>,
    env_operator: Option<String>,
) -> Result<String, String> {
    explicit
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or(case_operator)
        .or(env_operator)
        .ok_or_else(|| {
            "operator is required; pass --operator or set case.json operator, USER, or USERNAME"
                .to_string()
        })
}

pub fn source_artifact_id(selector: &str, sha256: &str) -> String {
    artifact_id("source", selector, sha256)
}

pub fn derived_artifact_id(kind: &str, sha256: &str) -> String {
    artifact_id("derived", kind, sha256)
}

fn artifact_id(prefix: &str, label: &str, sha256: &str) -> String {
    let safe_label = sanitize_id_part(label);
    let hash_prefix = sha256.chars().take(12).collect::<String>();
    format!("{prefix}-{safe_label}-{hash_prefix}")
}

fn sanitize_id_part(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    for ch in label.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "artifact".to_string()
    } else {
        out
    }
}

fn case_manifest_operator(case_dir: &Path) -> Option<String> {
    let text = read_to_string(&case_dir.join("case.json")).ok()?;
    extract_json_string(&text, "operator")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_operator() -> Option<String> {
    env::var("USERNAME")
        .ok()
        .or_else(|| env::var("USER").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn extract_json_string(text: &str, key: &str) -> Option<String> {
    let key = format!("\"{}\":", key);
    let start = text.find(&key)? + key.len();
    let value = text[start..].trim_start();
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

#[cfg(test)]
mod tests {
    use super::{
        derived_artifact_id, resolve_operator, resolve_operator_value, source_artifact_id,
    };
    use std::fs;

    #[test]
    fn artifact_ids_are_stable_and_sanitized() {
        assert_eq!(
            source_artifact_id("carve/000001", "abcdef1234567890"),
            "source-carve_000001-abcdef123456"
        );
        assert_eq!(
            derived_artifact_id("proxy", "bbbbbbbbbbbbbbbb"),
            "derived-proxy-bbbbbbbbbbbb"
        );
    }

    #[test]
    fn operator_prefers_explicit_then_case_manifest() {
        let dir =
            std::env::temp_dir().join(format!("frametrace-operator-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("case.json"), r#"{"operator":"case-op"}"#).unwrap();
        assert_eq!(resolve_operator(&dir, Some("cli-op")).unwrap(), "cli-op");
        assert_eq!(resolve_operator(&dir, None).unwrap(), "case-op");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn media_audit_rejects_missing_operator() {
        let dir = std::env::temp_dir().join(format!(
            "frametrace-missing-operator-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let err = resolve_operator_value(Some(" "), None, None).unwrap_err();

        assert!(err.contains("operator is required"));
        let _ = fs::remove_dir_all(dir);
    }
}
