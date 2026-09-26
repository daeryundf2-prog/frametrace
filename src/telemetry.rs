//! Dashcam telemetry extraction (GPS track / G-sensor side data).
//!
//! Honest scope: NMEA sentences are decoded fully — whether they ride a
//! dedicated `gps`/`text` data stream (common in Thinkware/FineVi/BlackVue
//! MP4s) or sit as free-box strings inside the file. camm (Camera Motion
//! Metadata) tracks are decoded per the public spec — implemented and
//! unit-tested, but flagged as not-yet-validated against a real recorder
//! sample. Vendor-proprietary `bin_data` streams are still reported as
//! present-but-unparsed rather than guessed at.
//!
//! Timestamps come from NMEA RMC date+time fields and are treated as UTC —
//! dashcams that log local time still produce a consistent, documented
//! timeline rather than a silently shifted one.

use crate::tool_policy::resolve_tool_binary;
use crate::util::run_with_timeout;
use crate::video_export::{resolve_video_source, sanitize_filename};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const EDGE_SCAN_BYTES: usize = 4 * 1024 * 1024;
const MAX_POINTS: usize = 20_000;
const TOOL_TIMEOUT: u64 = 60;

#[derive(Debug, Clone, Serialize)]
pub struct TelemetryPoint {
    pub lat: f64,
    pub lon: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts_unix: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_kmh: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_m: Option<f64>,
}

/// Per-case counts from a camm (Camera Motion Metadata) stream — kept
/// alongside the decoded points so the examiner sees how much of the
/// stream the spec-based parser covered versus skipped.
#[derive(Debug, Serialize)]
pub struct CammStats {
    pub packets: usize,
    /// case 5 lat/lon + case 7 GPS packets that yielded a point.
    pub gps_points: usize,
    /// case 7 packets skipped because gps_fix_type was 0 (no fix).
    pub gps_no_fix: usize,
    /// cases 0/2/3/6 (angle-axis/gyro/acceleration/orientation) counted
    /// but not surfaced as track points.
    pub imu_packets: usize,
    pub unknown_packets: usize,
}

#[derive(Debug, Serialize)]
pub struct TelemetryReport {
    pub schema_version: u32,
    pub event: &'static str,
    pub extracted_unix: u64,
    pub selector: String,
    pub source_path: String,
    /// "nmea-data-stream" | "nmea-file-scan" | "camm-gps" | combos | "none"
    pub source: String,
    pub points: Vec<TelemetryPoint>,
    /// Telemetry-looking streams we detected but cannot decode honestly
    /// (vendor-proprietary `bin_data` payloads, failed dumps).
    pub unparsed_streams: Vec<String>,
    /// Present when a camm data stream was found and decoded per the
    /// public Camera Motion Metadata spec — the layout is implemented
    /// and unit-tested, but no real recorder sample has validated it
    /// here, so points carry the caveat in `note`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camm: Option<CammStats>,
    pub point_count: usize,
    pub first_ts_unix: Option<f64>,
    pub last_ts_unix: Option<f64>,
    pub max_speed_kmh: Option<f64>,
    pub note: String,
}

/// Parses one NMEA sentence (RMC or GGA, any `$G?` talker). Returns a
/// point only when a valid lat/lon pair is present.
fn parse_nmea(sentence: &str) -> Option<TelemetryPoint> {
    let s = sentence.trim();
    if s.len() < 10 || !s.starts_with("$G") {
        return None;
    }
    let s = s.split('*').next().unwrap_or(s);
    let f: Vec<&str> = s.split(',').collect();
    let kind = f[0].get(3..)?;
    match kind {
        "RMC" => {
            if f.len() < 10 || f[2] != "A" {
                return None;
            }
            let lat = parse_coord(f[3], f[4])?;
            let lon = parse_coord(f[5], f[6])?;
            let speed = f[7].parse::<f64>().ok().map(|k| k * 1.852);
            let ts = rmc_unix(f[1], f[9]);
            Some(TelemetryPoint {
                lat,
                lon,
                ts_unix: ts,
                speed_kmh: speed,
                alt_m: None,
            })
        }
        "GGA" => {
            if f.len() < 10 {
                return None;
            }
            let lat = parse_coord(f[2], f[3])?;
            let lon = parse_coord(f[4], f[5])?;
            let alt = f[9].parse::<f64>().ok();
            Some(TelemetryPoint {
                lat,
                lon,
                ts_unix: None,
                speed_kmh: None,
                alt_m: alt,
            })
        }
        _ => None,
    }
}

/// ddmm.mmmm / dddmm.mmmm + hemisphere → signed decimal degrees.
fn parse_coord(raw: &str, hemi: &str) -> Option<f64> {
    let v: f64 = raw.parse().ok()?;
    if !(0.0..18000.0).contains(&v) {
        return None;
    }
    let deg = (v / 100.0).floor();
    let min = v - deg * 100.0;
    if min >= 60.0 {
        return None;
    }
    let mut out = deg + min / 60.0;
    if hemi == "S" || hemi == "W" {
        out = -out;
    }
    if !(-180.0..=180.0).contains(&out) {
        return None;
    }
    Some(out)
}

/// hhmmss.ss + ddmmyy → unix epoch (UTC assumption, documented).
fn rmc_unix(time: &str, date: &str) -> Option<f64> {
    let t: f64 = time.parse().ok()?;
    let d: i64 = date.parse().ok()?;
    let (sec, min, hour) = (
        t % 100.0,
        ((t / 100.0) as i64) % 100,
        ((t / 10000.0) as i64) % 100,
    );
    let (day, mon, mut yr) = (d / 10000, (d / 100) % 100, d % 100);
    yr += if yr < 80 { 2000 } else { 1900 };
    // days-from-civil (Howard Hinnant) — no external date crate needed.
    let y = if mon <= 2 { yr - 1 } else { yr };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (mon + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    if hour > 23 || min > 59 || sec >= 60.0 || !(1..=31).contains(&day) || !(1..=12).contains(&mon)
    {
        return None;
    }
    Some(days as f64 * 86400.0 + hour as f64 * 3600.0 + min as f64 * 60.0 + sec)
}

/// Scans raw bytes for NMEA sentences ($G?RMC / $G?GGA). Sentences are
/// located anywhere in the byte stream — dashcams often embed them
/// mid-record between binary fields, so no line-start alignment is
/// assumed; each candidate is bounded by the next CR/LF or 200 bytes.
fn scan_nmea_bytes(bytes: &[u8], points: &mut Vec<TelemetryPoint>) {
    let mut pos = 0usize;
    while pos + 1 < bytes.len() {
        if points.len() >= MAX_POINTS {
            return;
        }
        let Some(off) = bytes[pos..].windows(2).position(|w| w == b"$G") else {
            return;
        };
        let start = pos + off;
        let hard_end = (start + 200).min(bytes.len());
        let end = bytes[start..hard_end]
            .iter()
            .position(|b| *b == b'\n' || *b == b'\r')
            .map(|e| start + e)
            .unwrap_or(hard_end);
        let line = String::from_utf8_lossy(&bytes[start..end]);
        if line.len() > 10
            && line
                .get(3..)
                .is_some_and(|rest| rest.starts_with("RMC,") || rest.starts_with("GGA,"))
            && let Some(p) = parse_nmea(&line)
        {
            points.push(p);
        }
        pos = start + 2;
    }
}

fn run_tool(binary: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    let bin = resolve_tool_binary(binary, &[binary])?;
    let mut cmd = Command::new(&bin);
    cmd.args(args);
    let out = run_with_timeout(&mut cmd, Some(TOOL_TIMEOUT))?;
    if !out.status.success() {
        return Err(format!(
            "{binary} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(out.stdout)
}

/// Data/other stream indexes from ffprobe, split by payload kind:
/// `nmea` = generic data/other streams worth a free-scan, `camm` =
/// Camera Motion Metadata tracks decoded per the public spec.
fn telemetry_streams(path: &Path) -> (Vec<u32>, Vec<u32>) {
    let probe = crate::ffprobe::probe(path);
    let mut nmea = Vec::new();
    let mut camm = Vec::new();
    if let Some(raw) = probe.raw_json
        && let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw)
        && let Some(streams) = json.get("streams").and_then(|s| s.as_array())
    {
        for st in streams {
            let idx = st.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as u32;
            let codec_type = st.get("codec_type").and_then(|c| c.as_str()).unwrap_or("");
            let codec_name = st.get("codec_name").and_then(|c| c.as_str()).unwrap_or("");
            // Real camm tracks surface as codec_name=bin_data with
            // codec_tag_string=camm (the mov demuxer maps the 4CC), so
            // the tag — not the codec name — is the reliable signal.
            let tag = st
                .get("codec_tag_string")
                .and_then(|c| c.as_str())
                .unwrap_or("");
            if codec_name == "camm" || tag == "camm" {
                camm.push(idx);
            } else if codec_type == "data" || codec_type == "other" {
                nmea.push(idx);
            }
        }
    }
    (nmea, camm)
}

/// Seconds between the GPS epoch (1980-01-06T00:00:00Z) and the Unix
/// epoch. camm case-7 `time_gps_epoch` counts GPS seconds; the ~18s of
/// accumulated leap seconds are not corrected here — flagged in `note`.
const GPS_EPOCH_OFFSET: f64 = 315_964_800.0;

fn le_f32(b: &[u8], off: usize) -> f64 {
    f32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]]) as f64
}

fn le_f64(b: &[u8], off: usize) -> f64 {
    f64::from_le_bytes([
        b[off],
        b[off + 1],
        b[off + 2],
        b[off + 3],
        b[off + 4],
        b[off + 5],
        b[off + 6],
        b[off + 7],
    ])
}

fn le_i32(b: &[u8], off: usize) -> i32 {
    i32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

/// Parses one camm sample (4-byte header + type-specific payload).
/// Cases 5 and 7 produce track points; IMU cases only bump counters —
/// a packet shorter than the spec minimum is counted unknown rather
/// than mis-parsed.
fn parse_camm_packet(packet: &[u8], points: &mut Vec<TelemetryPoint>, stats: &mut CammStats) {
    if packet.len() < 4 {
        stats.unknown_packets += 1;
        return;
    }
    let ty = packet[2];
    match ty {
        5 if packet.len() >= 20 => {
            let (lat, lon) = (le_f64(packet, 4), le_f64(packet, 12));
            if lat.abs() <= 90.0 && lon.abs() <= 180.0 {
                points.push(TelemetryPoint {
                    lat,
                    lon,
                    ts_unix: None,
                    speed_kmh: None,
                    alt_m: None,
                });
                stats.gps_points += 1;
            } else {
                stats.unknown_packets += 1;
            }
        }
        7 if packet.len() >= 56 => {
            let fix = le_i32(packet, 8);
            if fix == 0 {
                stats.gps_no_fix += 1;
                return;
            }
            let (lat, lon) = (le_f64(packet, 12), le_f64(packet, 20));
            if lat.abs() > 90.0 || lon.abs() > 180.0 {
                stats.unknown_packets += 1;
                return;
            }
            // Layout: 4 time_gps_epoch(f32) · 8 fix(i32) · 12 lat(f64) ·
            // 20 lon(f64) · 28 alt(f32) · 32 h_acc · 36 v_acc · 40 vel_e
            // · 44 vel_n · 48 vel_up · 52 speed_acc — all little-endian.
            let speed_ms = (le_f32(packet, 40).powi(2)
                + le_f32(packet, 44).powi(2)
                + le_f32(packet, 48).powi(2))
            .sqrt();
            points.push(TelemetryPoint {
                lat,
                lon,
                ts_unix: Some(le_f32(packet, 4) + GPS_EPOCH_OFFSET),
                speed_kmh: Some(speed_ms * 3.6),
                alt_m: Some(le_f32(packet, 28)),
            });
            stats.gps_points += 1;
        }
        0..=4 | 6 => {
            stats.imu_packets += 1;
        }
        _ => {
            stats.unknown_packets += 1;
        }
    }
}

/// Packet payload sizes for one stream, from ffprobe -show_packets —
/// `-f data` dumps concatenate payloads with no delimiters, so sample
/// boundaries must come from the demuxer, not the byte stream itself.
fn packet_sizes(path: &Path, stream_index: u32) -> Result<Vec<usize>, String> {
    let idx = stream_index.to_string();
    let bytes = run_tool(
        "ffprobe",
        &[
            "-v",
            "error",
            "-select_streams",
            &idx,
            "-show_packets",
            "-of",
            "json",
            &path.to_string_lossy(),
        ],
    )?;
    let json: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|err| format!("failed to parse ffprobe packets json: {err}"))?;
    Ok(json
        .get("packets")
        .and_then(|p| p.as_array())
        .map(|packets| {
            packets
                .iter()
                .filter_map(|p| {
                    p.get("size").and_then(|s| {
                        s.as_str()
                            .and_then(|v| v.parse::<usize>().ok())
                            .or_else(|| s.as_u64().map(|v| v as usize))
                    })
                })
                .collect()
        })
        .unwrap_or_default())
}

/// Decodes one camm data stream into track points + coverage stats.
/// Packet boundaries come from ffprobe; each declared-size packet is
/// then parsed per the spec. A short dump or a packet smaller than the
/// spec minimum ends the walk — what was decoded stays decoded.
fn extract_camm_stream(
    path: &Path,
    stream_index: u32,
    points: &mut Vec<TelemetryPoint>,
) -> Result<CammStats, String> {
    let sizes = packet_sizes(path, stream_index)?;
    if sizes.is_empty() {
        return Err(format!("stream #{stream_index} — no packets reported"));
    }
    let idx = stream_index.to_string();
    let dump = run_tool(
        "ffmpeg",
        &[
            "-v",
            "error",
            "-i",
            &path.to_string_lossy(),
            "-map",
            &format!("0:{idx}"),
            "-c",
            "copy",
            "-f",
            "data",
            "-",
        ],
    )?;
    let mut stats = CammStats {
        packets: 0,
        gps_points: 0,
        gps_no_fix: 0,
        imu_packets: 0,
        unknown_packets: 0,
    };
    parse_camm_dump(&dump, &sizes, points, &mut stats);
    Ok(stats)
}

/// Walks a concatenated `-f data` dump using the demuxer's packet
/// sizes — the only honest boundary source, since the payloads carry
/// no length prefix. A dump shorter than the declared sizes marks the
/// remainder unknown and stops; a trailing tail beyond the last
/// declared packet is ignored (muxer padding).
fn parse_camm_dump(
    dump: &[u8],
    sizes: &[usize],
    points: &mut Vec<TelemetryPoint>,
    stats: &mut CammStats,
) {
    let mut cursor = 0usize;
    for size in sizes {
        if cursor + size > dump.len() {
            stats.unknown_packets += 1;
            return;
        }
        stats.packets += 1;
        parse_camm_packet(&dump[cursor..cursor + size], points, stats);
        cursor += size;
        if points.len() >= MAX_POINTS {
            return;
        }
    }
}

/// Extracts a telemetry report for one video file.
pub fn extract_file(path: &Path) -> Result<TelemetryReport, String> {
    let (stream_indexes, camm_indexes) = telemetry_streams(path);
    let mut unparsed = Vec::new();
    let mut points = Vec::new();
    let mut sources: Vec<&str> = Vec::new();
    let mut camm_stats: Option<CammStats> = None;

    // Pass 1: dedicated data streams (dashcam NMEA subtitle/data tracks).
    for idx in &stream_indexes {
        let idx = idx.to_string();
        match run_tool(
            "ffmpeg",
            &[
                "-v",
                "error",
                "-i",
                &path.to_string_lossy(),
                "-map",
                &format!("0:{idx}"),
                "-c",
                "copy",
                "-f",
                "data",
                "-",
            ],
        ) {
            Ok(bytes) => {
                let before = points.len();
                scan_nmea_bytes(&bytes, &mut points);
                if points.len() > before && !sources.contains(&"nmea-data-stream") {
                    sources.push("nmea-data-stream");
                }
            }
            Err(_) => unparsed.push(format!("stream #{idx} — payload dump failed")),
        }
    }

    // camm tracks decode per the public Camera Motion Metadata spec —
    // implemented and unit-tested, but no real recorder sample has
    // validated the layout, so stats + the note carry that caveat.
    for idx in &camm_indexes {
        match extract_camm_stream(path, *idx, &mut points) {
            Ok(stats) => {
                if stats.gps_points > 0 && !sources.contains(&"camm-gps") {
                    sources.push("camm-gps");
                }
                match &mut camm_stats {
                    Some(agg) => {
                        agg.packets += stats.packets;
                        agg.gps_points += stats.gps_points;
                        agg.gps_no_fix += stats.gps_no_fix;
                        agg.imu_packets += stats.imu_packets;
                        agg.unknown_packets += stats.unknown_packets;
                    }
                    None => camm_stats = Some(stats),
                }
            }
            Err(err) => unparsed.push(format!("stream #{idx} camm — {err}")),
        }
    }

    // Pass 2 (fallback): free-box / in-mdat NMEA strings — some recorders
    // embed sentences without declaring a data stream at all. Only the
    // head and tail are read: a multi-GB evidence file never enters
    // memory whole.
    if points.is_empty() {
        use std::io::{Read, Seek, SeekFrom};
        if let Ok(mut f) = fs::File::open(path) {
            let mut head = vec![0u8; EDGE_SCAN_BYTES];
            let n = f.read(&mut head).unwrap_or(0);
            scan_nmea_bytes(&head[..n], &mut points);
            if points.is_empty() {
                let mut tail = vec![0u8; EDGE_SCAN_BYTES];
                if f.seek(SeekFrom::End(-(EDGE_SCAN_BYTES as i64))).is_ok()
                    && f.read_exact(&mut tail).is_ok()
                {
                    scan_nmea_bytes(&tail, &mut points);
                }
            }
        }
        if !points.is_empty() {
            sources.push("nmea-file-scan");
        }
    }
    let source = if sources.is_empty() {
        "none".to_string()
    } else {
        sources.join("+")
    };
    let has_camm = camm_stats.is_some();

    // Sort by timestamp where present; keep stream order otherwise.
    points.sort_by(|a, b| {
        a.ts_unix
            .partial_cmp(&b.ts_unix)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let tss: Vec<f64> = points.iter().filter_map(|p| p.ts_unix).collect();
    let max_speed = points
        .iter()
        .filter_map(|p| p.speed_kmh)
        .fold(None, |acc: Option<f64>, s| {
            Some(acc.map_or(s, |m| m.max(s)))
        });

    Ok(TelemetryReport {
        schema_version: 2,
        event: "telemetry-extract",
        extracted_unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        selector: String::new(),
        source_path: crate::util::canonicalize_display(path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string()),
        source: source.clone(),
        point_count: points.len(),
        first_ts_unix: tss.first().copied(),
        last_ts_unix: tss.last().copied(),
        max_speed_kmh: max_speed,
        points,
        unparsed_streams: unparsed,
        camm: camm_stats,
        note: build_note(&source, has_camm),
    })
}

fn build_note(source: &str, has_camm: bool) -> String {
    let mut parts = Vec::new();
    if source == "none" {
        parts.push(
            "No NMEA/camm telemetry found. Dashcams that record GPS only in proprietary tracks report them as unparsed streams — absence here does not prove absence in the original."
                .to_string(),
        );
    }
    if source.contains("nmea") {
        parts.push(
            "NMEA-derived track points; timestamps are UTC per the RMC date field.".to_string(),
        );
    }
    if has_camm {
        parts.push(
            "camm GPS points decoded per the public Camera Motion Metadata spec — the layout is unit-tested but NOT yet validated against a real recorder sample; GPS-epoch timestamps ignore leap seconds (~18s). Verify against vendor tools before reporting positions as fact."
                .to_string(),
        );
    }
    parts.push(
        "Verify against recorder vendor tools before reporting positions as fact.".to_string(),
    );
    parts.join(" ")
}

/// Extract + persist `artifacts/telemetry/<sanitized-selector>.json`.
/// Returns the artifact path.
pub fn extract_telemetry(
    case_dir: &Path,
    selector: &str,
) -> Result<(PathBuf, TelemetryReport), String> {
    let source = resolve_video_source(case_dir, selector)?;
    let mut report = extract_file(&source)?;
    report.selector = selector.to_string();
    let dir = case_dir.join("artifacts/telemetry");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let out = dir.join(format!("{}.json", sanitize_filename(selector)));
    crate::util::write_text_atomic(
        &out,
        &serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok((out, report))
}

/// Artifact path for a selector, whether or not it exists yet.
pub fn artifact_path(case_dir: &Path, selector: &str) -> PathBuf {
    case_dir
        .join("artifacts/telemetry")
        .join(format!("{}.json", sanitize_filename(selector)))
}

/// Collects every telemetry artifact into the viewer data map
/// (`{ sanitized_id: report }`) — same convention as the deepfake map.
pub fn collect_reports(case_dir: &Path) -> serde_json::Value {
    let dir = case_dir.join("artifacts/telemetry");
    let mut map = serde_json::Map::new();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(_) => return serde_json::Value::Object(map),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        if json.get("event").and_then(|e| e.as_str()) != Some("telemetry-extract") {
            continue;
        }
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        map.insert(stem, json);
    }
    serde_json::Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rmc_sentence() {
        let p = parse_nmea("$GPRMC,123519.00,A,4807.038,N,01131.000,E,22.4,84.4,230394,003.1,W*6A")
            .unwrap();
        assert!((p.lat - 48.1173).abs() < 0.001);
        assert!((p.lon - 11.5167).abs() < 0.001);
        assert!((p.speed_kmh.unwrap() - 41.48).abs() < 0.1);
        // 1994-03-23 12:35:19 UTC
        assert_eq!(p.ts_unix.unwrap() as i64, 764426119);
    }

    #[test]
    fn parses_gga_with_altitude() {
        let p =
            parse_nmea("$GNGGA,025458.00,3723.2475,N,12158.3416,W,1,07,1.0,9.0,M,,M,,").unwrap();
        assert!((p.lat - 37.387458).abs() < 0.0001);
        assert!((p.lon + 121.97236).abs() < 0.0001);
        assert_eq!(p.alt_m, Some(9.0));
    }

    #[test]
    fn rejects_void_and_garbage() {
        assert!(parse_nmea("$GPRMC,,V,,,,,,,,,,N*53").is_none());
        assert!(parse_nmea("not nmea").is_none());
        assert!(parse_nmea("$GPRMC,123519,A,9999.99,N,01131.000,E,,,,").is_none());
    }

    #[test]
    fn scans_sentences_out_of_binary_bytes() {
        let mut bytes = vec![0u8; 64];
        bytes.extend_from_slice(
            b"$GNRMC,120000.00,A,3723.2000,N,12658.0000,E,30.0,0.0,010124,0,E*00\r\n",
        );
        bytes.extend_from_slice(&[0xAB; 32]);
        let mut points = Vec::new();
        scan_nmea_bytes(&bytes, &mut points);
        assert_eq!(points.len(), 1);
        assert!((points[0].lat - 37.386666).abs() < 0.001);
    }

    /// Builds a synthetic camm case-7 GPS packet per the public spec:
    /// 2-byte reserved, type byte, reserved, then LE fields.
    #[allow(clippy::too_many_arguments)]
    fn camm_case7(
        gps_epoch: f32,
        fix: i32,
        lat: f64,
        lon: f64,
        alt: f32,
        ve: f32,
        vn: f32,
        vup: f32,
    ) -> Vec<u8> {
        let mut p = vec![0u8, 0, 7, 0];
        p.extend_from_slice(&gps_epoch.to_le_bytes());
        p.extend_from_slice(&fix.to_le_bytes());
        p.extend_from_slice(&lat.to_le_bytes());
        p.extend_from_slice(&lon.to_le_bytes());
        p.extend_from_slice(&alt.to_le_bytes());
        p.extend_from_slice(&5.0f32.to_le_bytes()); // h_acc
        p.extend_from_slice(&5.0f32.to_le_bytes()); // v_acc
        p.extend_from_slice(&ve.to_le_bytes());
        p.extend_from_slice(&vn.to_le_bytes());
        p.extend_from_slice(&vup.to_le_bytes());
        p.extend_from_slice(&1.0f32.to_le_bytes()); // speed_acc
        p
    }

    #[test]
    fn parses_camm_case7_gps_fix() {
        let mut points = Vec::new();
        let mut stats = CammStats {
            packets: 0,
            gps_points: 0,
            gps_no_fix: 0,
            imu_packets: 0,
            unknown_packets: 0,
        };
        // gps_epoch 1400000000 → unix ≈ 1715964800; 3-4-5 velocity = 5 m/s.
        let packet = camm_case7(1_400_000_000.0, 3, 37.5, 127.0, 25.0, 3.0, 4.0, 0.0);
        assert_eq!(packet.len(), 56);
        parse_camm_packet(&packet, &mut points, &mut stats);
        assert_eq!(stats.gps_points, 1);
        let p = &points[0];
        assert!((p.lat - 37.5).abs() < 1e-9);
        assert!((p.lon - 127.0).abs() < 1e-9);
        assert_eq!(p.alt_m, Some(25.0));
        assert!((p.speed_kmh.unwrap() - 18.0).abs() < 0.01);
        assert!((p.ts_unix.unwrap() - (1_400_000_000.0 + 315_964_800.0)).abs() < 1.0);
    }

    #[test]
    fn skips_camm_case7_without_fix() {
        let mut points = Vec::new();
        let mut stats = CammStats {
            packets: 0,
            gps_points: 0,
            gps_no_fix: 0,
            imu_packets: 0,
            unknown_packets: 0,
        };
        let packet = camm_case7(1_400_000_000.0, 0, 37.5, 127.0, 25.0, 0.0, 0.0, 0.0);
        parse_camm_packet(&packet, &mut points, &mut stats);
        assert!(points.is_empty());
        assert_eq!(stats.gps_no_fix, 1);
    }

    #[test]
    fn parses_camm_case5_and_counts_imu() {
        let mut points = Vec::new();
        let mut stats = CammStats {
            packets: 0,
            gps_points: 0,
            gps_no_fix: 0,
            imu_packets: 0,
            unknown_packets: 0,
        };
        let mut case5 = vec![0u8, 0, 5, 0];
        case5.extend_from_slice(&37.123f64.to_le_bytes());
        case5.extend_from_slice(&127.456f64.to_le_bytes());
        parse_camm_packet(&case5, &mut points, &mut stats);
        assert_eq!(points.len(), 1);
        assert!((points[0].lat - 37.123).abs() < 1e-9);
        // IMU case (gyro) counts but yields no point.
        let mut gyro = vec![0u8, 0, 2, 0];
        gyro.extend_from_slice(&[0u8; 12]);
        parse_camm_packet(&gyro, &mut points, &mut stats);
        assert_eq!(points.len(), 1);
        assert_eq!(stats.imu_packets, 1);
    }

    #[test]
    fn camm_malformed_and_unknown_types_are_counted_not_parsed() {
        let mut points = Vec::new();
        let mut stats = CammStats {
            packets: 0,
            gps_points: 0,
            gps_no_fix: 0,
            imu_packets: 0,
            unknown_packets: 0,
        };
        parse_camm_packet(&[0u8, 0], &mut points, &mut stats); // too short
        parse_camm_packet(&[0u8, 0, 99, 0, 0, 0, 0, 0], &mut points, &mut stats); // unknown type
        let short7 = vec![0u8, 0, 7, 0, 0, 0]; // truncated case 7
        parse_camm_packet(&short7, &mut points, &mut stats);
        // out-of-range coords in a well-formed case 7 → unknown, no point
        let bad = camm_case7(1.0, 3, 95.0, 200.0, 0.0, 0.0, 0.0, 0.0);
        parse_camm_packet(&bad, &mut points, &mut stats);
        assert!(points.is_empty());
        assert_eq!(stats.unknown_packets, 4);
    }

    #[test]
    fn camm_dump_walk_uses_declared_packet_boundaries() {
        // Simulates `ffmpeg -f data` output: three packets concatenated
        // (GPS fix, gyro, no-fix) — the same shape a real camm stream
        // yields once ffprobe packet sizes are applied.
        let p1 = camm_case7(1_400_000_000.0, 3, 37.5, 127.0, 25.0, 3.0, 4.0, 0.0);
        let mut p2 = vec![0u8, 0, 2, 0];
        p2.extend_from_slice(&[0u8; 12]);
        let p3 = camm_case7(1_400_000_001.0, 0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let mut dump = p1.clone();
        dump.extend_from_slice(&p2);
        dump.extend_from_slice(&p3);
        let sizes = [p1.len(), p2.len(), p3.len()];

        let mut points = Vec::new();
        let mut stats = CammStats {
            packets: 0,
            gps_points: 0,
            gps_no_fix: 0,
            imu_packets: 0,
            unknown_packets: 0,
        };
        parse_camm_dump(&dump, &sizes, &mut points, &mut stats);
        assert_eq!(stats.packets, 3);
        assert_eq!(stats.gps_points, 1);
        assert_eq!(stats.imu_packets, 1);
        assert_eq!(stats.gps_no_fix, 1);
        assert_eq!(points.len(), 1);
        assert!((points[0].lat - 37.5).abs() < 1e-9);

        // A truncated dump marks the missing packet unknown and stops.
        let mut points2 = Vec::new();
        let mut stats2 = CammStats {
            packets: 0,
            gps_points: 0,
            gps_no_fix: 0,
            imu_packets: 0,
            unknown_packets: 0,
        };
        parse_camm_dump(&dump[..p1.len() + 4], &sizes, &mut points2, &mut stats2);
        assert_eq!(stats2.packets, 1);
        assert_eq!(stats2.unknown_packets, 1);
    }

    #[test]
    fn does_not_panic_on_binary_after_marker() {
        // Regression: a real dashcam MP4 put invalid-UTF-8 bytes right
        // after "$G"; the old str-slicing path panicked on a char
        // boundary. Byte-level scanning must tolerate any payload.
        let mut bytes = b"$G".to_vec();
        bytes.extend_from_slice(&[0xF0, 0x9F, 0x98]); // truncated 4-byte seq
        bytes.extend(std::iter::repeat_n(0x80, 210));
        let mut points = Vec::new();
        scan_nmea_bytes(&bytes, &mut points);
        assert!(points.is_empty());
    }
}
