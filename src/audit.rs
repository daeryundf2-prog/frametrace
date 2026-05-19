use crate::sha256;
use crate::util::{json_escape, read_to_string, write_text};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn digest_file(path: &Path) -> Result<String, String> {
    let file = File::open(path)
        .map_err(|err| format!("failed to open {} for hashing: {err}", path.display()))?;
    sha256::digest_reader(BufReader::new(file))
        .map_err(|err| format!("failed to hash {}: {err}", path.display()))
}

pub fn command_version(binary: &str) -> String {
    match Command::new(binary).arg("-version").output() {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("unknown")
            .trim()
            .to_string(),
        Ok(output) => format!(
            "unavailable: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(err) => format!("unavailable: {err}"),
    }
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

#[cfg(test)]
mod tests {
    use super::{append_chained_jsonl, digest_file};
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
