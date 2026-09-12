//! Audit-chain HMAC key resolution (design: `docs/audit-hmac-design.md`).
//!
//! Keyed mode is opt-in: with no configured key, `configured()` returns
//! `Ok(None)` and audit appends write exactly the same unkeyed entries they
//! always did. Sources, in the design's order of preference — the first
//! available wins:
//!
//! 1. A `0600` key file (the design's OS-store fallback for platforms
//!    without a wired keystore). Default location is
//!    `$XDG_CONFIG_HOME/frametrace/audit-key` or `~/.config/frametrace/
//!    audit-key` on Unix and `%APPDATA%\frametrace\audit-key` on Windows;
//!    `FRAMETRACE_AUDIT_KEY_FILE` overrides the path (CI, portable
//!    installs). The file must be hex- or base64-encoded 32-byte key
//!    material and, on Unix, must have no group/other permission bits —
//!    a world-readable key defeats the control, so it is refused.
//! 2. `FRAMETRACE_AUDIT_KEY` — the same encodings, documented as the
//!    weakest source: any process running as the examiner can read it.
//!
//! `FRAMETRACE_AUDIT_KEY_ID` names whichever key was loaded (default
//! `"default"`) and is stamped as `entry_hmac_key_id`. The key never
//! lives inside the case directory — that would defeat the control
//! entirely.

use std::path::{Path, PathBuf};

/// A configured audit-chain key: key material plus the non-secret
/// identifier stamped into each keyed entry.
#[derive(Debug, Clone)]
pub struct AuditKey {
    pub id: String,
    pub bytes: Vec<u8>,
}

impl AuditKey {
    /// Builds a key for tests/tools that already hold raw material.
    pub fn new(id: &str, bytes: &[u8]) -> Self {
        Self {
            id: id.to_string(),
            bytes: bytes.to_vec(),
        }
    }
}

/// The key set verification uses: the entry's declared `key_id` is tried
/// first and every known key is tried as a fallback (a renamed id must
/// never brick old entries — see the design's rotation rules). Today the
/// set holds at most the one configured key; rotation support grows this
/// to configured + retired keys.
pub fn verification_keys() -> Result<Vec<AuditKey>, String> {
    Ok(configured()?.into_iter().collect())
}

/// Resolves the configured append key, if any. Absent sources are not an
/// error; a *present but unusable* source (bad permissions, malformed
/// material, wrong length) is, so a misconfigured key never silently
/// degrades an intended-keyed log to unkeyed.
pub fn configured() -> Result<Option<AuditKey>, String> {
    if let Some(key) = from_key_file()? {
        return Ok(Some(key));
    }
    match std::env::var("FRAMETRACE_AUDIT_KEY") {
        Ok(raw) if !raw.trim().is_empty() => {
            let bytes = parse_key_material(raw.trim())
                .map_err(|err| format!("FRAMETRACE_AUDIT_KEY: {err}"))?;
            Ok(Some(AuditKey {
                id: configured_key_id(),
                bytes: bytes.to_vec(),
            }))
        }
        Ok(_) => Err("FRAMETRACE_AUDIT_KEY is set but empty".to_string()),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err("FRAMETRACE_AUDIT_KEY is not valid unicode".to_string())
        }
    }
}

fn configured_key_id() -> String {
    std::env::var("FRAMETRACE_AUDIT_KEY_ID")
        .ok()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| "default".to_string())
}

fn from_key_file() -> Result<Option<AuditKey>, String> {
    let Some(path) = key_file_path() else {
        return Ok(None);
    };
    if !path.is_file() {
        return Ok(None);
    }
    check_key_file_permissions(&path)?;
    let text = std::fs::read_to_string(&path)
        .map_err(|err| format!("failed to read audit key file {}: {err}", path.display()))?;
    let bytes = parse_key_material(text.trim())
        .map_err(|err| format!("audit key file {}: {err}", path.display()))?;
    Ok(Some(AuditKey {
        id: configured_key_id(),
        bytes: bytes.to_vec(),
    }))
}

fn key_file_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("FRAMETRACE_AUDIT_KEY_FILE") {
        let path = path.trim();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    #[cfg(unix)]
    {
        if let Ok(base) = std::env::var("XDG_CONFIG_HOME")
            && !base.trim().is_empty()
        {
            return Some(PathBuf::from(base).join("frametrace/audit-key"));
        }
        std::env::var("HOME")
            .ok()
            .map(|home| PathBuf::from(home).join(".config/frametrace/audit-key"))
    }
    #[cfg(windows)]
    {
        std::env::var("APPDATA")
            .ok()
            .map(|base| PathBuf::from(base).join("frametrace").join("audit-key"))
    }
}

/// Unix: the key file must not be readable by group or other — a key an
/// attacker can copy is a key an attacker can sign with. Windows relies on
/// the profile ACL and cannot express mode bits, so there is nothing to
/// check there.
#[cfg(unix)]
fn check_key_file_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(path)
        .map_err(|err| format!("failed to stat audit key file {}: {err}", path.display()))?
        .permissions()
        .mode();
    if mode & 0o077 != 0 {
        return Err(format!(
            "audit key file {} has permissions {:o}; run `chmod 600` so only the owner can read it",
            path.display(),
            mode & 0o777
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_key_file_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

/// Decodes hex- or base64-encoded key material into exactly 32 bytes.
/// Hex is tried first (64 hex chars); anything else is tried as base64.
fn parse_key_material(text: &str) -> Result<Vec<u8>, String> {
    let decoded = if text.len() == 64 && text.chars().all(|c| c.is_ascii_hexdigit()) {
        hex_decode(text)
    } else {
        base64_decode(text)
    }
    .ok_or_else(|| "key is not valid hex (64 chars) or base64".to_string())?;
    if decoded.len() != 32 {
        return Err(format!(
            "key must decode to exactly 32 bytes, got {}",
            decoded.len()
        ));
    }
    Ok(decoded)
}

fn hex_decode(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len() / 2)
        .map(|index| u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).ok())
        .collect()
}

fn base64_decode(text: &str) -> Option<Vec<u8>> {
    fn value(byte: u8) -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    // Strip padding, then decode whole sextets. Any '=' must sit at the
    // end (enforced implicitly: '=' fails value() inside the body).
    let bytes = text.as_bytes();
    let body = if let Some(body) = bytes.strip_suffix(b"==") {
        body
    } else if let Some(body) = bytes.strip_suffix(b"=") {
        body
    } else {
        bytes
    };
    if body.is_empty() || body.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(body.len() * 3 / 4 + 3);
    let chunks: Vec<&[u8]> = body.chunks(4).collect();
    for chunk in chunks {
        let mut acc = 0u32;
        for (slot, byte) in chunk.iter().enumerate() {
            acc |= (value(*byte)? as u32) << (18 - 6 * slot);
        }
        let decoded = acc.to_be_bytes();
        let take = match chunk.len() {
            4 => 3,
            3 => 2,
            2 => 1,
            _ => return None,
        };
        out.extend_from_slice(&decoded[1..1 + take]);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::{base64_decode, hex_decode, parse_key_material};

    #[test]
    fn decodes_hex_key() {
        let hex = "00".repeat(32);
        assert_eq!(parse_key_material(&hex).unwrap(), vec![0u8; 32]);
        let hex = "ff".repeat(32);
        assert_eq!(parse_key_material(&hex).unwrap(), vec![0xffu8; 32]);
    }

    #[test]
    fn decodes_base64_key() {
        // 32 zero bytes base64-encode to 44 chars ending '='.
        let b64 = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
        assert_eq!(parse_key_material(b64).unwrap(), vec![0u8; 32]);
        // Known vector: "hello world" <-> aGVsbG8gd29ybGQ=
        assert_eq!(base64_decode("aGVsbG8gd29ybGQ=").unwrap(), b"hello world");
        assert_eq!(base64_decode("aGVsbG8gd29ybGQ").unwrap(), b"hello world");
        assert_eq!(base64_decode("aA==").unwrap(), b"h");
        assert_eq!(base64_decode("aGk=").unwrap(), b"hi");
    }

    #[test]
    fn rejects_wrong_length_and_garbage() {
        assert!(parse_key_material("00").is_err());
        assert!(parse_key_material(&"00".repeat(33)).is_err());
        assert!(parse_key_material("not a key at all!!!").is_err());
        assert!(base64_decode("a").is_none());
        assert!(hex_decode("xyz").is_none());
    }
}
