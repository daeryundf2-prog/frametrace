use crate::sha256;
use crate::util::{json_escape, read_to_string, write_text};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditChainVerification {
    pub entries: usize,
    pub last_entry_sha256: String,
}

pub fn digest_file(path: &Path) -> Result<String, String> {
    let file = File::open(path)
        .map_err(|err| format!("failed to open {} for hashing: {err}", path.display()))?;
    sha256::digest_reader(BufReader::new(file))
        .map_err(|err| format!("failed to hash {}: {err}", path.display()))
}

pub fn append_chained_jsonl(path: &Path, body_json: &str) -> Result<(), String> {
    let existing = read_to_string(path).unwrap_or_default();
    let previous_entry_sha256 = existing
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map(|line| sha256::digest_bytes(line.as_bytes()))
        .unwrap_or_else(|| "GENESIS".to_string());
    let body = body_json.trim();
    if !body.starts_with('{') || !body.ends_with('}') {
        return Err("audit log body must be a JSON object".to_string());
    }

    let without_close = &body[..body.len() - 1];
    let chained = format!(
        "{without_close},\"previous_entry_sha256\":\"{}\"}}",
        json_escape(&previous_entry_sha256)
    );
    let entry_sha256 = sha256::digest_bytes(chained.as_bytes());
    let line = format!(
        "{},\"entry_sha256\":\"{}\"}}\n",
        &chained[..chained.len() - 1],
        json_escape(&entry_sha256)
    );
    write_text(path, &(existing + &line)).map_err(|err| {
        format!(
            "failed to append chained audit log {}: {err}",
            path.display()
        )
    })
}

pub fn verify_chained_jsonl(path: &Path) -> Result<AuditChainVerification, String> {
    let text = read_to_string(path)
        .map_err(|err| format!("failed to read audit log {}: {err}", path.display()))?;
    let mut previous_entry_sha256 = "GENESIS".to_string();
    let mut last_entry_sha256 = previous_entry_sha256.clone();
    let mut entries = 0usize;

    for (index, line) in text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
    {
        let line_number = index + 1;
        let recorded_previous = extract_json_string(line, "previous_entry_sha256")
            .ok_or_else(|| format!("audit line {line_number} is missing previous_entry_sha256"))?;
        if recorded_previous != previous_entry_sha256 {
            return Err(format!(
                "audit line {line_number} previous hash mismatch: expected {previous_entry_sha256}, found {recorded_previous}"
            ));
        }

        let recorded_entry = extract_json_string(line, "entry_sha256")
            .ok_or_else(|| format!("audit line {line_number} is missing entry_sha256"))?;
        let signed_entry = entry_without_recorded_hash(line).ok_or_else(|| {
            format!("audit line {line_number} has invalid entry_sha256 placement")
        })?;
        let computed_entry = sha256::digest_bytes(signed_entry.as_bytes());
        if recorded_entry != computed_entry {
            return Err(format!(
                "audit line {line_number} entry hash mismatch: expected {computed_entry}, found {recorded_entry}"
            ));
        }

        previous_entry_sha256 = sha256::digest_bytes(line.as_bytes());
        last_entry_sha256 = recorded_entry;
        entries += 1;
    }

    Ok(AuditChainVerification {
        entries,
        last_entry_sha256,
    })
}

pub fn indexed_source_hash(case_dir: &Path, selector: &str, source_path: &Path) -> Option<String> {
    let source = source_path.to_string_lossy();
    let text = read_to_string(&case_dir.join("db/videos.jsonl")).ok()?;
    for line in text.lines() {
        let id = extract_json_string(line, "id");
        let indexed_source = extract_json_string(line, "source_path");
        let relative_path = extract_json_string(line, "relative_path");
        let matches_selector = id.as_deref() == Some(selector)
            || indexed_source.as_deref() == Some(selector)
            || relative_path.as_deref() == Some(selector)
            || indexed_source.as_deref() == Some(source.as_ref());
        if matches_selector {
            return extract_json_string(line, "sha256");
        }
    }
    None
}

pub fn json_string_array(values: &[String]) -> String {
    let items = values
        .iter()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

pub fn optional_string(value: Option<&str>) -> String {
    value
        .map(|value| format!("\"{}\"", json_escape(value)))
        .unwrap_or_else(|| "null".to_string())
}

pub fn path_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub fn canonical_or_original(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
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

fn entry_without_recorded_hash(line: &str) -> Option<String> {
    let marker = ",\"entry_sha256\":";
    let start = line.rfind(marker)?;
    Some(format!("{}}}", &line[..start]))
}

#[cfg(test)]
mod tests {
    use super::{append_chained_jsonl, digest_file, verify_chained_jsonl};
    use crate::util::read_to_string;
    use std::fs;

    #[test]
    fn appends_chained_json_lines() {
        let dir =
            std::env::temp_dir().join(format!("frametrace-audit-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("audit.jsonl");

        append_chained_jsonl(&path, r#"{"kind":"one"}"#).unwrap();
        append_chained_jsonl(&path, r#"{"kind":"two"}"#).unwrap();
        let text = read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert!(text.contains("\"previous_entry_sha256\":\"GENESIS\""));
        assert!(text.contains("\"entry_sha256\""));

        let verification = verify_chained_jsonl(&path).unwrap();
        assert_eq!(verification.entries, 2);
        assert_ne!(verification.last_entry_sha256, "GENESIS");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_tampered_audit_lines() {
        let dir = std::env::temp_dir().join(format!(
            "frametrace-audit-tamper-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("audit.jsonl");

        append_chained_jsonl(&path, r#"{"kind":"one"}"#).unwrap();
        let text = read_to_string(&path).unwrap().replace("one", "two");
        fs::write(&path, text).unwrap();

        assert!(verify_chained_jsonl(&path).is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn hashes_files() {
        let path =
            std::env::temp_dir().join(format!("frametrace-audit-hash-test-{}", std::process::id()));
        fs::write(&path, b"abc").unwrap();
        assert_eq!(
            digest_file(&path).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let _ = fs::remove_file(path);
    }
}
