//! Hikvision export lane (IMKH-prefixed MPEG-PS style downloads).
//!
//! Public references (VLC `demux/mpeg/ps.c`, FFmpeg-user reports) document a
//! 40-byte proprietary header starting with ASCII `IMKH`. Remux strips that
//! header and asks FFmpeg to copy streams with `+genpts+discardcorrupt`.
//! Proprietary HDD filesystem recovery (`HIKVISION@HANGZHOU`) stays out of
//! scope until a real image corpus exists — see docs/HIKVISION_VALIDATION.md.

use crate::audit;
use crate::util::run_with_timeout;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::Command;

pub const IMKH_HEADER_SIZE: u64 = 40;
const IMKH_MAGIC: &[u8; 4] = b"IMKH";

pub fn is_imkh_header(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && &bytes[0..4] == IMKH_MAGIC
}

pub fn read_imkh_prefix(path: &Path) -> Result<[u8; 4], String> {
    let mut file = File::open(path)
        .map_err(|err| format!("failed to open Hikvision export {}: {err}", path.display()))?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)
        .map_err(|err| format!("failed to read Hikvision header: {err}"))?;
    Ok(magic)
}

/// Copies payload after the 40-byte IMKH header into `stripped_output`.
pub fn strip_imkh_header(source: &Path, stripped_output: &Path) -> Result<u64, String> {
    let mut input =
        File::open(source).map_err(|err| format!("failed to open Hikvision export: {err}"))?;
    let mut magic = [0u8; 4];
    input
        .read_exact(&mut magic)
        .map_err(|err| format!("failed to read Hikvision magic: {err}"))?;
    if !is_imkh_header(&magic) {
        return Err(format!(
            "not a Hikvision IMKH export (missing IMKH magic): {}",
            source.display()
        ));
    }
    input
        .seek(SeekFrom::Start(IMKH_HEADER_SIZE))
        .map_err(|err| format!("failed to seek past IMKH header: {err}"))?;
    if let Some(parent) = stripped_output.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create strip output directory: {err}"))?;
    }
    let mut output = File::create(stripped_output)
        .map_err(|err| format!("failed to create stripped output: {err}"))?;
    let written = std::io::copy(&mut input, &mut output)
        .map_err(|err| format!("failed to copy stripped payload: {err}"))?;
    output
        .flush()
        .and_then(|_| output.sync_all())
        .map_err(|err| format!("failed to finalize stripped output: {err}"))?;
    if written == 0 {
        return Err("Hikvision export has no payload after the IMKH header".to_string());
    }
    Ok(written)
}

/// Prefer stripping IMKH then remuxing; fall back to direct ffmpeg open.
pub fn remux_imkh_to_mp4(
    source: &Path,
    mp4_output: &Path,
    stripped_output: &Path,
    timeout_secs: Option<u64>,
) -> Result<&'static str, String> {
    let ffmpeg = crate::tool_policy::resolve_tool_binary("ffmpeg", &["ffmpeg"])
        .map_err(|err| format!("{err} (install FFmpeg and ensure ffmpeg is in PATH)"))?;
    if let Some(parent) = mp4_output.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create export directory: {err}"))?;
    }

    let written = strip_imkh_header(source, stripped_output)?;
    let mut command = Command::new(&ffmpeg);
    command
        .args([
            "-y",
            "-v",
            "error",
            "-fflags",
            "+genpts+discardcorrupt",
            "-i",
        ])
        .arg(stripped_output)
        .args(["-c", "copy", "-movflags", "+faststart"])
        .arg(mp4_output);
    let output = run_with_timeout(&mut command, timeout_secs)?;
    if output.status.success() && mp4_output.exists() {
        let _ = written;
        return Ok("imkh-strip-remux");
    }
    let strip_err = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let _ = std::fs::remove_file(mp4_output);

    let mut direct = Command::new(&ffmpeg);
    direct
        .args([
            "-y",
            "-v",
            "error",
            "-fflags",
            "+genpts+discardcorrupt",
            "-i",
        ])
        .arg(source)
        .args(["-c", "copy", "-movflags", "+faststart"])
        .arg(mp4_output);
    let direct_out = run_with_timeout(&mut direct, timeout_secs)?;
    if direct_out.status.success() && mp4_output.exists() {
        return Ok("ffmpeg-direct-imkh");
    }
    let direct_err = String::from_utf8_lossy(&direct_out.stderr)
        .trim()
        .to_string();
    let _ = std::fs::remove_file(mp4_output);
    Err(format!(
        "Hikvision IMKH remux failed after strip ({strip_err}); direct open also failed: {direct_err}"
    ))
}

pub fn digest(path: &Path) -> Result<String, String> {
    audit::digest_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_imkh() {
        let path = std::env::temp_dir().join(format!("ft-hik-bad-{}", std::process::id()));
        std::fs::write(&path, b"not hikvision").unwrap();
        let err = strip_imkh_header(&path, &path.with_extension("bin")).unwrap_err();
        assert!(err.contains("missing IMKH magic"), "{err}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn strips_forty_byte_header() {
        let mut bytes = vec![0u8; 40];
        bytes[0..4].copy_from_slice(b"IMKH");
        bytes.extend_from_slice(b"PAYLOAD_BYTES");
        let path = std::env::temp_dir().join(format!("ft-hik-ok-{}", std::process::id()));
        let out = std::env::temp_dir().join(format!("ft-hik-strip-{}", std::process::id()));
        std::fs::write(&path, &bytes).unwrap();
        let written = strip_imkh_header(&path, &out).unwrap();
        assert_eq!(written, b"PAYLOAD_BYTES".len() as u64);
        assert_eq!(std::fs::read(&out).unwrap(), b"PAYLOAD_BYTES");
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn rejects_header_only_export_without_payload() {
        let mut bytes = vec![0u8; 40];
        bytes[0..4].copy_from_slice(b"IMKH");
        let path = std::env::temp_dir().join(format!("ft-hik-empty-{}", std::process::id()));
        let out = std::env::temp_dir().join(format!("ft-hik-empty-out-{}", std::process::id()));
        std::fs::write(&path, &bytes).unwrap();
        let err = strip_imkh_header(&path, &out).unwrap_err();
        assert!(err.contains("no payload"), "{err}");
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn imkh_header_detection_requires_exact_magic() {
        assert!(is_imkh_header(b"IMKH_TRAILING"));
        assert!(!is_imkh_header(b"IMKX_TRAILING"));
        assert!(!is_imkh_header(b"IMK"));
        assert!(!is_imkh_header(b""));
    }
}
