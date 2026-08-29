use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_unix() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|err| format!("system time before UNIX epoch: {err}"))
}

pub fn create_case_layout(case_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(case_dir)?;
    for child in [
        "evidence",
        "evidence/hashes",
        "evidence/images",
        "evidence/logs",
        "artifacts",
        "artifacts/recovered",
        "artifacts/recovered/filesystem",
        "artifacts/carved",
        "artifacts/proxies",
        "artifacts/thumbnails",
        "artifacts/clips",
        "db",
        "db/filesystem",
        "review",
        "reports",
    ] {
        fs::create_dir_all(case_dir.join(child))?;
    }
    Ok(())
}

pub fn write_text(path: &Path, text: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
}

pub fn unique_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }

    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("artifact");
    let extension = path.extension().and_then(|extension| extension.to_str());

    for index in 1..10_000 {
        let filename = match extension {
            Some(extension) if !extension.is_empty() => {
                format!("{stem}_{index:03}.{extension}")
            }
            _ => format!("{stem}_{index:03}"),
        };
        let candidate = parent.join(filename);
        if !candidate.exists() {
            return candidate;
        }
    }

    path.to_path_buf()
}

pub fn read_to_string(path: &Path) -> io::Result<String> {
    fs::read_to_string(path)
}

/// Canonicalizes a path and strips the Windows extended-length prefix (`\\?\`)
/// so user-facing output and audit logs keep ordinary paths. `\\?\UNC\` maps
/// back to the leading `\\server\share` form.
pub fn canonicalize_display(path: &Path) -> io::Result<PathBuf> {
    Ok(strip_windows_extended_prefix(&path.canonicalize()?))
}

pub fn strip_windows_extended_prefix(path: &Path) -> PathBuf {
    let text = path.as_os_str().to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = text.strip_prefix(r"\\?\") {
        PathBuf::from(rest.to_string())
    } else {
        path.to_path_buf()
    }
}

pub fn json_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 8);
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            ch if ch < ' ' => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

pub fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn json_for_script(input: &str) -> String {
    input
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

pub fn path_to_file_url(path: &Path) -> String {
    // `std::fs::canonicalize` returns `\\?\`-prefixed paths on Windows; those
    // prefixes would survive into the URL and break it (`file:////?/C:/...`),
    // so strip them before encoding.
    let path =
        strip_windows_extended_prefix(&path.canonicalize().unwrap_or_else(|_| PathBuf::from(path)));
    let raw = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        format!("file:///{}", percent_encode_path(&raw))
    } else {
        format!("file://{}", percent_encode_path(&raw))
    }
}

pub fn compact_json_value_if_well_formed(input: &str) -> Option<String> {
    let trimmed = input.trim();
    let opener = trimmed.chars().next()?;
    let closer = match opener {
        '{' => '}',
        '[' => ']',
        _ => return None,
    };

    let mut stack = Vec::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut compact = String::new();

    for ch in trimmed.chars() {
        if in_string {
            compact.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                compact.push(ch);
            }
            '{' | '[' => {
                stack.push(if ch == '{' { '}' } else { ']' });
                compact.push(ch);
            }
            '}' | ']' => {
                if stack.pop()? != ch {
                    return None;
                }
                compact.push(ch);
            }
            ch if ch.is_whitespace() => {}
            _ => compact.push(ch),
        }
    }

    if in_string || escaped || !stack.is_empty() || !compact.ends_with(closer) {
        return None;
    }
    Some(compact)
}

fn percent_encode_path(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b':' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            byte => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        canonicalize_display, compact_json_value_if_well_formed, json_escape, path_to_file_url,
        strip_windows_extended_prefix, unique_path,
    };
    use std::fs;
    use std::path::{Path, PathBuf};

    #[test]
    fn escapes_json_control_characters() {
        assert_eq!(json_escape("a\"b\\c\n"), "a\\\"b\\\\c\\n");
    }

    #[test]
    fn strips_windows_extended_prefixes() {
        assert_eq!(
            strip_windows_extended_prefix(Path::new(r"\\?\C:\Cases\a.mp4")),
            PathBuf::from(r"C:\Cases\a.mp4")
        );
        assert_eq!(
            strip_windows_extended_prefix(Path::new(r"\\?\UNC\server\share\a.mp4")),
            PathBuf::from(r"\\server\share\a.mp4")
        );
        assert_eq!(
            strip_windows_extended_prefix(Path::new(r"C:\Cases\a.mp4")),
            PathBuf::from(r"C:\Cases\a.mp4")
        );
    }

    #[test]
    fn canonicalizes_to_display_paths() {
        let dir =
            std::env::temp_dir().join(format!("frametrace-canonical-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let canonical = canonicalize_display(&dir).unwrap();
        assert!(
            !canonical.as_os_str().to_string_lossy().starts_with(r"\\?\"),
            "unexpected extended prefix: {canonical:?}"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn creates_file_url() {
        let url = path_to_file_url(Path::new("/tmp/a b.mp4"));
        assert!(url.starts_with("file://"));
        assert!(url.contains("a%20b.mp4"));
    }

    #[cfg(windows)]
    #[test]
    fn file_url_never_contains_extended_prefix() {
        let url = path_to_file_url(Path::new(r"C:\Windows\System32\drivers\etc\hosts"));
        assert!(
            url.starts_with("file:///C:/Windows"),
            "unexpected url: {url}"
        );
        assert!(!url.contains("%3F"), "unexpected encoded prefix: {url}");
    }

    #[test]
    fn compacts_only_well_formed_json_values() {
        assert_eq!(
            compact_json_value_if_well_formed("{\n  \"a\": \"x y\"\n}").as_deref(),
            Some("{\"a\":\"x y\"}")
        );
        assert_eq!(compact_json_value_if_well_formed("not json"), None);
        assert_eq!(compact_json_value_if_well_formed("{\"a\":1"), None);
    }

    #[test]
    fn creates_unique_path_with_suffix() {
        let dir = std::env::temp_dir().join(format!(
            "frametrace-unique-path-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("clip.mp4");
        fs::write(&path, b"one").unwrap();
        assert_eq!(unique_path(&path), dir.join("clip_001.mp4"));
        fs::write(dir.join("clip_001.mp4"), b"two").unwrap();
        assert_eq!(unique_path(&path), dir.join("clip_002.mp4"));
        let _ = fs::remove_dir_all(dir);
    }
}
