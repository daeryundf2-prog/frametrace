//! Dahua DAV / DHAV container support (first proprietary lane).
//!
//! Layout follows FFmpeg `libavformat/dhav.c` (the only public demuxer reference):
//! - optional `DAHUA` 0x400-byte file preamble, otherwise frames begin at offset 0
//! - per-frame `DHAV` header: type/subtype/channel/subnumber/frame#/length/date
//!   then (except type 0xF1) timestamp + ext_length + checksum + extension TLVs
//! - video types `0xFD` (key) / `0xFC` (non-key); audio `0xF0`; skip `0xF1`
//! - each frame ends with footer `dhav` + 4-byte back-pointer
//!
//! Remux prefers FFmpeg's native DHAV demuxer (`ffmpeg -i file.dav -c copy`).
//! Elementary-stream extraction remains for carved fragments and unit fixtures.
//! Real recorder exports still need examiner intake via
//! `scripts/validate-dav-samples.ps1` before claiming field validation.

use crate::audit;
use crate::util::run_with_timeout;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::Command;

const DAHUA_PREAMBLE: u64 = 0x400;
const MIN_FRAME_LEN: u32 = 24;
const FOOTER_MAGIC: [u8; 4] = *b"dhav";
const STREAM_AUDIO: u8 = 0xF0;
const STREAM_SKIP: u8 = 0xF1;
/// Non-key video frame type (public for the anomaly scanner).
pub const STREAM_VIDEO_P: u8 = 0xFC;
/// Key video frame type (public for the anomaly scanner).
pub const STREAM_VIDEO_I: u8 = 0xFD;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DavFrame {
    pub offset: u64,
    pub stream_type: u8,
    pub channel: u16,
    /// Packed Dahua date field (FFmpeg `get_date`): year-2000 << 26 |
    /// month << 22 | day << 17 | hour << 12 | minute << 6 | second.
    pub date: u32,
    /// Clock-time seconds within a minute, from the per-frame timestamp.
    pub timestamp_secs: u16,
    pub payload_offset: u64,
    pub payload_size: u64,
}

impl DavFrame {
    /// Absolute Unix-ish seconds derived from the packed date field. The
    /// Dahua date has no timezone; the raw packed value is kept for chain
    /// comparisons, and this converts it for human-readable details only.
    pub fn date_packed(&self) -> u32 {
        self.date
    }

    pub fn date_breakdown(&self) -> (u32, u32, u32, u32, u32, u32) {
        // Same unpacking order FFmpeg uses for the DHAV date field.
        let year = 2000 + (self.date >> 26);
        let month = (self.date >> 22) & 0xF;
        let day = (self.date >> 17) & 0x1F;
        let hour = (self.date >> 12) & 0x1F;
        let minute = (self.date >> 6) & 0x3F;
        let second = self.date & 0x3F;
        (year, month, day, hour, minute, second)
    }
}

pub fn is_dav_header(bytes: &[u8]) -> bool {
    if bytes.len() >= 5 && &bytes[0..5] == b"DAHUA" {
        return true;
    }
    if bytes.len() >= 5 && &bytes[0..4] == b"DHAV" {
        return matches!(
            bytes[4],
            STREAM_AUDIO | STREAM_SKIP | STREAM_VIDEO_P | STREAM_VIDEO_I
        );
    }
    false
}

fn read_u32_le(buf: &[u8]) -> u32 {
    u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]])
}

/// Locates the first DHAV frame (skipping an optional DAHUA preamble).
fn locate_stream_start(file: &mut File) -> Result<u64, String> {
    let mut head = [0u8; 5];
    file.seek(SeekFrom::Start(0))
        .map_err(|err| format!("failed to seek DAV start: {err}"))?;
    file.read_exact(&mut head)
        .map_err(|err| format!("failed to read DAV header: {err}"))?;
    if &head[0..5] == b"DAHUA" {
        file.seek(SeekFrom::Start(DAHUA_PREAMBLE))
            .map_err(|err| format!("failed to seek past DAHUA preamble: {err}"))?;
        return Ok(DAHUA_PREAMBLE);
    }
    if &head[0..4] == b"DHAV" {
        file.seek(SeekFrom::Start(0))
            .map_err(|err| format!("failed to rewind DAV: {err}"))?;
        return Ok(0);
    }
    Err("not a DAV container (missing DHAV/DAHUA magic)".to_string())
}

/// Walks every frame record (streaming, no full buffer) in FFmpeg order.
pub fn walk_frames(path: &Path) -> Result<Vec<DavFrame>, String> {
    let mut file =
        File::open(path).map_err(|err| format!("failed to open DAV {}: {err}", path.display()))?;
    let file_len = file
        .seek(SeekFrom::End(0))
        .map_err(|err| format!("failed to stat DAV size: {err}"))?;
    let mut pos = locate_stream_start(&mut file)?;
    let mut frames = Vec::new();

    while pos + 20 <= file_len {
        file.seek(SeekFrom::Start(pos))
            .map_err(|err| format!("failed to seek DAV frame at {pos}: {err}"))?;
        let mut prefix = [0u8; 20];
        match file.read_exact(&mut prefix) {
            Ok(()) => {}
            Err(ref err) if err.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(err) => return Err(format!("failed to read DAV frame header: {err}")),
        }
        if &prefix[0..4] != b"DHAV" {
            return Err(format!(
                "DAV frame at offset {pos} is missing the DHAV frame magic (unsupported variant; real-sample validation pending)"
            ));
        }
        let stream_type = prefix[4];
        let channel = prefix[6] as u16;
        let date = read_u32_le(&prefix[16..20]);
        let frame_length = read_u32_le(&prefix[12..16]);
        if frame_length < MIN_FRAME_LEN {
            return Err(format!(
                "DAV frame at offset {pos} has invalid length {frame_length}"
            ));
        }
        let frame_end = pos.saturating_add(frame_length as u64);
        if frame_end > file_len {
            return Err(format!(
                "DAV frame at offset {pos} extends past EOF (length {frame_length})"
            ));
        }

        if stream_type == STREAM_SKIP {
            pos = frame_end;
            continue;
        }

        let mut cursor = pos + 20;
        if cursor + 4 > file_len {
            break;
        }
        let mut meta = [0u8; 4];
        file.seek(SeekFrom::Start(cursor))
            .map_err(|err| format!("failed to seek DAV meta: {err}"))?;
        file.read_exact(&mut meta)
            .map_err(|err| format!("failed to read DAV meta: {err}"))?;
        let timestamp_secs = u16::from_le_bytes([meta[0], meta[1]]);
        let ext_length = meta[2] as u64;
        cursor += 4 + ext_length;

        let footer_start = frame_end.saturating_sub(8);
        if cursor > footer_start {
            return Err(format!(
                "DAV frame at offset {pos} has no room for payload/footer"
            ));
        }
        let payload_size = footer_start - cursor;

        file.seek(SeekFrom::Start(footer_start))
            .map_err(|err| format!("failed to seek DAV footer: {err}"))?;
        let mut footer = [0u8; 8];
        file.read_exact(&mut footer)
            .map_err(|err| format!("failed to read DAV footer: {err}"))?;
        if footer[0..4] != FOOTER_MAGIC {
            return Err(format!(
                "DAV frame at offset {pos} has a corrupted end marker (found {:02x?})",
                &footer[0..4]
            ));
        }

        if matches!(stream_type, STREAM_AUDIO | STREAM_VIDEO_P | STREAM_VIDEO_I) {
            frames.push(DavFrame {
                offset: pos,
                stream_type,
                channel,
                date,
                timestamp_secs,
                payload_offset: cursor,
                payload_size,
            });
        }
        pos = frame_end;
    }
    Ok(frames)
}

fn is_video_frame(stream_type: u8) -> bool {
    matches!(stream_type, STREAM_VIDEO_P | STREAM_VIDEO_I)
}

/// Copies every video-frame payload into an Annex-B elementary stream.
pub fn extract_video_es(
    dav_path: &Path,
    es_output: &Path,
) -> Result<(u64, usize, Option<u16>), String> {
    let frames = walk_frames(dav_path)?;
    let video: Vec<&DavFrame> = frames
        .iter()
        .filter(|frame| is_video_frame(frame.stream_type))
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
            .seek(SeekFrom::Start(frame.payload_offset))
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

/// Prefer FFmpeg's native DHAV demuxer; fall back to ES extract + remux.
pub fn remux_dav_to_mp4(
    dav_path: &Path,
    mp4_output: &Path,
    timeout_secs: Option<u64>,
) -> Result<&'static str, String> {
    let ffmpeg = crate::tool_policy::resolve_tool_binary("ffmpeg", &["ffmpeg"])
        .map_err(|err| format!("{err} (install FFmpeg and ensure ffmpeg is in PATH)"))?;
    if let Some(parent) = mp4_output.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create export directory: {err}"))?;
    }

    let mut command = Command::new(&ffmpeg);
    command
        .args(["-y", "-v", "error", "-i"])
        .arg(dav_path)
        .args(["-c", "copy", "-movflags", "+faststart"])
        .arg(mp4_output);
    let output = run_with_timeout(&mut command, timeout_secs)?;
    if output.status.success() && mp4_output.exists() {
        return Ok("ffmpeg-dhav-demux");
    }
    let native_err = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let _ = std::fs::remove_file(mp4_output);

    let es_path = mp4_output.with_extension("es.bin");
    let _ = extract_video_es(dav_path, &es_path)?;
    remux_es_to_mp4(&es_path, mp4_output, timeout_secs).map_err(|err| {
        format!("ffmpeg native demux failed ({native_err}); ES remux also failed: {err}")
    })?;
    let _ = std::fs::remove_file(&es_path);
    Ok("es-extract-remux")
}

/// Remuxes an Annex-B elementary stream into MP4 without re-encoding.
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

pub fn digest(path: &Path) -> Result<String, String> {
    audit::digest_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_frame(stream_type: u8, channel: u8, payload: &[u8]) -> Vec<u8> {
        let header_len = 24u32;
        let frame_length = header_len + payload.len() as u32 + 8;
        let mut bytes = Vec::with_capacity(frame_length as usize);
        bytes.extend_from_slice(b"DHAV");
        bytes.push(stream_type);
        bytes.push(0); // subtype
        bytes.push(channel);
        bytes.push(0); // frame_subnumber
        bytes.extend_from_slice(&1u32.to_le_bytes()); // frame_number
        bytes.extend_from_slice(&frame_length.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes()); // date
        bytes.extend_from_slice(&0u16.to_le_bytes()); // timestamp
        bytes.push(0); // ext_length
        bytes.push(0); // checksum
        bytes.extend_from_slice(payload);
        bytes.extend_from_slice(&FOOTER_MAGIC);
        bytes.extend_from_slice(&frame_length.to_le_bytes());
        bytes
    }

    fn fixture_bytes() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(build_frame(STREAM_VIDEO_I, 1, b"VIDEO_PAYLOAD_ONE"));
        bytes.extend(build_frame(STREAM_AUDIO, 1, b"AUDIO"));
        bytes.extend(build_frame(STREAM_VIDEO_P, 1, b"VIDEO_PAYLOAD_TWO"));
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
        assert_eq!(frames[0].stream_type, STREAM_VIDEO_I);
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
        let last_magic = bytes.len() - 8;
        bytes[last_magic] = 0xFF;
        let path = std::env::temp_dir().join(format!("ft-dav-corrupt-{}", std::process::id()));
        std::fs::write(&path, &bytes).unwrap();
        let error = walk_frames(&path).unwrap_err();
        assert!(error.contains("corrupted end marker"), "{error}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn accepts_dahua_preamble() {
        let mut bytes = vec![0u8; DAHUA_PREAMBLE as usize];
        bytes[0..5].copy_from_slice(b"DAHUA");
        bytes.extend(build_frame(STREAM_VIDEO_I, 2, b"PREAMBLE_VIDEO"));
        let path = std::env::temp_dir().join(format!("ft-dav-preamble-{}", std::process::id()));
        std::fs::write(&path, &bytes).unwrap();
        let frames = walk_frames(&path).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].channel, 2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn parses_packed_date_and_clock_timestamp() {
        // 2026-09-08 14:03:07 in the FFmpeg get_date() packing:
        // (year-2000)<<26 | month<<22 | day<<17 | hour<<12 | minute<<6 | second.
        let packed = (26u32 << 26) | (9 << 22) | (8 << 17) | (14 << 12) | (3 << 6) | 7;
        let mut bytes = Vec::new();
        bytes.extend(build_frame_with_date(
            STREAM_VIDEO_I,
            1,
            packed,
            42,
            b"DATED",
        ));
        let path = std::env::temp_dir().join(format!("ft-dav-date-{}", std::process::id()));
        std::fs::write(&path, &bytes).unwrap();
        let frames = walk_frames(&path).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].date_packed(), packed);
        assert_eq!(frames[0].timestamp_secs, 42);
        assert_eq!(frames[0].date_breakdown(), (2026, 9, 8, 14, 3, 7));
        let _ = std::fs::remove_file(&path);
    }

    fn build_frame_with_date(
        stream_type: u8,
        channel: u8,
        date: u32,
        timestamp: u16,
        payload: &[u8],
    ) -> Vec<u8> {
        let header_len = 24u32;
        let frame_length = header_len + payload.len() as u32 + 8;
        let mut bytes = Vec::with_capacity(frame_length as usize);
        bytes.extend_from_slice(b"DHAV");
        bytes.push(stream_type);
        bytes.push(0);
        bytes.push(channel);
        bytes.push(0);
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&frame_length.to_le_bytes());
        bytes.extend_from_slice(&date.to_le_bytes());
        bytes.extend_from_slice(&timestamp.to_le_bytes());
        bytes.push(0);
        bytes.push(0);
        bytes.extend_from_slice(payload);
        bytes.extend_from_slice(&FOOTER_MAGIC);
        bytes.extend_from_slice(&frame_length.to_le_bytes());
        bytes
    }
}
