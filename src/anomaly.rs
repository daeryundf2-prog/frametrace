//! Candidate anomaly findings for examiner review (ROADMAP M4).
//!
//! Labels are `candidate-finding` only — never claim tampering, authenticity
//! failure, or legal proof. Findings are written to
//! `evidence/logs/anomaly-log.jsonl` and surfaced in the case report.

use crate::audit;
use crate::util::{json_escape, now_unix, read_to_string};
use std::path::{Path, PathBuf};

pub const LABEL: &str = "candidate-finding";
const GAP_SECS: u64 = 7 * 24 * 60 * 60;
/// DAV frames normally arrive at 25-30fps (0.03-0.04s apart). A gap an order
/// of magnitude larger suggests dropped frames or recorder pauses. The
/// threshold is compared against the packed-date second span; the
/// free-running ms counter only refines the sub-second remainder.
const DAV_FRAME_GAP_SECS: f64 = 2.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub kind: &'static str,
    pub selector: String,
    pub source_path: String,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct IndexedRow {
    pub id: String,
    pub source_path: String,
    pub sha256: Option<String>,
    pub size_bytes: Option<u64>,
    pub modified_unix: Option<u64>,
    pub duration_seconds: Option<f64>,
    pub format_name: Option<String>,
    pub video_codec: Option<String>,
    pub ffprobe_ok: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct AnomalyScanResult {
    pub findings: Vec<Finding>,
    pub log_path: PathBuf,
    pub scanned_unix: u64,
}

/// Scan the case index for candidate anomalies and append a chained log.
///
/// `rehash` selects the integrity lane: `true` re-hashes every indexed file
/// (full revalidation, slow on large cases); `false` trusts the stored index
/// hashes and only flags records whose filesystem metadata drifted from what
/// was indexed — cheap enough to run inside `make-report` by default.
pub fn scan_case(case_dir: &Path, rehash: bool) -> Result<AnomalyScanResult, String> {
    let rows = read_indexed_rows(case_dir)?;
    let mut findings = Vec::new();
    if rehash {
        findings.extend(hash_revalidation_findings(&rows));
    } else {
        findings.extend(index_staleness_findings(&rows));
    }
    findings.extend(timestamp_findings(&rows));
    findings.extend(container_stream_findings(&rows));
    findings.extend(dav_frame_gap_findings(case_dir, &rows));

    let scanned_unix = now_unix()?;
    let log_path = case_dir.join("evidence/logs/anomaly-log.jsonl");
    for finding in &findings {
        let line = format!(
            "{{\"schema_version\":1,\"event\":\"anomaly-scan\",\"scanned_unix\":{},\"label\":\"{}\",\"kind\":\"{}\",\"selector\":\"{}\",\"source_path\":\"{}\",\"detail\":\"{}\"}}",
            scanned_unix,
            LABEL,
            json_escape(finding.kind),
            json_escape(&finding.selector),
            json_escape(&finding.source_path),
            json_escape(&finding.detail),
        );
        audit::append_chained_jsonl(&log_path, &line)?;
    }
    if findings.is_empty() {
        let line = format!(
            "{{\"schema_version\":1,\"event\":\"anomaly-scan\",\"scanned_unix\":{},\"label\":\"none\",\"kind\":\"none\",\"selector\":\"\",\"source_path\":\"\",\"detail\":\"no candidate findings on indexed videos\"}}",
            scanned_unix
        );
        audit::append_chained_jsonl(&log_path, &line)?;
    }
    // Every scan records which integrity lane ran so a report generated from
    // stored hashes is never mistaken for a full revalidation pass.
    let line = format!(
        "{{\"schema_version\":1,\"event\":\"anomaly-scan-run\",\"scanned_unix\":{},\"hash_mode\":\"{}\",\"finding_count\":{},\"detail\":\"{}\"}}",
        scanned_unix,
        if rehash { "rehash" } else { "stored" },
        findings.len(),
        json_escape(if rehash {
            "live re-hashing of every indexed file"
        } else {
            "stored index hashes only; pass --rehash for full revalidation"
        }),
    );
    audit::append_chained_jsonl(&log_path, &line)?;
    Ok(AnomalyScanResult {
        findings,
        log_path,
        scanned_unix,
    })
}

/// Compare a live digest against the indexed hash for one selector, if known.
pub fn hash_mismatch_finding(
    case_dir: &Path,
    selector: &str,
    live_sha256: &str,
) -> Result<Option<Finding>, String> {
    let rows = read_indexed_rows(case_dir)?;
    let Some(row) = rows.iter().find(|row| row.id == selector) else {
        return Ok(None);
    };
    let Some(indexed) = row.sha256.as_deref() else {
        return Ok(None);
    };
    if indexed.eq_ignore_ascii_case(live_sha256) {
        return Ok(None);
    }
    Ok(Some(Finding {
        kind: "hash-revalidation-mismatch",
        selector: row.id.clone(),
        source_path: row.source_path.clone(),
        detail: format!("indexed sha256 {indexed} != live sha256 {live_sha256}"),
    }))
}

fn hash_revalidation_findings(rows: &[IndexedRow]) -> Vec<Finding> {
    let mut out = Vec::new();
    for row in rows {
        let Some(indexed) = row.sha256.as_deref() else {
            continue;
        };
        let path = Path::new(&row.source_path);
        if !path.is_file() {
            continue;
        }
        let Ok(live) = audit::digest_file(path) else {
            continue;
        };
        if !indexed.eq_ignore_ascii_case(&live) {
            out.push(Finding {
                kind: "hash-revalidation-mismatch",
                selector: row.id.clone(),
                source_path: row.source_path.clone(),
                detail: format!("indexed sha256 {indexed} != live sha256 {live}"),
            });
        }
    }
    out
}

/// Cheap staleness lane used when `scan_case` runs without re-hashing. Only
/// `metadata()` calls (no file reads) compare each indexed row against the
/// live filesystem: a missing or drifted file means the stored sha256 may no
/// longer describe the bytes on disk — reported as candidate staleness, never
/// as a verified mismatch.
fn index_staleness_findings(rows: &[IndexedRow]) -> Vec<Finding> {
    let mut out = Vec::new();
    for row in rows {
        let Some(indexed) = row.sha256.as_deref() else {
            continue; // no stored hash: nothing to be stale about
        };
        let path = Path::new(&row.source_path);
        let Ok(metadata) = std::fs::metadata(path) else {
            out.push(Finding {
                kind: "index-record-stale",
                selector: row.id.clone(),
                source_path: row.source_path.clone(),
                detail: format!(
                    "indexed sha256 {indexed} on record but the source file is no longer readable; stored hash is the last verified state (rerun with --rehash for full revalidation)"
                ),
            });
            continue;
        };
        let mut drift = Vec::new();
        if let Some(indexed_size) = row.size_bytes
            && metadata.len() != indexed_size
        {
            drift.push(format!(
                "size {} != indexed {}",
                metadata.len(),
                indexed_size
            ));
        }
        if let Some(indexed_mtime) = row.modified_unix
            && let Ok(live_mtime) = metadata.modified()
            && let Ok(live) = live_mtime.duration_since(std::time::UNIX_EPOCH)
            && live.as_secs() != indexed_mtime
        {
            drift.push(format!(
                "mtime {} != indexed {}",
                live.as_secs(),
                indexed_mtime
            ));
        }
        if !drift.is_empty() {
            out.push(Finding {
                kind: "index-record-stale",
                selector: row.id.clone(),
                source_path: row.source_path.clone(),
                detail: format!(
                    "{}; stored sha256 {indexed} may no longer describe the file (rerun with --rehash for full revalidation)",
                    drift.join("; ")
                ),
            });
        }
    }
    out
}

fn timestamp_findings(rows: &[IndexedRow]) -> Vec<Finding> {
    let mut by_parent: std::collections::BTreeMap<String, Vec<&IndexedRow>> =
        std::collections::BTreeMap::new();
    for row in rows {
        if row.modified_unix.is_none() {
            continue;
        }
        let parent = Path::new(&row.source_path)
            .parent()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default();
        by_parent.entry(parent).or_default().push(row);
    }

    let mut out = Vec::new();
    for (_parent, mut group) in by_parent {
        group.sort_by(|a, b| a.source_path.cmp(&b.source_path));
        for window in group.windows(2) {
            let left = window[0];
            let right = window[1];
            let left_ts = left.modified_unix.unwrap_or(0);
            let right_ts = right.modified_unix.unwrap_or(0);
            if right_ts < left_ts {
                out.push(Finding {
                    kind: "timestamp-regression",
                    selector: right.id.clone(),
                    source_path: right.source_path.clone(),
                    detail: format!(
                        "path-sorted after {} but mtime {} < {}",
                        left.source_path, right_ts, left_ts
                    ),
                });
            } else if right_ts.saturating_sub(left_ts) > GAP_SECS {
                out.push(Finding {
                    kind: "timestamp-gap",
                    selector: right.id.clone(),
                    source_path: right.source_path.clone(),
                    detail: format!(
                        "mtime gap {}s from {} (threshold {}s)",
                        right_ts - left_ts,
                        left.source_path,
                        GAP_SECS
                    ),
                });
            }
        }
    }
    out
}

fn container_stream_findings(rows: &[IndexedRow]) -> Vec<Finding> {
    let mut out = Vec::new();
    for row in rows {
        if row.ffprobe_ok != Some(true) {
            continue;
        }
        let format = row
            .format_name
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase();
        let codec = row
            .video_codec
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase();
        if (format.contains("image2") || format == "png_pipe" || format == "jpeg_pipe")
            && !codec.is_empty()
        {
            out.push(Finding {
                kind: "container-stream-mismatch",
                selector: row.id.clone(),
                source_path: row.source_path.clone(),
                detail: format!(
                    "format `{format}` looks like still image but video codec is `{codec}`"
                ),
            });
        }
        if let Some(duration) = row.duration_seconds
            && duration == 0.0
            && !codec.is_empty()
        {
            out.push(Finding {
                kind: "container-stream-mismatch",
                selector: row.id.clone(),
                source_path: row.source_path.clone(),
                detail: format!("duration 0 with video codec `{codec}`"),
            });
        }
    }
    out
}

/// DAV frame-interval findings: consecutive video frames on the same channel
/// whose embedded recording times jump by more than `DAV_FRAME_GAP_SECS`,
/// indicating dropped frames or a recorder pause. Runs only for indexed DAV
/// sources whose files are still reachable; unreadable files are skipped.
fn dav_frame_gap_findings(case_dir: &Path, rows: &[IndexedRow]) -> Vec<Finding> {
    let _ = case_dir;
    let mut out = Vec::new();
    for row in rows {
        let path = Path::new(&row.source_path);
        if !path.is_file() {
            continue;
        }
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_none_or(|ext| !ext.eq_ignore_ascii_case("dav"))
        {
            continue;
        }
        let Ok(frames) = crate::dav::walk_frames(path) else {
            continue; // unparseable DAV stays out of anomaly scope
        };

        // Video-only, per-channel sequence with packed-date ordering.
        let video: Vec<&crate::dav::DavFrame> = frames
            .iter()
            .filter(|frame| {
                matches!(
                    frame.stream_type,
                    crate::dav::STREAM_VIDEO_P | crate::dav::STREAM_VIDEO_I
                )
            })
            .collect();
        for window in video.windows(2) {
            let (prev, next) = (window[0], window[1]);
            if prev.channel != next.channel {
                continue;
            }
            if next.date_packed() < prev.date_packed() {
                out.push(Finding {
                    kind: "dav-frame-timestamp-regression",
                    selector: row.id.clone(),
                    source_path: row.source_path.clone(),
                    detail: format!(
                        "frame {} packed date {} moves backwards after {}",
                        next.offset,
                        next.date_packed(),
                        prev.date_packed()
                    ),
                });
            } else if next.date_packed() > prev.date_packed() {
                // Coarse span from the 1-second-resolution packed dates; the
                // wrapped ms counter refines the boundary. FFmpeg get_pts
                // treats `timestamp` as a free-running 65535-wrap ms counter,
                // never as wall-clock seconds, so it must not be summed into
                // the date seconds.
                let prev_secs = packed_date_seconds(prev);
                let next_secs = packed_date_seconds(next);
                let delta = (next_secs - prev_secs) as f64
                    + (next.subsecond_ms() as f64)
                        .mul_add(0.001, -(prev.subsecond_ms() as f64) * 0.001);
                if delta > DAV_FRAME_GAP_SECS {
                    out.push(Finding {
                        kind: "dav-frame-gap",
                        selector: row.id.clone(),
                        source_path: row.source_path.clone(),
                        detail: format!(
                            "video frames at byte offsets {} -> {} span {delta:.3}s (threshold {DAV_FRAME_GAP_SECS}s); possible dropped frames or recorder pause",
                            prev.offset, next.offset
                        ),
                    });
                }
            }
        }
    }
    out
}

/// Total seconds encoded in the packed Dahua date (monotonic-in-practice
/// civil time; exact calendar math is unnecessary for gap spans).
fn packed_date_seconds(frame: &crate::dav::DavFrame) -> u64 {
    let (year, month, day, hour, minute, second) = frame.date_breakdown();
    u64::from(year) * 31536000
        + u64::from(month) * 2592000
        + u64::from(day) * 86400
        + u64::from(hour) * 3600
        + u64::from(minute) * 60
        + u64::from(second)
}

pub fn read_indexed_rows(case_dir: &Path) -> Result<Vec<IndexedRow>, String> {
    let path = case_dir.join("db/videos.jsonl");
    let text = match read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(format!(
                "failed to read indexed videos {}: {err}",
                path.display()
            ));
        }
    };
    Ok(text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .map(|value| IndexedRow {
            id: value
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            source_path: value
                .get("source_path")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            sha256: value
                .get("sha256")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            size_bytes: value.get("size_bytes").and_then(serde_json::Value::as_u64),
            modified_unix: value
                .get("modified_unix")
                .and_then(serde_json::Value::as_u64),
            duration_seconds: value
                .get("duration_seconds")
                .and_then(serde_json::Value::as_f64),
            format_name: value
                .get("format_name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            video_codec: value
                .get("video_codec")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            ffprobe_ok: value.get("ffprobe_ok").and_then(serde_json::Value::as_bool),
        })
        .filter(|row| !row.id.is_empty())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dav_frame_gap_finding_marks_large_jumps() {
        // Two DAV video frames whose embedded recording clocks are 10s apart.
        let dir =
            std::env::temp_dir().join(format!("frametrace-dav-gap-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let dav_path = dir.join("cam.dav");
        std::fs::write(&dav_path, dav_fixture_with_dates(0, 10)).unwrap();

        let rows = vec![IndexedRow {
            id: "vid_1".into(),
            source_path: dav_path.to_string_lossy().to_string(),
            sha256: None,
            size_bytes: None,
            modified_unix: None,
            duration_seconds: None,
            format_name: None,
            video_codec: None,
            ffprobe_ok: None,
        }];
        let findings = dav_frame_gap_findings(&dir, &rows);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "dav-frame-gap");
        // 10s from packed dates; the ms remainder (500-0) only refines it.
        assert!(
            findings[0].detail.contains("span 10.500s") || findings[0].detail.contains("10.5"),
            "{detail}",
            detail = findings[0].detail
        );

        // A normal 1s cadence stays clean even across the 65535 ms wrap.
        let steady_path = dir.join("steady.dav");
        std::fs::write(&steady_path, dav_fixture_with_dates(0, 1)).unwrap();
        let steady_rows = vec![IndexedRow {
            id: "vid_2".into(),
            source_path: steady_path.to_string_lossy().to_string(),
            sha256: None,
            size_bytes: None,
            modified_unix: None,
            duration_seconds: None,
            format_name: None,
            video_codec: None,
            ffprobe_ok: None,
        }];
        assert!(dav_frame_gap_findings(&dir, &steady_rows).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Builds a minimal two-video-frame DAV whose second frame's packed date
    /// is `gap_secs` after the first (same minute, so only seconds differ).
    /// The ms counter sits near its 65535 wrap so a naive seconds-style
    /// reading of it would produce a wildly wrong span.
    fn dav_fixture_with_dates(first_sec: u32, second_sec: u32) -> Vec<u8> {
        fn frame(channel: u8, second: u32, timestamp_ms: u16, payload: &[u8]) -> Vec<u8> {
            let header_len = 24u32;
            let frame_length = header_len + payload.len() as u32 + 8;
            let date = second & 0x3F; // packed: only the seconds field
            let mut bytes = Vec::with_capacity(frame_length as usize);
            bytes.extend_from_slice(b"DHAV");
            bytes.push(0xFD); // key video frame
            bytes.push(0);
            bytes.push(channel);
            bytes.push(0);
            bytes.extend_from_slice(&1u32.to_le_bytes());
            bytes.extend_from_slice(&frame_length.to_le_bytes());
            bytes.extend_from_slice(&date.to_le_bytes());
            bytes.extend_from_slice(&timestamp_ms.to_le_bytes());
            bytes.push(0); // ext_length
            bytes.push(0); // checksum
            bytes.extend_from_slice(payload);
            bytes.extend_from_slice(b"dhav");
            bytes.extend_from_slice(&frame_length.to_le_bytes());
            bytes
        }
        let mut out = Vec::new();
        out.extend(frame(1, first_sec, 65000, b"V1"));
        out.extend(frame(1, second_sec, 500, b"V2"));
        out
    }

    #[test]
    fn detects_timestamp_regression_within_folder() {
        let rows = vec![
            IndexedRow {
                id: "vid_1".into(),
                source_path: "C:/ev/a.mp4".into(),
                sha256: None,
                size_bytes: None,
                modified_unix: Some(200),
                duration_seconds: None,
                format_name: None,
                video_codec: None,
                ffprobe_ok: None,
            },
            IndexedRow {
                id: "vid_2".into(),
                source_path: "C:/ev/b.mp4".into(),
                sha256: None,
                size_bytes: None,
                modified_unix: Some(100),
                duration_seconds: None,
                format_name: None,
                video_codec: None,
                ffprobe_ok: None,
            },
        ];
        let findings = timestamp_findings(&rows);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "timestamp-regression");
        assert_eq!(findings[0].selector, "vid_2");
    }

    #[test]
    fn stored_hash_lane_flags_drift_and_missing_files_without_rehashing() {
        let dir =
            std::env::temp_dir().join(format!("frametrace-staleness-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let media = dir.join("clip.mp4");
        std::fs::write(&media, b"payload").unwrap();
        let metadata = std::fs::metadata(&media).unwrap();
        let live_mtime = metadata
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let row = |size: Option<u64>, mtime: Option<u64>, path: &Path| IndexedRow {
            id: "vid_1".into(),
            source_path: path.to_string_lossy().to_string(),
            sha256: Some("deadbeef".into()),
            size_bytes: size,
            modified_unix: mtime,
            duration_seconds: None,
            format_name: None,
            video_codec: None,
            ffprobe_ok: None,
        };

        // Recorded metadata still matches: stored hash is treated as current.
        assert!(
            index_staleness_findings(&[row(Some(metadata.len()), Some(live_mtime), &media)])
                .is_empty()
        );

        // Size drift means the stored hash may no longer describe the file.
        let drifted =
            index_staleness_findings(&[row(Some(metadata.len() + 1), Some(live_mtime), &media)]);
        assert_eq!(drifted.len(), 1);
        assert_eq!(drifted[0].kind, "index-record-stale");
        assert!(drifted[0].detail.contains("size"), "{drifted:?}");

        // A vanished source is reported from the record, not silently skipped.
        let missing = index_staleness_findings(&[row(
            Some(metadata.len()),
            Some(live_mtime),
            &dir.join("gone.mp4"),
        )]);
        assert_eq!(missing.len(), 1);
        assert!(missing[0].detail.contains("no longer readable"));

        // Rows without a stored hash stay out of the staleness lane.
        let mut unhashed = row(Some(metadata.len()), Some(live_mtime), &media);
        unhashed.sha256 = None;
        assert!(index_staleness_findings(&[unhashed]).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detects_zero_duration_with_video_codec() {
        let rows = vec![IndexedRow {
            id: "vid_1".into(),
            source_path: "C:/ev/a.mp4".into(),
            sha256: None,
            size_bytes: None,
            modified_unix: None,
            duration_seconds: Some(0.0),
            format_name: Some("mov,mp4,m4a,3gp,3g2,mj2".into()),
            video_codec: Some("h264".into()),
            ffprobe_ok: Some(true),
        }];
        let findings = container_stream_findings(&rows);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "container-stream-mismatch");
    }

    #[test]
    fn label_is_candidate_finding_not_legal_claim() {
        assert_eq!(LABEL, "candidate-finding");
        assert!(!LABEL.contains("tamper"));
        assert!(!LABEL.contains("authentic"));
    }
}
