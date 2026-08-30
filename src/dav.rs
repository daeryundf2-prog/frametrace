//! Dahua DAV container support (first proprietary lane, per
//! docs/MANUFACTURER_PARSER_RESEARCH.md).
//!
//! The walker implements the container skeleton shared by the public OSS DAV
//! parsers: a 0x30-byte `DHAV` file header, per-frame `DHAV` records with a
//! stream-type byte (0xF1 video / 0xF3 audio), a little-endian payload length
//! at offset 0x20 of the frame header, and the `DC MD` end marker after each
//! payload. It has NOT yet been validated against real recorder exports —
//! treat outputs as candidate until examiner review (see ROADMAP M2-1).
//!
//! Extraction concatenates video frame payloads into an Annex-B elementary
//! stream, which ffmpeg can remux to MP4 without re-encoding.

use crate::audit;
use crate::util::run_with_timeout;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::Command;

const FILE_HEADER_SIZE: u64 = 0x30;
const FRAME_HEADER_SIZE: usize = 0x24;
const PAYLOAD_LENGTH_OFFSET: usize = 0x20;
const END_MARKER: [u8; 4] = [0xDC, 0x4D, 0x44, 0x00];
const STREAM_VIDEO: u8 = 0xF1;
const STREAM_AUDIO: u8 = 0xF3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DavFrame {
    pub offset: u64,
    pub stream_type: u8,
    pub channel: u16,
    pub payload_size: u64,
}

pub fn is_dav_header(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && &bytes[0..4] == b"DHAV"
}

/// Walks every frame record in the file (header + streaming walk, no full
/// buffering) and returns video/audio frames in file order. Errors on the
/// first structurally impossible record instead of guessing.
pub fn walk_frames(path: &Path) -> Result<Vec<DavFrame>, String> {
    let mut file =
        File::open(path).map_err(|err| format!("failed to open DAV {}: {err}", path.display()))?;
    let mut header = [0u8; 4];
    file.read_exact(&mut header)
        .map_err(|err| format!("failed to read DAV header: {err}"))?;
    if !is_dav_header(&header) {
        return Err(format!(
            "not a DAV container (missing DHAV magic): {}",
            path.display()
        ));
    }
    file.seek(SeekFrom::Start(FILE_HEADER_SIZE))
        .map_err(|err| format!("failed to seek past DAV header: {err}"))?;

    let mut frames = Vec::new();
    let mut frame_offset = FILE_HEADER_SIZE;
    loop {
        let mut frame_header = [0u8; FRAME_HEADER_SIZE];
        match file.read_exact(&mut frame_header) {
            Ok(()) => {}
            Err(ref err) if err.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(err) => return Err(format!("failed to read DAV frame header: {err}")),
        }
        if &frame_header[0..4] != b"DHAV" {
            return Err(format!(
                "DAV frame at offset {frame_offset} is missing the DHAV frame magic (unsupported variant; real-sample validation pending)"
            ));
        }
        let stream_type = frame_header[4];
        let channel = u16::from_le_bytes([frame_header[12], frame_header[13]]);
        let payload_size = u32::from_le_bytes([
            frame_header[PAYLOAD_LENGTH_OFFSET],
            frame_header[PAYLOAD_LENGTH_OFFSET + 1],
            frame_header[PAYLOAD_LENGTH_OFFSET + 2],
            frame_header[PAYLOAD_LENGTH_OFFSET + 3],
        ]) as u64;

        let mut end_marker = [0u8; 4];
        let skip = payload_size;
        file.seek(SeekFrom::Current(skip as i64))
            .map_err(|err| format!("failed to skip DAV payload: {err}"))?;
        file.read_exact(&mut end_marker)
            .map_err(|err| format!("failed to read DAV end marker: {err}"))?;
        if end_marker != END_MARKER {
            return Err(format!(
                "DAV frame at offset {frame_offset} has a corrupted end marker (found {:02x?})",
                end_marker
            ));
        }

        if stream_type == STREAM_VIDEO || stream_type == STREAM_AUDIO {
            frames.push(DavFrame {
                offset: frame_offset,
                stream_type,
                channel,
                payload_size,
            });
        }
        frame_offset = frame_offset + FRAME_HEADER_SIZE as u64 + payload_size + 4;
        // The recorder writes an end-of-file frame with a zero length; treat
        // the DHAV+0xFA trailer as EOF when nothing further parses.
        let at_eof = file.stream_position().unwrap_or(u64::MAX);
        let file_len = file
            .seek(SeekFrom::End(0))
            .map_err(|err| format!("failed to stat DAV size: {err}"))?;
        if at_eof >= file_len {
            break;
        }
        file.seek(SeekFrom::Start(at_eof))
            .map_err(|err| format!("failed to restore DAV position: {err}"))?;
    }
    Ok(frames)
}

/// Copies every video-frame payload into an Annex-B elementary stream.
/// Returns (es_path_bytes_written, video_frame_count, channel).
pub fn extract_video_es(
    dav_path: &Path,
    es_output: &Path,
) -> Result<(u64, usize, Option<u16>), String> {
    let frames = walk_frames(dav_path)?;
    let video: Vec<&DavFrame> = frames
        .iter()
        .filter(|frame| frame.stream_type == STREAM_VIDEO)
        .collect();
    if video.is_empty() {
        return Err("DAV contains no video frames".to_string());
    }
    let channel = video.first().map(|frame| frame.channel);

    if let Some(parent) = es_output.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create ES output directory: {err}"))?;
    }
    let mut input =
        BufReader::new(File::open(dav_path).map_err(|err| format!("failed to open DAV: {err}"))?);
    let mut output =
        File::create(es_output).map_err(|err| format!("failed to create ES output: {err}"))?;
    let mut written = 0u64;
    for frame in &video {
        input
            .seek(SeekFrom::Start(frame.offset + FRAME_HEADER_SIZE as u64))
            .map_err(|err| format!("failed to seek DAV frame: {err}"))?;
        let mut remaining = frame.payload_size;
        let mut chunk = [0u8; 64 * 1024];
        while remaining > 0 {
            let want = remaining.min(chunk.len() as u64) as usize;
            input
                .read_exact(&mut chunk[..want])
                .map_err(|err| format!("failed to read DAV payload: {err}"))?;
            output
                .write_all(&chunk[..want])
                .map_err(|err| format!("failed to write ES output: {err}"))?;
            written += want as u64;
            remaining -= want as u64;
        }
    }
    output
        .flush()
        .and_then(|_| output.sync_all())
        .map_err(|err| format!("failed to finalize ES output: {err}"))?;
    Ok((written, video.len(), channel))
}

/// Remuxes an Annex-B elementary stream into MP4 without re-encoding. Tries
/// h264 first, then hevc, mirroring the two codecs DAV recorders ship.
pub fn remux_es_to_mp4(
    es_path: &Path,
    mp4_output: &Path,
    timeout_secs: Option<u64>,
) -> Result<(), String> {
    let ffmpeg = crate::tool_policy::resolve_tool_binary("ffmpeg", &["ffmpeg"])
        .map_err(|err| format!("{err} (install FFmpeg and ensure ffmpeg is in PATH)"))?;
    if let Some(parent) = mp4_output.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create export directory: {err}"))?;
    }
    let mut last_error = String::new();
    for format in ["h264", "hevc"] {
        let mut command = Command::new(&ffmpeg);
        command
            .args(["-y", "-v", "error", "-f", format, "-i"])
            .arg(es_path)
            .args(["-c", "copy", "-movflags", "+faststart"])
            .arg(mp4_output);
        let output = run_with_timeout(&mut command, timeout_secs)?;
        if output.status.success() && mp4_output.exists() {
            return Ok(());
        }
        last_error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let _ = std::fs::remove_file(mp4_output);
    }
    Err(format!(
        "DAV elementary stream could not be remuxed as h264 or hevc: {last_error}"
    ))
}

/// SHA-256 of the exported MP4, for the audit log entry.
pub fn digest(path: &Path) -> Result<String, String> {
    audit::digest_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_bytes() -> Vec<u8> {
        // 0x30-byte file header
        let mut bytes = vec![0u8; FILE_HEADER_SIZE as usize];
        bytes[0..4].copy_from_slice(b"DHAV");
        bytes[4] = 0xF0;
        let mut frame = |stream_type: u8, channel: u16, payload: &[u8]| {
            let mut header = [0u8; FRAME_HEADER_SIZE];
            header[0..4].copy_from_slice(b"DHAV");
            header[4] = stream_type;
            header[12..14].copy_from_slice(&channel.to_le_bytes());
            header[PAYLOAD_LENGTH_OFFSET..PAYLOAD_LENGTH_OFFSET + 4]
                .copy_from_slice(&(payload.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&header);
            bytes.extend_from_slice(payload);
            bytes.extend_from_slice(&END_MARKER);
        };
        frame(STREAM_VIDEO, 1, b"VIDEO_PAYLOAD_ONE");
        frame(STREAM_AUDIO, 1, b"AUDIO");
        frame(STREAM_VIDEO, 1, b"VIDEO_PAYLOAD_TWO");
        bytes
    }

    fn fixture_path(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "{name}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::write(&path, fixture_bytes()).unwrap();
        path
    }

    #[test]
    fn rejects_files_without_dhav_magic() {
        let path = std::env::temp_dir().join(format!("ft-dav-bad-{}", std::process::id()));
        std::fs::write(&path, b"not a dav file at all").unwrap();
        let error = walk_frames(&path).unwrap_err();
        assert!(error.contains("not a DAV container"), "{error}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn walks_video_and_audio_frames_in_order() {
        let path = fixture_path("ft-dav-walk");
        let frames = walk_frames(&path).unwrap();
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0].stream_type, STREAM_VIDEO);
        assert_eq!(frames[0].payload_size, b"VIDEO_PAYLOAD_ONE".len() as u64);
        assert_eq!(frames[1].stream_type, STREAM_AUDIO);
        assert_eq!(frames[2].channel, 1);
        assert_eq!(frames[2].payload_size, b"VIDEO_PAYLOAD_TWO".len() as u64);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn extract_concatenates_only_video_payloads() {
        let path = fixture_path("ft-dav-extract");
        let es = std::env::temp_dir().join(format!("ft-dav-es-{}.bin", std::process::id()));
        let (written, frames, channel) = extract_video_es(&path, &es).unwrap();
        assert_eq!(
            written,
            (b"VIDEO_PAYLOAD_ONE".len() + b"VIDEO_PAYLOAD_TWO".len()) as u64
        );
        assert_eq!(frames, 2);
        assert_eq!(channel, Some(1));
        let content = std::fs::read(&es).unwrap();
        assert_eq!(content, b"VIDEO_PAYLOAD_ONEVIDEO_PAYLOAD_TWO".to_vec());
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&es);
    }

    #[test]
    fn corrupt_end_marker_is_reported_not_guessed() {
        let mut bytes = fixture_bytes();
        let last = bytes.len() - 4;
        bytes[last] = 0xFF;
        let path = std::env::temp_dir().join(format!("ft-dav-corrupt-{}", std::process::id()));
        std::fs::write(&path, &bytes).unwrap();
        let error = walk_frames(&path).unwrap_err();
        assert!(error.contains("corrupted end marker"), "{error}");
        let _ = std::fs::remove_file(&path);
    }
}
