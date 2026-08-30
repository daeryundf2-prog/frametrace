use crate::sha256;
use crate::util::{json_escape, read_to_string};
use fs2::FileExt;
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

/// Appends one hash-chained JSONL entry. The whole read-modify-append happens
/// under an OS-level exclusive lock so concurrent workers (and a second CLI
/// process) can never interleave entries, and the entry itself is written with
/// a single append + fsync so a crash can at worst tear the final line — which
/// `append` refuses to chain onto and `verify-audit` reports distinctly.
pub fn append_chained_jsonl(path: &Path, body_json: &str) -> Result<(), String> {
    let body = body_json.trim();
    if !body.starts_with('{') || !body.ends_with('}') {
        return Err("audit log body must be a JSON object".to_string());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create audit log directory: {err}"))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("failed to open audit log {}: {err}", path.display()))?;
    file.lock_exclusive().map_err(|err| {
        format!(
            "failed to lock audit log {} (another worker may be writing): {err}",
            path.display()
        )
    })?;
    let result = append_chained_locked(&mut file, body);
    let _ = file.unlock();
    result
}

fn append_chained_locked(file: &mut File, body: &str) -> Result<(), String> {
    use std::io::{Read, Seek, SeekFrom, Write};
    let mut existing = String::new();
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_to_string(&mut existing))
        .map_err(|err| format!("failed to read audit log for chaining: {err}"))?;
    if !existing.is_empty() && !existing.ends_with('\n') {
        return Err(
            "audit log ends with an incomplete line (likely a torn write); run verify-audit and repair the log before appending"
                .to_string(),
        );
    }
    let previous_entry_sha256 = existing
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map(|line| sha256::digest_bytes(line.as_bytes()))
        .unwrap_or_else(|| "GENESIS".to_string());

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
    // Append mode always writes at the end; one write_all + fsync keeps the
    // window for a torn write down to the final line only.
    file.write_all(line.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|err| format!("failed to append chained audit log entry: {err}"))
}

pub fn verify_chained_jsonl(path: &Path) -> Result<AuditChainVerification, String> {
    let text = read_to_string(path)
        .map_err(|err| format!("failed to read audit log {}: {err}", path.display()))?;
    let complete_tail = text.is_empty() || text.ends_with('\n');
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let mut previous_entry_sha256 = "GENESIS".to_string();
    let mut last_entry_sha256 = previous_entry_sha256.clone();
    let mut entries = 0usize;

    for (index, line) in lines.iter().enumerate() {
        let line_number = index + 1;
        let verify_line = || -> Result<(), String> {
            let recorded_previous =
                extract_json_string(line, "previous_entry_sha256").ok_or_else(|| {
                    format!("audit line {line_number} is missing previous_entry_sha256")
                })?;
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
            Ok(())
        };
        if let Err(error) = verify_line() {
            if line_number == lines.len() && !complete_tail {
                return Err(format!(
                    "audit log {} ends with an incomplete final entry (torn write): {error}; remove or repair the last line, then re-run verify",
                    path.display()
                ));
            }
            return Err(error);
        }

        previous_entry_sha256 = sha256::digest_bytes(line.as_bytes());
        last_entry_sha256 = recorded_entry_hash(line).unwrap_or_else(|| last_entry_sha256.clone());
        entries += 1;
    }

    Ok(AuditChainVerification {
        entries,
        last_entry_sha256,
    })
}

fn recorded_entry_hash(line: &str) -> Option<String> {
    extract_json_string(line, "entry_sha256")
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
    use std::io::Write;

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

    #[test]
    fn append_refuses_to_chain_onto_an_incomplete_tail() {
        let dir =
            std::env::temp_dir().join(format!("frametrace-audit-torn-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("audit.jsonl");
        append_chained_jsonl(&path, r#"{"kind":"one"}"#).unwrap();
        // Simulate a crash mid-append: partial JSON with no trailing newline.
        let mut handle = fs::OpenOptions::new().append(true).open(&path).unwrap();
        handle.write_all(b"{\"kind\":\"tor").unwrap();
        drop(handle);

        let append_error = append_chained_jsonl(&path, r#"{"kind":"two"}"#).unwrap_err();
        assert!(append_error.contains("incomplete line"), "{append_error}");
        let verify_error = verify_chained_jsonl(&path).unwrap_err();
        assert!(
            verify_error.contains("incomplete final entry"),
            "{verify_error}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn concurrent_appends_keep_the_chain_intact() {
        let dir =
            std::env::temp_dir().join(format!("frametrace-audit-race-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("audit.jsonl");
        std::thread::scope(|scope| {
            for worker in 0..4u32 {
                let path = path.clone();
                scope.spawn(move || {
                    for index in 0..15u32 {
                        append_chained_jsonl(
                            &path,
                            &format!("{{\"kind\":\"w{worker}\",\"i\":{index}}}"),
                        )
                        .unwrap();
                    }
                });
            }
        });
        let verification = verify_chained_jsonl(&path).unwrap();
        assert_eq!(verification.entries, 60);
        let _ = fs::remove_dir_all(&dir);
    }
}
