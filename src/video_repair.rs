//! Repair of recovered videos a player cannot open (deleted-video triage,
//! phase 2). The output is a derived file, never the evidence: the source
//! keeps its bytes and hash, and the repaired copy, its method and inputs
//! are logged as a `repair-video` event in `evidence/logs/tsk-audit.jsonl`.
//!
//! * **AVI** — the RIFF/LIST/movi chunk chain is walked chunk by chunk; its
//!   first break (an overwritten or foreign cluster, a recording cut off
//!   mid-write) ends the usable data. The copy keeps the header list,
//!   re-emits the movi chunks up to the break, writes a fresh `idx1` and
//!   fixes the RIFF/movi sizes and the avih/strh frame counts.
//! * **MP4 without a usable moov** — the sample table is gone but the
//!   frames are still in mdat as length-prefixed NAL units. They are walked
//!   (resynchronising over audio or foreign runs), the codec configuration
//!   (SPS/PPS) comes from a healthy video of the same case, and ffmpeg
//!   remuxes the Annex-B stream at that video's frame rate. Video only:
//!   audio frames cannot be delimited without their table and are left out.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const OUTPUT_DIR: &str = "artifacts/Recovery/Repaired";
/// Largest source read into memory for a repair.
const MAX_SOURCE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// How far the NAL walk searches for the next frame after a foreign run.
const RESYNC_WINDOW: usize = 8 * 1024 * 1024;
/// References tried (best first) for a moov-less MP4.
const MAX_REFERENCES: usize = 3;

/// What one repair did — the body of its `repair-video` audit event.
#[derive(Debug, Clone, Serialize)]
pub struct RepairReport {
    pub event: &'static str,
    pub schema_version: u32,
    pub repaired_unix: u64,
    pub source_path: String,
    pub source_sha256: String,
    pub method: &'static str,
    pub output_path: String,
    pub sha256: String,
    pub size_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codec: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_sha256: Option<String>,
    /// Frames (AVI video chunks) or NAL units (MP4) kept.
    pub units_kept: u64,
    /// Source bytes left out (foreign runs skipped, data past the break).
    pub bytes_dropped: u64,
    pub resyncs: u64,
    /// Source offset where the usable data ended, when it ended early.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopped_at: Option<u64>,
    pub audio: &'static str,
    pub notes: Vec<String>,
    pub validation_status: String,
}

/// Repairs `source` (a video inside the case) into
/// `artifacts/Recovery/Repaired/`, logs it and validates the result.
/// Returns the `repair-video` entry (plus `reused`: an earlier repair of the
/// same bytes was handed back instead of a new copy).
pub fn repair_video(case_dir: &Path, source: &Path) -> Result<serde_json::Value, String> {
    let size = fs::metadata(source)
        .map_err(|err| format!("failed to read {}: {err}", source.display()))?
        .len();
    if size > MAX_SOURCE_BYTES {
        return Err(format!(
            "{} is larger than the {} GiB repair limit",
            source.display(),
            MAX_SOURCE_BYTES >> 30
        ));
    }
    let data =
        fs::read(source).map_err(|err| format!("failed to read {}: {err}", source.display()))?;
    let source_sha256 = crate::sha256::digest_bytes(&data);
    // The same bytes repaired before: hand back that copy, not another one.
    if let Some(mut previous) = previous_repair(case_dir, &source_sha256) {
        previous["reused"] = true.into();
        return Ok(previous);
    }
    let out_dir = case_dir.join(OUTPUT_DIR);
    fs::create_dir_all(&out_dir)
        .map_err(|err| format!("failed to create {}: {err}", out_dir.display()))?;
    let stem = source
        .file_stem()
        .map(|stem| crate::util::sanitize_path_component(&stem.to_string_lossy()))
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "video".to_string());

    let report = if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"AVI " {
        let target =
            crate::util::unique_available_path(&out_dir.join(format!("{stem}_repaired.avi")));
        let (bytes, stats) = repair_avi(&data)?;
        publish(&target, &bytes)?;
        RepairReport {
            method: "avi-reindex",
            output_path: target.to_string_lossy().to_string(),
            codec: stats.video_handler.clone(),
            units_kept: stats.video_frames,
            bytes_dropped: stats.bytes_dropped,
            stopped_at: stats.broken_at,
            audio: if stats.audio_chunks > 0 {
                "kept"
            } else {
                "none"
            },
            notes: stats.notes(),
            ..empty_report(source, &source_sha256)
        }
    } else {
        let target =
            crate::util::unique_available_path(&out_dir.join(format!("{stem}_repaired.mp4")));
        let (start, end) = mdat_range(&data);
        repair_mp4(
            case_dir,
            source,
            &data,
            (start, end),
            &source_sha256,
            &target,
            true,
        )?
    };
    finish(case_dir, report)
}

/// Hashes, validates and logs a published repair; the entry is returned.
fn finish(case_dir: &Path, mut report: RepairReport) -> Result<serde_json::Value, String> {
    let output = PathBuf::from(&report.output_path);
    let published =
        fs::read(&output).map_err(|err| format!("failed to read {}: {err}", output.display()))?;
    report.sha256 = crate::sha256::digest_bytes(&published);
    report.size_bytes = published.len() as u64;
    report.repaired_unix = crate::util::now_unix()?;
    report.validation_status = crate::validation::validate_artifact(
        case_dir,
        &report.output_path,
        &crate::validation::ValidationOptions::default(),
    )
    .map(|result| result.validation_status)
    .unwrap_or_else(|err| format!("validation-error: {err}"));
    crate::tsk::append_tsk_audit(
        case_dir,
        &serde_json::to_string(&report).map_err(|err| err.to_string())?,
    )?;
    let mut entry = serde_json::to_value(&report).map_err(|err| err.to_string())?;
    entry["reused"] = false.into();
    Ok(entry)
}

/// Rebuilds a headerless frame run read out of the image (`raw`: AVI movi
/// chunks or length-prefixed H.264/H.265 NAL units, see
/// [`crate::frame_carve`]) into a playable file at `target`, with the
/// header/codec configuration of a healthy video of the same card.
pub(crate) fn rebuild_frames(
    case_dir: &Path,
    raw: &Path,
    kind: crate::frame_carve::RunKind,
    target: &Path,
) -> Result<serde_json::Value, String> {
    let data = fs::read(raw).map_err(|err| format!("failed to read {}: {err}", raw.display()))?;
    let source_sha256 = crate::sha256::digest_bytes(&data);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let report = match kind {
        crate::frame_carve::RunKind::AviChunks => {
            let (reference, hdrl) = reference_avi_header(case_dir).ok_or(
                "같은 카드의 정상 AVI(참조 헤더)가 없어 헤더 없는 AVI 프레임을 재구성할 수 없습니다",
            )?;
            let mut avi = b"RIFF\0\0\0\0AVI ".to_vec();
            avi.extend_from_slice(&hdrl);
            if hdrl.len() % 2 == 1 {
                avi.push(0);
            }
            avi.extend_from_slice(b"LIST\0\0\0\0movi");
            avi.extend_from_slice(&data);
            let (bytes, stats) = repair_avi(&avi)?;
            publish(target, &bytes)?;
            let mut notes = stats.notes();
            notes.insert(
                0,
                format!(
                    "헤더 없이 남은 AVI 프레임 청크를 참조 영상 {}의 헤더로 다시 묶었습니다",
                    reference
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                ),
            );
            let reference_sha256 = fs::read(&reference)
                .ok()
                .map(|b| crate::sha256::digest_bytes(&b));
            RepairReport {
                method: "avi-frames-rebuild",
                output_path: target.to_string_lossy().to_string(),
                codec: stats.video_handler.clone(),
                reference_path: Some(reference.to_string_lossy().to_string()),
                reference_sha256,
                units_kept: stats.video_frames,
                bytes_dropped: stats.bytes_dropped,
                stopped_at: stats.broken_at,
                audio: if stats.audio_chunks > 0 {
                    "kept"
                } else {
                    "none"
                },
                notes,
                ..empty_report(raw, &source_sha256)
            }
        }
        crate::frame_carve::RunKind::AvccNals => {
            let mut report = repair_mp4(
                case_dir,
                raw,
                &data,
                (0, data.len()),
                &source_sha256,
                target,
                false,
            )?;
            report.method = "h264-frames-rebuild";
            report.notes.insert(
                0,
                "헤더(ftyp·moov) 없이 남은 H.264/H.265 프레임을 참조 영상의 코덱 설정으로 다시 묶었습니다".to_string(),
            );
            report
        }
    };
    finish(case_dir, report)
}

/// The header list (`LIST hdrl`) of the first healthy AVI of the case.
fn reference_avi_header(case_dir: &Path) -> Option<(PathBuf, Vec<u8>)> {
    let text = fs::read_to_string(case_dir.join("db/videos.jsonl")).ok()?;
    text.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|row| row.get("extension").and_then(|v| v.as_str()) == Some("avi"))
        .filter_map(|row| Some(PathBuf::from(row.get("source_path")?.as_str()?)))
        .take(12)
        .find_map(|path| {
            crate::lazy_extract::ensure_materialized(case_dir, &path).ok()?;
            let data = fs::read(&path).ok()?;
            if data.len() < 12 || &data[..4] != b"RIFF" || &data[8..12] != b"AVI " {
                return None;
            }
            let mut pos = 12;
            while pos + 12 <= data.len() {
                let size = le32(&data, pos + 4)? as usize;
                if &data[pos..pos + 4] == b"LIST" && &data[pos + 8..pos + 12] == b"hdrl" {
                    return data
                        .get(pos..pos + 8 + size)
                        .map(|hdrl| (path.clone(), hdrl.to_vec()));
                }
                pos += 8 + size + (size & 1);
            }
            None
        })
}

/// The newest usable `repair-video` entry for these source bytes whose copy
/// still exists (read through the lane's trust check like every reader).
fn previous_repair(case_dir: &Path, source_sha256: &str) -> Option<serde_json::Value> {
    let lane =
        crate::audit::read_lane_rows(&case_dir.join("evidence/logs/tsk-audit.jsonl")).ok()?;
    lane.usable()
        .rev()
        .map(|row| row.value.clone())
        .find(|entry| {
            entry.get("event").and_then(|v| v.as_str()) == Some("repair-video")
                && entry.get("source_sha256").and_then(|v| v.as_str()) == Some(source_sha256)
                && entry
                    .get("output_path")
                    .and_then(|v| v.as_str())
                    .is_some_and(|path| Path::new(path).is_file())
        })
}

fn empty_report(source: &Path, source_sha256: &str) -> RepairReport {
    RepairReport {
        event: "repair-video",
        schema_version: 1,
        repaired_unix: 0,
        source_path: source.to_string_lossy().to_string(),
        source_sha256: source_sha256.to_string(),
        method: "",
        output_path: String::new(),
        sha256: String::new(),
        size_bytes: 0,
        codec: None,
        fps: None,
        reference_path: None,
        reference_sha256: None,
        units_kept: 0,
        bytes_dropped: 0,
        resyncs: 0,
        stopped_at: None,
        audio: "none",
        notes: Vec::new(),
        validation_status: String::new(),
    }
}

/// Writes `bytes` to a pid-tagged part next to `target`, then publishes it
/// (never a torn file under the final name).
fn publish(target: &Path, bytes: &[u8]) -> Result<(), String> {
    let part = crate::join::pid_part_path(target, std::process::id());
    fs::write(&part, bytes).map_err(|err| format!("failed to write {}: {err}", part.display()))?;
    publish_part(&part, target)
}

fn publish_part(part: &Path, target: &Path) -> Result<(), String> {
    match crate::join::publish_part_as(part, target) {
        Ok(true) => Ok(()),
        Ok(false) => {
            let _ = fs::remove_file(part);
            Err(format!("output already exists: {}", target.display()))
        }
        Err(err) => {
            let _ = fs::remove_file(part);
            Err(err)
        }
    }
}

// ---------------------------------------------------------------- AVI ----

fn le32(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

fn put_le32(data: &mut [u8], at: usize, value: u32) {
    if let Some(slot) = data.get_mut(at..at + 4) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
}

fn printable(id: &[u8]) -> bool {
    id.len() == 4 && id.iter().all(|byte| (0x20..0x7f).contains(byte))
}

/// `00dc`/`01wb`… stream chunks, `ix00` OpenDML indexes, `JUNK`, `LIST`.
fn is_movi_id(id: &[u8]) -> bool {
    let stream = id[0].is_ascii_digit()
        && id[1].is_ascii_digit()
        && matches!(&id[2..4], b"dc" | b"db" | b"wb" | b"tx" | b"pc");
    let index = &id[..2] == b"ix" && id[2].is_ascii_digit() && id[3].is_ascii_digit();
    stream || index || id == b"JUNK" || id == b"LIST"
}

/// What an AVI repair kept.
#[derive(Debug, Default)]
struct AviStats {
    video_frames: u64,
    audio_chunks: u64,
    /// Video frames before the first keyframe, left out.
    leading_dropped: u64,
    bytes_dropped: u64,
    broken_at: Option<u64>,
    had_index: bool,
    riff_size_wrong: bool,
    video_handler: Option<String>,
}

impl AviStats {
    fn notes(&self) -> Vec<String> {
        let mut notes = vec![format!(
            "movi 청크 사슬을 따라 영상 {}프레임·오디오 {}청크를 다시 색인했습니다",
            self.video_frames, self.audio_chunks
        )];
        if self.leading_dropped > 0 {
            notes.push(leading_note(self.leading_dropped));
        }
        if let Some(at) = self.broken_at {
            notes.push(format!(
                "원본 {at} 바이트 위치에서 청크 사슬이 끊겨 그 뒤 {} 바이트는 제외했습니다 (덮어쓰기·녹화 중단 가능성)",
                self.bytes_dropped
            ));
        }
        if !self.had_index {
            notes.push("원본에 색인(idx1)이 없어 새로 만들었습니다".to_string());
        }
        if self.riff_size_wrong {
            notes.push("원본 RIFF 크기 값이 실제와 달라 고쳤습니다".to_string());
        }
        notes
    }
}

fn leading_note(frames: u64) -> String {
    format!("첫 키프레임 앞의 {frames}프레임은 참조 화면이 없어 디코딩할 수 없으므로 뺐습니다")
}

/// Where an AVI's intact chunk chain ends in `buf`, a prefix of the file
/// being read out: `Some(end)` once a chunk fails to parse (or, when
/// `complete`, where the parsable chunks stop); `None` while every chunk
/// so far is intact and the next one is not fully in `buf` yet. A cut-off
/// recording's RIFF/movi sizes are 0 or stale, so only the chain counts.
pub(crate) fn avi_data_end(buf: &[u8], complete: bool) -> Option<usize> {
    if buf.len() < 12 {
        return complete.then_some(buf.len());
    }
    if &buf[..4] != b"RIFF" || &buf[8..12] != b"AVI " {
        return Some(0);
    }
    let mut pos = 12usize;
    let mut movi_end: Option<usize> = None;
    loop {
        if movi_end.is_some_and(|end| pos >= end) {
            movi_end = None;
        }
        if pos + 8 > buf.len() {
            return complete.then_some(pos.min(buf.len()));
        }
        let id = &buf[pos..pos + 4];
        let size = le32(buf, pos + 4)? as usize;
        let inside = movi_end.is_some();
        if !(if inside {
            is_movi_id(id)
        } else {
            printable(id)
        }) {
            return Some(pos);
        }
        if id == b"LIST" {
            if pos + 12 > buf.len() {
                return complete.then_some(pos);
            }
            if &buf[pos + 8..pos + 12] == b"movi" {
                movi_end = Some(if size >= 4 {
                    pos + 8 + size
                } else {
                    usize::MAX
                });
                pos += 12;
                continue;
            }
            if inside {
                pos += 12;
                continue;
            }
        }
        let next = pos + 8 + size + (size & 1);
        if next > buf.len() {
            return complete.then_some(pos);
        }
        pos = next;
    }
}

/// Rebuilds an AVI from its header list and the intact part of its movi.
fn repair_avi(data: &[u8]) -> Result<(Vec<u8>, AviStats), String> {
    let mut stats = AviStats::default();
    let declared = le32(data, 4).map(|size| size as usize + 8).unwrap_or(0);
    stats.riff_size_wrong = declared != data.len();
    let mut hdrl: Option<(usize, usize)> = None;
    let mut movi: Option<(usize, usize)> = None;
    let mut pos = 12usize;
    while pos + 8 <= data.len() {
        let id = &data[pos..pos + 4];
        let Some(size) = le32(data, pos + 4).map(|size| size as usize) else {
            break;
        };
        if !printable(id) {
            break;
        }
        if id == b"LIST" && data.get(pos + 8..pos + 12) == Some(b"movi") {
            // A recording cut off mid-write leaves the movi size 0 or stale.
            let end = pos + 8 + size;
            let end = if size >= 4 && end <= data.len() {
                end
            } else {
                data.len()
            };
            movi = Some((pos + 12, end));
            pos = end + (size & 1);
            continue;
        }
        if pos + 8 + size > data.len() {
            break;
        }
        if id == b"LIST" && data.get(pos + 8..pos + 12) == Some(b"hdrl") {
            hdrl = Some((pos, 8 + size));
        } else if id == b"idx1" {
            stats.had_index = true;
        }
        pos += 8 + size + (size & 1);
    }
    let (hdrl_start, hdrl_len) =
        hdrl.ok_or("AVI header list (hdrl) not found — the header is overwritten")?;
    let (movi_start, movi_end) = movi.ok_or("AVI movi list not found")?;
    let video_handler = avi_video_handler(&data[hdrl_start..hdrl_start + hdrl_len]);
    stats.video_handler = video_handler.clone();

    // Walk the movi chain; the first chunk that does not parse ends it.
    let mut chunks: Vec<(usize, usize)> = Vec::new();
    let mut p = movi_start;
    while p + 8 <= movi_end {
        let id = &data[p..p + 4];
        let Some(size) = le32(data, p + 4).map(|size| size as usize) else {
            break;
        };
        if !is_movi_id(id) || p + 8 + size > data.len() {
            stats.broken_at = Some(p as u64);
            break;
        }
        if id == b"LIST" {
            // `rec ` groups: descend into them.
            p += 12;
            continue;
        }
        if id[0].is_ascii_digit() {
            chunks.push((p, size));
        }
        p += 8 + size + (size & 1);
    }
    if stats.broken_at.is_none() && p < movi_end {
        stats.broken_at = Some(p as u64);
    }
    if let Some(at) = stats.broken_at {
        stats.bytes_dropped = (movi_end as u64).saturating_sub(at);
    }
    if chunks.is_empty() {
        return Err("no intact movi chunk — nothing to re-index".to_string());
    }
    // A run that starts mid-GOP: the frames before its first keyframe
    // reference pictures that are gone. Unknown codecs keep everything.
    let is_video = |at: usize| matches!(&data[at + 2..at + 4], b"dc" | b"db");
    let hevc = hevc_handler(stats.video_handler.as_deref());
    if let Some(first_key) = chunks
        .iter()
        .position(|&(at, size)| is_video(at) && avi_keyframe(&data[at + 8..at + 8 + size], hevc))
    {
        stats.leading_dropped = chunks[..first_key]
            .iter()
            .filter(|&&(at, _)| is_video(at))
            .count() as u64;
        chunks.drain(..first_key);
    }

    let mut out = Vec::with_capacity(data.len() + chunks.len() * 16 + 64);
    out.extend_from_slice(b"RIFF\0\0\0\0AVI ");
    let hdrl_at = out.len();
    out.extend_from_slice(&data[hdrl_start..hdrl_start + hdrl_len]);
    if hdrl_len & 1 == 1 {
        out.push(0);
    }
    let movi_list = out.len();
    out.extend_from_slice(b"LIST\0\0\0\0movi");
    let movi_fourcc = movi_list + 8;
    let mut index = Vec::with_capacity(chunks.len() * 16);
    let mut first_video = true;
    for &(at, size) in &chunks {
        let id = &data[at..at + 4];
        let body = &data[at + 8..at + 8 + size];
        let key = if is_video(at) {
            let key = first_video || avi_keyframe(body, hevc);
            first_video = false;
            stats.video_frames += 1;
            key
        } else {
            if &id[2..4] == b"wb" {
                stats.audio_chunks += 1;
            }
            true
        };
        let offset = (out.len() - movi_fourcc) as u32;
        out.extend_from_slice(&data[at..at + 8 + size]);
        if size & 1 == 1 {
            out.push(0);
        }
        index.extend_from_slice(id);
        index.extend_from_slice(&(if key { 0x10u32 } else { 0 }).to_le_bytes());
        index.extend_from_slice(&offset.to_le_bytes());
        index.extend_from_slice(&(size as u32).to_le_bytes());
    }
    let movi_size = (out.len() - movi_list - 8) as u32;
    put_le32(&mut out, movi_list + 4, movi_size);
    out.extend_from_slice(b"idx1");
    out.extend_from_slice(&(index.len() as u32).to_le_bytes());
    out.extend_from_slice(&index);
    let riff_size = (out.len() - 8) as u32;
    put_le32(&mut out, 4, riff_size);
    patch_avi_header(
        &mut out[hdrl_at..hdrl_at + hdrl_len],
        stats.video_frames as u32,
    );
    Ok((out, stats))
}

/// Every `(id, data start, size)` chunk inside `list` (LISTs descended).
fn avi_header_chunks(list: &[u8]) -> Vec<([u8; 4], usize, usize)> {
    let mut found = Vec::new();
    let mut stack = vec![(12usize, list.len())];
    while let Some((mut p, end)) = stack.pop() {
        while p + 8 <= end {
            let Ok(id) = <[u8; 4]>::try_from(&list[p..p + 4]) else {
                break;
            };
            let Some(size) = le32(list, p + 4).map(|size| size as usize) else {
                break;
            };
            if !printable(&id) || p + 8 + size > end {
                break;
            }
            if &id == b"LIST" {
                stack.push((p + 12, p + 8 + size));
            } else {
                found.push((id, p + 8, size));
            }
            p += 8 + size + (size & 1);
        }
    }
    found
}

fn avi_video_handler(hdrl: &[u8]) -> Option<String> {
    avi_header_chunks(hdrl)
        .into_iter()
        .find(|(id, at, size)| id == b"strh" && *size >= 8 && &hdrl[*at..*at + 4] == b"vids")
        .map(|(_, at, _)| {
            String::from_utf8_lossy(&hdrl[at + 4..at + 8])
                .trim()
                .to_string()
        })
}

/// avih: total frames + AVIF_HASINDEX; the video strh: its length.
fn patch_avi_header(hdrl: &mut [u8], video_frames: u32) {
    for (id, at, size) in avi_header_chunks(hdrl) {
        if &id == b"avih" && size >= 20 {
            let flags = le32(hdrl, at + 12).unwrap_or(0) | 0x10;
            put_le32(hdrl, at + 12, flags);
            put_le32(hdrl, at + 16, video_frames);
        } else if &id == b"strh" && size >= 36 && &hdrl[at..at + 4] == b"vids" {
            put_le32(hdrl, at + 32, video_frames);
        }
    }
}

/// Whether a video chunk starts a decodable picture: every MJPEG frame; an
/// H.264/H.265 chunk carrying an IDR / parameter-set NAL. The two codecs'
/// NAL types overlap (an H.264 P slice 0x41 reads as an H.265 VPS), so the
/// stream's handler decides which one applies.
fn avi_keyframe(body: &[u8], hevc: bool) -> bool {
    if body.starts_with(&[0xFF, 0xD8]) {
        return true;
    }
    let scan = &body[..body.len().min(256)];
    scan.windows(4).any(|w| {
        w[..3] == [0, 0, 1] && {
            if hevc {
                let kind = (w[3] >> 1) & 0x3F;
                (16..=21).contains(&kind) || kind == 32 || kind == 33
            } else {
                matches!(w[3] & 0x1F, 5 | 7)
            }
        }
    })
}

fn hevc_handler(handler: Option<&str>) -> bool {
    handler.is_some_and(|h| {
        matches!(
            h.to_ascii_uppercase().as_str(),
            "HEVC" | "H265" | "HVC1" | "HEV1" | "X265"
        )
    })
}

// ---------------------------------------------------------------- MP4 ----

fn be32(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

fn be64(data: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_be_bytes(data.get(at..at + 8)?.try_into().ok()?))
}

/// `(type, payload start, end)` of the boxes in `[start, end)`.
fn boxes(data: &[u8], start: usize, end: usize) -> Vec<([u8; 4], usize, usize)> {
    let mut out = Vec::new();
    let mut p = start;
    while p + 8 <= end {
        let Some(size32) = be32(data, p) else { break };
        let Ok(kind) = <[u8; 4]>::try_from(&data[p + 4..p + 8]) else {
            break;
        };
        if !printable(&kind) {
            break;
        }
        let (size, header) = match size32 {
            1 => match be64(data, p + 8) {
                Some(size) => (size as usize, 16),
                None => break,
            },
            0 => (end - p, 8),
            size => (size as usize, 8),
        };
        if size < header {
            break;
        }
        let box_end = p.saturating_add(size).min(end);
        out.push((kind, p + header, box_end));
        if p + size > end {
            break;
        }
        p += size;
    }
    out
}

fn child(data: &[u8], parent: (usize, usize), kind: &[u8; 4]) -> Option<(usize, usize)> {
    boxes(data, parent.0, parent.1)
        .into_iter()
        .find(|(k, _, _)| k == kind)
        .map(|(_, start, end)| (start, end))
}

/// The video codec configuration of a moov.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VideoConfig {
    hevc: bool,
    length_size: usize,
    /// Parameter-set NAL units (SPS/PPS, VPS) in decoding order.
    parameter_sets: Vec<Vec<u8>>,
    fps: f64,
}

/// The codec configuration of the first healthy MP4 of the case (read out
/// of the image if needed) — what headerless H.264/H.265 frames found on
/// the card are checked against and rebuilt with.
pub(crate) fn reference_config(case_dir: &Path) -> Option<(VideoConfig, PathBuf)> {
    reference_candidates(case_dir, Path::new(""))
        .into_iter()
        .find_map(|path| {
            crate::lazy_extract::ensure_materialized(case_dir, &path).ok()?;
            let config = video_config(&fs::read(&path).ok()?)?;
            Some((config, path))
        })
}

/// An H.264 configuration for tests: 4-byte lengths, PPS id 0.
#[cfg(test)]
pub(crate) fn test_config() -> VideoConfig {
    VideoConfig {
        hevc: false,
        length_size: 4,
        parameter_sets: vec![vec![0x67, 0x64, 0, 0x28], vec![0x68, 0x80]],
        fps: 30.0,
    }
}

/// Checks one length-prefixed NAL unit from the first bytes at a stream
/// position (no data beyond them needed) — the frame-run scanner's view of
/// [`NalStream`].
const MAX_NON_SLICE_NAL: usize = 64 * 1024;

pub(crate) struct AvccProbe {
    n: usize,
    hevc: bool,
    pps: Vec<u32>,
    frame_num: Option<FrameNumCoding>,
}

impl AvccProbe {
    /// Bytes a probe needs from the unit's first byte.
    pub(crate) const PEEK: usize = 48;

    pub(crate) fn new(config: &VideoConfig) -> Self {
        Self {
            n: config.length_size,
            hevc: config.hevc,
            pps: pps_ids(config),
            frame_num: config_frame_num(config),
        }
    }

    /// `frame_num` of the slice unit at `head` (H.264 only).
    pub(crate) fn order(&self, head: &[u8]) -> Option<u32> {
        let nal = head.get(self.n..)?;
        if self.hevc || !is_slice(false, *nal.first()?) {
            return None;
        }
        slice_frame_num(self.frame_num?, nal)
    }

    /// Whether the slice unit at `head` can follow a picture numbered
    /// `last` across a gap (see [`frame_num_continues`]).
    pub(crate) fn continues(&self, last: Option<u32>, head: &[u8]) -> bool {
        match head.get(self.n..) {
            Some(nal) if !self.hevc && nal.first().is_some_and(|h| is_slice(false, *h)) => {
                frame_num_continues(self.frame_num, last, nal)
            }
            _ => true,
        }
    }

    /// `(unit length incl. prefix, is a slice)` when `head` starts a
    /// plausible NAL unit; with `first` only a picture's first slice (or a
    /// parameter-set/SEI unit) qualifies.
    pub(crate) fn unit(&self, head: &[u8], first: bool) -> Option<(u64, bool)> {
        let n = self.n;
        if head.len() < n + 2 {
            return None;
        }
        let len = head[..n]
            .iter()
            .fold(0usize, |acc, byte| (acc << 8) | *byte as usize);
        if !(2..=16 * 1024 * 1024).contains(&len) || !nal_header_ok(self.hevc, &head[n..n + 2]) {
            return None;
        }
        let header = head[n];
        if !is_slice(self.hevc, header) {
            // Parameter sets, SEI, AUD are small; a long one is a false
            // match whose length would swallow real frames.
            return (len <= MAX_NON_SLICE_NAL).then_some(((n + len) as u64, false));
        }
        let header_len = if self.hevc { 2 } else { 1 };
        let payload = head.get(n + header_len..)?;
        let mut bits = Bits::new(payload);
        let (is_first, pps) = if self.hevc {
            let flag = bits.bit()? == 1;
            if (16..=23).contains(&((header >> 1) & 0x3F)) {
                bits.bit()?;
            }
            (flag, bits.ue()?)
        } else {
            let first_mb = bits.ue()?;
            if !h264_slice_type_ok(header, bits.ue()?) {
                return None;
            }
            (first_mb == 0, bits.ue()?)
        };
        let known = self.pps.is_empty() || self.pps.contains(&pps);
        (known && (!first || is_first)).then_some(((n + len) as u64, true))
    }
}

/// The first video track's configuration from the moov in `data`.
fn video_config(data: &[u8]) -> Option<VideoConfig> {
    let moov = boxes(data, 0, data.len())
        .into_iter()
        .find(|(k, _, _)| k == b"moov")
        .map(|(_, s, e)| (s, e))?;
    for (kind, start, end) in boxes(data, moov.0, moov.1) {
        if &kind != b"trak" {
            continue;
        }
        let Some(mdia) = child(data, (start, end), b"mdia") else {
            continue;
        };
        let Some(hdlr) = child(data, mdia, b"hdlr") else {
            continue;
        };
        if data.get(hdlr.0 + 8..hdlr.0 + 12) != Some(b"vide") {
            continue;
        }
        let timescale = child(data, mdia, b"mdhd").and_then(|mdhd| {
            if data.get(mdhd.0) == Some(&1) {
                be32(data, mdhd.0 + 20)
            } else {
                be32(data, mdhd.0 + 12)
            }
        });
        let stbl = child(data, mdia, b"minf").and_then(|minf| child(data, minf, b"stbl"))?;
        let delta = child(data, stbl, b"stts").and_then(|stts| be32(data, stts.0 + 12));
        let fps = match (timescale, delta) {
            (Some(scale), Some(delta)) if scale > 0 && delta > 0 => scale as f64 / delta as f64,
            _ => 30.0,
        };
        let stsd = child(data, stbl, b"stsd")?;
        // stsd: version/flags + entry count, then the first sample entry;
        // its child boxes follow the 8 + 78 byte visual sample entry header.
        let (entry_kind, entry_start, entry_end) =
            boxes(data, stsd.0 + 8, stsd.1).into_iter().next()?;
        let hevc = matches!(&entry_kind, b"hvc1" | b"hev1");
        if !hevc && !matches!(&entry_kind, b"avc1" | b"avc3") {
            return None;
        }
        let config_kind: &[u8; 4] = if hevc { b"hvcC" } else { b"avcC" };
        let config = boxes(data, entry_start + 78, entry_end)
            .into_iter()
            .find(|(k, _, _)| k == config_kind)
            .map(|(_, s, e)| &data[s..e])?;
        let (length_size, parameter_sets) = if hevc {
            parse_hvcc(config)?
        } else {
            parse_avcc(config)?
        };
        return Some(VideoConfig {
            hevc,
            length_size,
            parameter_sets,
            fps,
        });
    }
    None
}

fn parse_avcc(config: &[u8]) -> Option<(usize, Vec<Vec<u8>>)> {
    let length_size = (config.get(4)? & 3) as usize + 1;
    let mut sets = Vec::new();
    let mut p = 5;
    for group in 0..2 {
        let count = if group == 0 {
            (config.get(p)? & 0x1F) as usize
        } else {
            *config.get(p)? as usize
        };
        p += 1;
        for _ in 0..count {
            let len = u16::from_be_bytes(config.get(p..p + 2)?.try_into().ok()?) as usize;
            sets.push(config.get(p + 2..p + 2 + len)?.to_vec());
            p += 2 + len;
        }
    }
    Some((length_size, sets))
}

fn parse_hvcc(config: &[u8]) -> Option<(usize, Vec<Vec<u8>>)> {
    let length_size = (config.get(21)? & 3) as usize + 1;
    let arrays = *config.get(22)? as usize;
    let mut sets = Vec::new();
    let mut p = 23;
    for _ in 0..arrays {
        let count = u16::from_be_bytes(config.get(p + 1..p + 3)?.try_into().ok()?) as usize;
        p += 3;
        for _ in 0..count {
            let len = u16::from_be_bytes(config.get(p..p + 2)?.try_into().ok()?) as usize;
            sets.push(config.get(p + 2..p + 2 + len)?.to_vec());
            p += 2 + len;
        }
    }
    Some((length_size, sets))
}

/// The mdat payload range (the whole file after its leading boxes when
/// no mdat header survives).
fn mdat_range(data: &[u8]) -> (usize, usize) {
    let top = boxes(data, 0, data.len());
    if let Some((_, start, end)) = top.iter().find(|(k, _, _)| k == b"mdat") {
        // A cut-off recording's mdat size often still claims the full span.
        let end = if *end <= start + 8 { data.len() } else { *end };
        return (*start, end.min(data.len()));
    }
    let after = top.last().map_or(0, |(_, _, end)| *end);
    (after, data.len())
}

fn nal_header_ok(hevc: bool, header: &[u8]) -> bool {
    if hevc {
        header.len() >= 2
            && header[0] & 0x80 == 0
            && ((header[0] >> 1) & 0x3F) <= 40
            && header[0] & 1 == 0
            && header[1] >> 3 == 0
            && header[1] & 7 != 0
    } else {
        let Some(&h) = header.first() else {
            return false;
        };
        let kind = h & 0x1F;
        let ref_idc = h & 0x60;
        h & 0x80 == 0
            && matches!(kind, 1..=12 | 19 | 20)
            && !(kind == 5 && ref_idc == 0)
            && !(matches!(kind, 6 | 9 | 10 | 11 | 12) && ref_idc != 0)
    }
}

fn is_slice(hevc: bool, header: u8) -> bool {
    if hevc {
        (header >> 1) & 0x3F <= 21
    } else {
        matches!(header & 0x1F, 1 | 5)
    }
}

/// Exp-Golomb reader over the start of a NAL payload (emulation-prevention
/// bytes dropped).
struct Bits {
    bytes: Vec<u8>,
    at: usize,
}

impl Bits {
    fn new(payload: &[u8]) -> Self {
        Self::with_limit(payload, 32)
    }

    fn with_limit(payload: &[u8], limit: usize) -> Self {
        let mut bytes = Vec::with_capacity(payload.len().min(limit));
        let mut zeros = 0;
        for &byte in payload.iter().take(limit) {
            if zeros >= 2 && byte == 3 {
                zeros = 0;
                continue;
            }
            zeros = if byte == 0 { zeros + 1 } else { 0 };
            bytes.push(byte);
        }
        Self { bytes, at: 0 }
    }

    fn bit(&mut self) -> Option<u32> {
        let byte = *self.bytes.get(self.at / 8)?;
        let bit = (byte >> (7 - self.at % 8)) & 1;
        self.at += 1;
        Some(bit as u32)
    }

    fn ue(&mut self) -> Option<u32> {
        let mut zeros = 0;
        while self.bit()? == 0 {
            zeros += 1;
            if zeros > 24 {
                return None;
            }
        }
        let mut value = 1u32;
        for _ in 0..zeros {
            value = (value << 1) | self.bit()?;
        }
        Some(value - 1)
    }

    fn se(&mut self) -> Option<i32> {
        let k = self.ue()? as i64;
        Some(if k % 2 == 1 { (k + 1) / 2 } else { -(k / 2) } as i32)
    }

    fn bits(&mut self, n: u32) -> Option<u32> {
        (0..n).try_fold(0u32, |acc, _| Some((acc << 1) | self.bit()?))
    }
}

/// A valid H.264 `slice_type` for the NAL `header`: an IDR picture holds
/// only I/SI slices.
fn h264_slice_type_ok(header: u8, slice_type: u32) -> bool {
    slice_type <= 9 && (header & 0x1F != 5 || matches!(slice_type % 5, 2 | 4))
}

/// How an H.264 slice header codes `frame_num`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct FrameNumCoding {
    /// log2(MaxFrameNum).
    bits: u32,
    /// `separate_colour_plane_flag`: a 2-bit colour_plane_id comes first.
    colour_plane: bool,
}

/// The `frame_num` coding of an H.264 SPS (`sps`: NAL header included).
fn frame_num_coding(sps: &[u8]) -> Option<FrameNumCoding> {
    let mut bits = Bits::with_limit(sps.get(1..)?, 512);
    let profile = bits.bits(8)?;
    bits.bits(16)?; // constraint flags, level
    bits.ue()?; // seq_parameter_set_id
    let mut colour_plane = false;
    if matches!(
        profile,
        100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 | 138 | 139 | 134 | 135
    ) {
        let chroma = bits.ue()?;
        if chroma == 3 {
            colour_plane = bits.bit()? == 1;
        }
        bits.ue()?; // bit_depth_luma_minus8
        bits.ue()?; // bit_depth_chroma_minus8
        bits.bit()?; // qpprime_y_zero_transform_bypass_flag
        if bits.bit()? == 1 {
            for list in 0..if chroma == 3 { 12 } else { 8 } {
                if bits.bit()? == 0 {
                    continue;
                }
                let (mut last, mut next) = (8i32, 8i32);
                for _ in 0..if list < 6 { 16 } else { 64 } {
                    if next != 0 {
                        next = (last + bits.se()? + 256) % 256;
                    }
                    if next != 0 {
                        last = next;
                    }
                }
            }
        }
    }
    let minus4 = bits.ue()?;
    (minus4 <= 12).then_some(FrameNumCoding {
        bits: minus4 + 4,
        colour_plane,
    })
}

/// The H.264 `frame_num` coding of `config` (None for H.265 or an SPS that
/// does not parse: then no continuity is checked).
fn config_frame_num(config: &VideoConfig) -> Option<FrameNumCoding> {
    if config.hevc {
        return None;
    }
    config
        .parameter_sets
        .iter()
        .find(|set| set.first().map(|h| h & 0x1F) == Some(7))
        .and_then(|sps| frame_num_coding(sps))
}

/// Whether the H.264 slice NAL `nal` (header included) can follow a
/// picture with `frame_num` `last` across lost bytes: an IDR carries 0;
/// any other picture repeats `last` (another slice, a non-reference
/// picture) or adds one. A false match in audio or foreign bytes reads a
/// random number and almost never fits.
fn frame_num_continues(coding: Option<FrameNumCoding>, last: Option<u32>, nal: &[u8]) -> bool {
    let Some(coding) = coding else {
        return true;
    };
    let Some(frame_num) = slice_frame_num(coding, nal) else {
        return false;
    };
    if nal[0] & 0x1F == 5 {
        return frame_num == 0;
    }
    let Some(last) = last else {
        return true;
    };
    let max = 1u32 << coding.bits;
    frame_num == last || frame_num == (last + 1) % max
}

/// `frame_num` of the H.264 slice NAL `nal` (header included).
fn slice_frame_num(coding: FrameNumCoding, nal: &[u8]) -> Option<u32> {
    let mut bits = Bits::new(nal.get(1..)?);
    bits.ue()?; // first_mb_in_slice
    bits.ue()?; // slice_type
    bits.ue()?; // pic_parameter_set_id
    if coding.colour_plane {
        bits.bits(2)?;
    }
    bits.bits(coding.bits)
}

/// The PPS ids a configuration declares (empty: accept any).
fn pps_ids(config: &VideoConfig) -> Vec<u32> {
    config
        .parameter_sets
        .iter()
        .filter_map(|set| {
            let is_pps = if config.hevc {
                set.first().map(|h| (h >> 1) & 0x3F) == Some(34)
            } else {
                set.first().map(|h| h & 0x1F) == Some(8)
            };
            let skip = if config.hevc { 2 } else { 1 };
            is_pps.then(|| Bits::new(set.get(skip..)?).ue()).flatten()
        })
        .collect()
}

/// How strictly a NAL position is checked.
#[derive(Clone, Copy, PartialEq)]
enum NalCheck {
    /// Right after a video NAL: any slice of a known PPS.
    Continue,
    /// After foreign bytes: only the first slice of a picture (or a
    /// parameter-set/SEI/AUD unit followed by another video NAL).
    Resync,
}

/// What the walk needs to know about the stream.
struct NalStream<'a> {
    data: &'a [u8],
    end: usize,
    n: usize,
    hevc: bool,
    pps: Vec<u32>,
    frame_num: Option<FrameNumCoding>,
}

impl NalStream<'_> {
    /// Whether the unit at `pos` continues a picture numbered `last` (non-
    /// slice units always do).
    fn continues(&self, last: Option<u32>, pos: usize) -> bool {
        let at = pos + self.n;
        let nal = &self.data[at..self.end.min(at + 40)];
        self.hevc
            || !nal.first().is_some_and(|h| is_slice(false, *h))
            || frame_num_continues(self.frame_num, last, nal)
    }

    fn frame_num_of(&self, nal: &[u8]) -> Option<u32> {
        if self.hevc || !nal.first().is_some_and(|h| is_slice(false, *h)) {
            return None;
        }
        slice_frame_num(self.frame_num?, nal)
    }

    /// Length (prefix included) of a plausible NAL unit at `pos`.
    fn basic(&self, pos: usize) -> Option<usize> {
        let (data, n) = (self.data, self.n);
        if pos + n + 2 > self.end {
            return None;
        }
        let len = data[pos..pos + n]
            .iter()
            .fold(0usize, |acc, byte| (acc << 8) | *byte as usize);
        if !(2..=64 * 1024 * 1024).contains(&len) || pos + n + len > self.end {
            return None;
        }
        nal_header_ok(self.hevc, &data[pos + n..pos + n + 2]).then_some(n + len)
    }

    /// Whether the H.264 slice header at `payload` (after the NAL header)
    /// reads like this stream's: a valid slice type and a declared PPS id;
    /// at a resync point also the first slice of its picture.
    fn slice_ok(&self, header: u8, payload: &[u8], check: NalCheck) -> bool {
        let mut bits = Bits::new(payload);
        let (Some(first_mb), Some(slice_type)) = (bits.ue(), bits.ue()) else {
            return false;
        };
        if !h264_slice_type_ok(header, slice_type) {
            return false;
        }
        let Some(pps) = bits.ue() else {
            return false;
        };
        (self.pps.is_empty() || self.pps.contains(&pps))
            && (check == NalCheck::Continue || first_mb == 0)
    }

    /// Length of the NAL unit at `pos` when it passes `check`.
    fn nal(&self, pos: usize, check: NalCheck) -> Option<usize> {
        let len = self.basic(pos)?;
        let header_len = if self.hevc { 2 } else { 1 };
        let header = self.data[pos + self.n];
        if is_slice(self.hevc, header) {
            let payload_start = pos + self.n + header_len;
            let payload = &self.data[payload_start..(pos + len).min(payload_start + 32)];
            if self.hevc {
                // first_slice_segment_in_pic_flag; IRAP types (16..=23)
                // then carry no_output_of_prior_pics_flag before the PPS id.
                let mut bits = Bits::new(payload);
                let first = bits.bit()? == 1;
                if (16..=23).contains(&((header >> 1) & 0x3F)) {
                    bits.bit()?;
                }
                let pps = bits.ue()?;
                let known = self.pps.is_empty() || self.pps.contains(&pps);
                return (known && (check == NalCheck::Continue || first)).then_some(len);
            }
            return self.slice_ok(header, payload, check).then_some(len);
        }
        // Parameter sets, SEI, AUD: trusted only when a video NAL follows.
        if check == NalCheck::Resync {
            let next = pos + len;
            if next != self.end && self.basic(next).is_none() {
                return None;
            }
        }
        Some(len)
    }
}

/// What the NAL walk recovered from an mdat.
#[derive(Debug, Default, PartialEq)]
struct NalWalk {
    annexb: Vec<u8>,
    units: u64,
    slices: u64,
    skipped: u64,
    resyncs: u64,
    /// Slices before the first keyframe, left out.
    leading_dropped: u64,
    stopped_at: Option<u64>,
}

/// Whether the slice NAL `nal` (header included) starts a decodable
/// picture: an IDR/IRAP unit, or an H.264 I/SI slice.
fn key_slice(hevc: bool, nal: &[u8]) -> bool {
    let Some(&header) = nal.first() else {
        return false;
    };
    if hevc {
        return (16..=23).contains(&((header >> 1) & 0x3F));
    }
    if header & 0x1F == 5 {
        return true;
    }
    let mut bits = Bits::new(&nal[1..nal.len().min(17)]);
    let _first_mb = bits.ue();
    matches!(bits.ue().map(|t| t % 5), Some(2 | 4))
}

/// Walks the length-prefixed NAL units of `data[start..end]` into an
/// Annex-B stream. Bytes that are not video (interleaved audio chunks,
/// foreign clusters) are skipped up to the next picture start; slices
/// before the first keyframe are left out unless the stream has none.
fn walk_nals(data: &[u8], start: usize, end: usize, config: &VideoConfig) -> NalWalk {
    let walk = walk_nals_from(data, start, end, config, true);
    if walk.slices == 0 && walk.leading_dropped > 0 {
        return walk_nals_from(data, start, end, config, false);
    }
    walk
}

fn walk_nals_from(
    data: &[u8],
    start: usize,
    end: usize,
    config: &VideoConfig,
    wait_for_key: bool,
) -> NalWalk {
    let stream = NalStream {
        data,
        end,
        n: config.length_size,
        hevc: config.hevc,
        pps: pps_ids(config),
        frame_num: config_frame_num(config),
    };
    let mut walk = NalWalk::default();
    let mut pos = start;
    let mut check = NalCheck::Resync;
    let mut started = !wait_for_key;
    let mut last_frame_num = None;
    while pos < end {
        if let Some(len) = stream.nal(pos, check) {
            let nal = &data[pos + stream.n..pos + len];
            let slice = is_slice(config.hevc, nal[0]);
            if let Some(frame_num) = stream.frame_num_of(nal) {
                last_frame_num = Some(frame_num);
            }
            if slice && !started && !key_slice(config.hevc, nal) {
                walk.leading_dropped += 1;
            } else {
                started |= slice;
                walk.annexb.extend_from_slice(&[0, 0, 0, 1]);
                walk.annexb.extend_from_slice(nal);
                walk.units += 1;
                if slice {
                    walk.slices += 1;
                }
            }
            pos += len;
            check = NalCheck::Continue;
            continue;
        }
        let window = end.min(pos + RESYNC_WINDOW);
        let resync = |q: &usize| stream.nal(*q, NalCheck::Resync).is_some();
        // The picture that continues the numbering first; only when none
        // does (frames really lost) any picture start.
        let next = (pos + 1..window)
            .find(|q| resync(q) && stream.continues(last_frame_num, *q))
            .or_else(|| (pos + 1..window).find(resync));
        match next {
            Some(q) => {
                walk.skipped += (q - pos) as u64;
                if walk.units > 0 {
                    walk.resyncs += 1;
                }
                pos = q;
                check = NalCheck::Resync;
            }
            None => {
                walk.skipped += (end - pos) as u64;
                walk.stopped_at = Some(pos as u64);
                break;
            }
        }
    }
    walk
}

/// Healthy videos of the case to borrow a codec configuration from, best
/// first: same channel (front/rear), then same extension.
fn reference_candidates(case_dir: &Path, source: &Path) -> Vec<PathBuf> {
    let Ok(text) = fs::read_to_string(case_dir.join("db/videos.jsonl")) else {
        return Vec::new();
    };
    let name = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let channel = crate::join::channel_code_of_text(&name);
    let source_key = crate::lazy_extract::path_key(source);
    let mut rows: Vec<(u8, PathBuf)> = text
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|row| {
            matches!(
                row.get("extension").and_then(|v| v.as_str()),
                Some("mp4" | "mov" | "m4v")
            )
        })
        .filter_map(|row| {
            let path = PathBuf::from(row.get("source_path")?.as_str()?);
            if crate::lazy_extract::path_key(&path) == source_key {
                return None;
            }
            let relative = row
                .get("relative_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let rank = if channel.is_some() && crate::join::channel_code(relative) == channel {
                0
            } else {
                1
            };
            Some((rank, path))
        })
        .collect();
    rows.sort_by_key(|(rank, _)| *rank);
    rows.into_iter().map(|(_, path)| path).take(12).collect()
}

/// Decode errors ffmpeg reports for the first seconds of `video`.
fn decode_errors(ffmpeg: &str, video: &Path) -> usize {
    let mut command = Command::new(ffmpeg);
    command
        .args(["-v", "error", "-t", "4", "-i"])
        .arg(video)
        .args(["-f", "null", "-"]);
    match crate::util::run_with_timeout(&mut command, Some(120)) {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stderr)
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count(),
        _ => usize::MAX,
    }
}

/// The best remux so far: (decode errors, config, reference (path, sha), walk).
type Attempt = (usize, VideoConfig, Option<(PathBuf, String)>, NalWalk);

/// Rebuilds the NAL units of `data[range]` into an MP4 at `target`; with
/// `own_moov` the file's own (table-broken) moov is tried first.
fn repair_mp4(
    case_dir: &Path,
    source: &Path,
    data: &[u8],
    (start, end): (usize, usize),
    source_sha256: &str,
    target: &Path,
    own_moov: bool,
) -> Result<RepairReport, String> {
    let ffmpeg = crate::tool_policy::resolve_tool_binary("ffmpeg", &["ffmpeg"])
        .map_err(|err| format!("{err} (the MP4 repair remuxes with ffmpeg)"))?;
    // Configurations to try: the file's own moov (tables broken, config
    // intact), then healthy videos of the case.
    let mut configs: Vec<(VideoConfig, Option<(PathBuf, String)>)> = Vec::new();
    if own_moov && let Some(own) = video_config(data) {
        configs.push((own, None));
    }
    for reference in reference_candidates(case_dir, source) {
        if configs.len() > MAX_REFERENCES {
            break;
        }
        if crate::lazy_extract::ensure_materialized(case_dir, &reference).unwrap_or(false)
            && let Ok(bytes) = fs::read(&reference)
            && let Some(config) = video_config(&bytes)
            && !configs.iter().any(|(known, _)| known == &config)
        {
            let sha = crate::sha256::digest_bytes(&bytes);
            configs.push((config, Some((reference, sha))));
        }
    }
    if configs.is_empty() {
        return Err(
            "코덱 설정을 찾지 못했습니다 — 같은 카드의 정상 MP4 영상(참조)이 필요합니다"
                .to_string(),
        );
    }

    let part = crate::join::pid_part_path(target, std::process::id());
    let mut best: Option<Attempt> = None;
    let mut last_error = String::new();
    for (config, reference) in configs {
        let walk = walk_nals(data, start, end, &config);
        if walk.slices == 0 {
            last_error = "mdat에서 영상 프레임(NAL)을 찾지 못했습니다".to_string();
            continue;
        }
        let mut stream = Vec::with_capacity(walk.annexb.len() + 256);
        for set in &config.parameter_sets {
            stream.extend_from_slice(&[0, 0, 0, 1]);
            stream.extend_from_slice(set);
        }
        stream.extend_from_slice(&walk.annexb);
        let raw = part.with_extension(if config.hevc { "hevc" } else { "h264" });
        fs::write(&raw, &stream)
            .map_err(|err| format!("failed to write {}: {err}", raw.display()))?;
        let _ = fs::remove_file(&part);
        let mut command = Command::new(&ffmpeg);
        command
            .args(["-y", "-v", "error", "-fflags", "+genpts", "-r"])
            .arg(format!("{:.3}", config.fps))
            .args(["-f", if config.hevc { "hevc" } else { "h264" }, "-i"])
            .arg(&raw)
            .args([
                "-c:v",
                "copy",
                "-an",
                "-movflags",
                "+faststart",
                "-f",
                "mp4",
            ])
            .arg(&part);
        let remuxed = crate::util::run_with_timeout(&mut command, Some(600));
        let _ = fs::remove_file(&raw);
        match remuxed {
            Ok(output) if output.status.success() && part.is_file() => {}
            Ok(output) => {
                last_error = format!(
                    "ffmpeg remux failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                );
                continue;
            }
            Err(err) => {
                last_error = err;
                continue;
            }
        }
        let errors = decode_errors(&ffmpeg, &part);
        let better = best.as_ref().is_none_or(|(known, ..)| errors < *known);
        if better {
            let keep = target.with_extension("best.part");
            let _ = fs::remove_file(&keep);
            fs::rename(&part, &keep).map_err(|err| format!("failed to keep candidate: {err}"))?;
            best = Some((errors, config, reference, walk));
            if errors == 0 {
                break;
            }
        } else {
            let _ = fs::remove_file(&part);
        }
    }
    let Some((errors, config, reference, walk)) = best else {
        return Err(last_error);
    };
    let kept = target.with_extension("best.part");
    publish_part(&kept, target)?;
    let mut notes = vec![format!(
        "mdat의 NAL 단위 {}개(프레임 조각 {}개)를 따라가 재구성했습니다 — 오디오는 표(moov) 없이 경계를 알 수 없어 제외했습니다",
        walk.units, walk.slices
    )];
    if walk.leading_dropped > 0 {
        notes.push(leading_note(walk.leading_dropped));
    }
    match &reference {
        Some((path, _)) => notes.push(format!(
            "코덱 설정(SPS/PPS)과 프레임레이트({:.2}fps)는 참조 영상 {}에서 가져왔습니다",
            config.fps,
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        )),
        None => notes.push(format!(
            "코덱 설정과 프레임레이트({:.2}fps)는 파일 자체의 moov에서 가져왔습니다",
            config.fps
        )),
    }
    if walk.resyncs > 0 || walk.skipped > 0 {
        notes.push(format!(
            "영상이 아닌 구간 {}곳({} 바이트)을 건너뛰었습니다",
            walk.resyncs, walk.skipped
        ));
    }
    if errors > 0 {
        notes.push(format!(
            "시험 디코드에서 오류 {errors}건 — 일부 프레임이 깨져 보일 수 있습니다"
        ));
    }
    Ok(RepairReport {
        method: "mp4-nal-walk",
        output_path: target.to_string_lossy().to_string(),
        codec: Some(if config.hevc { "hevc" } else { "h264" }.to_string()),
        fps: Some((config.fps * 1000.0).round() / 1000.0),
        reference_path: reference
            .as_ref()
            .map(|(path, _)| path.to_string_lossy().to_string()),
        reference_sha256: reference.map(|(_, sha)| sha),
        units_kept: walk.units,
        bytes_dropped: walk.skipped,
        resyncs: walk.resyncs,
        stopped_at: walk.stopped_at,
        audio: "omitted",
        notes,
        ..empty_report(source, source_sha256)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = id.to_vec();
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
        if body.len() % 2 == 1 {
            out.push(0);
        }
        out
    }

    fn list(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut inner = kind.to_vec();
        inner.extend_from_slice(body);
        chunk(b"LIST", &inner)
    }

    /// A small AVI: one video stream, `frames` MJPEG chunks interleaved
    /// with audio, no idx1, RIFF size 0 (a recording cut off).
    fn broken_avi(frames: usize, garbage_tail: bool) -> Vec<u8> {
        let mut avih = vec![0u8; 56];
        avih[16..20].copy_from_slice(&0u32.to_le_bytes());
        let mut strh = vec![0u8; 56];
        strh[..4].copy_from_slice(b"vids");
        strh[4..8].copy_from_slice(b"MJPG");
        let strl = list(
            b"strl",
            &[chunk(b"strh", &strh), chunk(b"strf", &[0u8; 40])].concat(),
        );
        let hdrl = list(b"hdrl", &[chunk(b"avih", &avih), strl].concat());
        let mut movi_body = Vec::new();
        for i in 0..frames {
            movi_body.extend(chunk(b"00dc", &[0xFF, 0xD8, i as u8, 1, 2]));
            movi_body.extend(chunk(b"01wb", &[9u8; 6]));
        }
        if garbage_tail {
            movi_body.extend_from_slice(&[0xAB; 40]);
        }
        let mut movi = b"LIST".to_vec();
        movi.extend_from_slice(&0u32.to_le_bytes());
        movi.extend_from_slice(b"movi");
        movi.extend(movi_body);
        let mut out = b"RIFF\0\0\0\0AVI ".to_vec();
        out.extend(hdrl);
        out.extend(movi);
        out
    }

    #[test]
    fn avi_data_end_follows_the_chain_not_the_size_fields() {
        let intact = broken_avi(4, false);
        // Every prefix shorter than the file asks for more data…
        assert_eq!(avi_data_end(&intact[..intact.len() - 3], false), None);
        // …and the whole file ends at its last chunk.
        assert_eq!(avi_data_end(&intact, true), Some(intact.len()));
        // Foreign bytes after the last frame end the chain right there.
        let broken = broken_avi(4, true);
        assert_eq!(avi_data_end(&broken, false), Some(intact.len()));
        assert_eq!(avi_data_end(b"not an avi at all", false), Some(0));
    }

    #[test]
    fn avi_reindex_rebuilds_index_sizes_and_counts() {
        let source = broken_avi(5, true);
        let (fixed, stats) = repair_avi(&source).unwrap();
        assert_eq!(stats.video_frames, 5);
        assert_eq!(stats.audio_chunks, 5);
        assert!(!stats.had_index);
        assert!(stats.broken_at.is_some(), "garbage tail ends the chain");
        assert_eq!(le32(&fixed, 4).unwrap() as usize, fixed.len() - 8);
        let idx = fixed.windows(4).rposition(|w| w == b"idx1").unwrap();
        assert_eq!(le32(&fixed, idx + 4).unwrap(), 10 * 16);
        // idx1 offsets point at the chunks relative to the movi fourcc.
        let movi = fixed.windows(4).position(|w| w == b"movi").unwrap();
        let first_offset = le32(&fixed, idx + 8 + 8).unwrap() as usize;
        assert_eq!(
            &fixed[movi + first_offset..movi + first_offset + 4],
            b"00dc"
        );
        // avih total frames and the video strh length now read 5.
        let avih = fixed.windows(4).position(|w| w == b"avih").unwrap();
        assert_eq!(le32(&fixed, avih + 8 + 16), Some(5));
        assert_eq!(le32(&fixed, avih + 8 + 12).unwrap() & 0x10, 0x10);
        let strh = fixed.windows(4).position(|w| w == b"strh").unwrap();
        assert_eq!(le32(&fixed, strh + 8 + 32), Some(5));
        assert!(repair_avi(b"RIFF\0\0\0\0AVI JUNK").is_err());
    }

    /// A length-prefixed NAL: header, slice-header bits, then filler.
    fn avcc_nal(header: u8, slice_bits: &[u8], filler: usize) -> Vec<u8> {
        let body_len = 1 + slice_bits.len() + filler;
        let mut out = (body_len as u32).to_be_bytes().to_vec();
        out.push(header);
        out.extend_from_slice(slice_bits);
        out.extend(std::iter::repeat_n(0x5A, filler));
        out
    }

    // first_mb_in_slice=0 '1' · slice_type ue · pic_parameter_set_id=0 '1'
    const IDR_SLICE: &[u8] = &[0x88, 0x80]; // type 7 (I)
    const P_SLICE: &[u8] = &[0x9A]; // type 5 (P)

    fn h264_config() -> VideoConfig {
        VideoConfig {
            hevc: false,
            length_size: 4,
            parameter_sets: vec![vec![0x67, 0x64, 0, 0x28], vec![0x68, 0x80]],
            fps: 30.0,
        }
    }

    #[test]
    fn nal_walk_resyncs_over_audio_between_single_frames() {
        // ffmpeg/dashcam interleaving: one video sample, one audio chunk.
        let audio = [0xFF, 0xF1, 0x50, 0x80, 0x02, 0x1F, 0xFC, 0x21, 0x10, 0x05];
        // An audio run that happens to look like a P slice of PPS id 3.
        let fake = [0, 0, 0, 5, 0x41, 0x98, 0x40, 0, 0];
        let mut mdat = Vec::new();
        mdat.extend(avcc_nal(0x06, &[5, 1, 0x80], 0)); // SEI
        mdat.extend(avcc_nal(0x65, IDR_SLICE, 40));
        mdat.extend_from_slice(&audio);
        mdat.extend(avcc_nal(0x41, P_SLICE, 30));
        mdat.extend_from_slice(&fake);
        mdat.extend_from_slice(&audio);
        mdat.extend(avcc_nal(0x41, P_SLICE, 30));
        mdat.extend(avcc_nal(0x41, P_SLICE, 30));
        let walk = walk_nals(&mdat, 0, mdat.len(), &h264_config());
        assert_eq!(walk.units, 5, "{walk:?}");
        assert_eq!(walk.slices, 4);
        assert_eq!(walk.resyncs, 2);
        assert_eq!(walk.skipped, (audio.len() * 2 + fake.len()) as u64);
        assert!(walk.annexb.starts_with(&[0, 0, 0, 1, 0x06]));
        assert_eq!(pps_ids(&h264_config()), vec![0]);
    }

    #[test]
    fn frame_num_continuity_rejects_false_picture_starts() {
        // Baseline SPS: sps_id 0, log2_max_frame_num_minus4 0 → 4 bits.
        let coding = frame_num_coding(&[0x67, 0x42, 0xC0, 0x1E, 0xC0]);
        assert_eq!(
            coding,
            Some(FrameNumCoding {
                bits: 4,
                colour_plane: false
            })
        );
        // P slice (type 5), PPS 0, frame_num n: 1 00110 1 nnnn.
        let p = |n: u8| [0x41, 0x9A | n >> 3, (n & 7) << 5 | 0x1F];
        assert_eq!(slice_frame_num(coding.unwrap(), &p(3)), Some(3));
        assert!(frame_num_continues(coding, Some(3), &p(3)));
        assert!(frame_num_continues(coding, Some(3), &p(4)));
        assert!(frame_num_continues(coding, Some(15), &p(0)));
        assert!(!frame_num_continues(coding, Some(3), &p(9)));
        // An IDR picture carries frame_num 0, and only I slices.
        assert!(frame_num_continues(coding, Some(7), &[0x65, 0x88, 0x80]));
        assert!(!frame_num_continues(coding, Some(7), &[0x65, 0x88, 0x90]));
        assert!(!h264_slice_type_ok(0x65, 5));
        assert!(h264_slice_type_ok(0x41, 5));
        // Without a parsable SPS nothing is checked.
        assert!(frame_num_continues(None, Some(3), &p(9)));
    }

    #[test]
    fn nal_walk_starts_at_the_first_keyframe() {
        let mut run = Vec::new();
        run.extend(avcc_nal(0x41, P_SLICE, 30));
        run.extend(avcc_nal(0x41, P_SLICE, 30));
        run.extend(avcc_nal(0x65, IDR_SLICE, 40));
        run.extend(avcc_nal(0x41, P_SLICE, 30));
        let walk = walk_nals(&run, 0, run.len(), &h264_config());
        assert_eq!((walk.slices, walk.leading_dropped), (2, 2), "{walk:?}");
        assert!(walk.annexb.starts_with(&[0, 0, 0, 1, 0x65]));
        // No keyframe at all: everything is kept rather than nothing.
        let mut tail = Vec::new();
        tail.extend(avcc_nal(0x41, P_SLICE, 30));
        tail.extend(avcc_nal(0x41, P_SLICE, 30));
        let walk = walk_nals(&tail, 0, tail.len(), &h264_config());
        assert_eq!((walk.slices, walk.leading_dropped), (2, 0));
    }

    #[test]
    fn avi_repair_starts_at_the_first_keyframe() {
        let chunk = |id: &[u8], body: &[u8]| {
            let mut out = id.to_vec();
            out.extend_from_slice(&(body.len() as u32).to_le_bytes());
            out.extend_from_slice(body);
            if body.len() % 2 == 1 {
                out.push(0);
            }
            out
        };
        let p_frame: &[u8] = &[0, 0, 0, 1, 0x41, 0x9A, 1, 2];
        let i_frame: &[u8] = &[0, 0, 0, 1, 0x65, 0x88, 1, 2];
        let mut movi = b"movi".to_vec();
        movi.extend(chunk(b"00dc", p_frame));
        movi.extend(chunk(b"01wb", &[9, 9]));
        movi.extend(chunk(b"00dc", i_frame));
        movi.extend(chunk(b"00dc", p_frame));
        let mut strh = b"vids".to_vec();
        strh.extend_from_slice(b"H264");
        strh.resize(56, 0);
        let mut hdrl = b"hdrl".to_vec();
        hdrl.extend(chunk(b"avih", &[0; 56]));
        hdrl.extend(chunk(b"strh", &strh));
        let mut avi = b"RIFF\0\0\0\0AVI ".to_vec();
        avi.extend(chunk(b"LIST", &hdrl));
        avi.extend(chunk(b"LIST", &movi));
        let (_, stats) = repair_avi(&avi).expect("repair");
        assert_eq!((stats.video_frames, stats.leading_dropped), (2, 1));
        assert_eq!(stats.audio_chunks, 0);
    }

    #[test]
    fn exp_golomb_reads_ue_and_skips_emulation_prevention() {
        let mut bits = Bits::new(&[0x88, 0x80]);
        assert_eq!(
            (bits.ue(), bits.ue(), bits.ue()),
            (Some(0), Some(7), Some(0))
        );
        // 00 00 03 01: the 03 is dropped, leaving 00 00 01.
        let bits = Bits::new(&[0, 0, 3, 1]);
        assert_eq!(bits.bytes, vec![0, 0, 1]);
    }

    #[test]
    fn parses_avcc_parameter_sets() {
        let mut avcc = vec![1, 0x64, 0, 0x28, 0xFF, 0xE1];
        avcc.extend_from_slice(&3u16.to_be_bytes());
        avcc.extend_from_slice(&[0x67, 1, 2]);
        avcc.push(1);
        avcc.extend_from_slice(&2u16.to_be_bytes());
        avcc.extend_from_slice(&[0x68, 9]);
        let (length_size, sets) = parse_avcc(&avcc).unwrap();
        assert_eq!(length_size, 4);
        assert_eq!(sets, vec![vec![0x67, 1, 2], vec![0x68, 9]]);
    }

    #[test]
    fn mdat_range_survives_a_cut_off_size() {
        let mut file = 24u32.to_be_bytes().to_vec();
        file.extend_from_slice(b"ftypisom");
        file.extend_from_slice(&[0; 12]);
        file.extend_from_slice(&0u32.to_be_bytes());
        file.extend_from_slice(b"mdat");
        file.extend_from_slice(&[7; 100]);
        assert_eq!(mdat_range(&file), (32, file.len()));
    }
}
