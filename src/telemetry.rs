//! Dashcam telemetry extraction (GPS track / G-sensor side data).
//!
//! Honest scope: NMEA sentences are decoded fully — whether they ride a
//! dedicated `gps`/`text` data stream (common in Thinkware/FineVi/BlackVue
//! MP4s) or sit as free-box strings inside the file. Binary telemetry
//! tracks (camm IMU boxes, vendor-proprietary `bin_data` streams) are
//! detected and reported as present-but-unparsed rather than guessed at,
//! because no real-recorder sample has validated a decoder layout here.
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

#[derive(Debug, Serialize)]
pub struct TelemetryReport {
    pub schema_version: u32,
    pub event: &'static str,
    pub extracted_unix: u64,
    pub selector: String,
    pub source_path: String,
    /// "nmea-data-stream" | "nmea-file-scan" | "none"
    pub source: String,
    pub points: Vec<TelemetryPoint>,
    /// Telemetry-looking streams we detected but cannot decode honestly
    /// (binary camm/IMU tracks, proprietary vendor payloads).
    pub unparsed_streams: Vec<String>,
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
    let kind = &f[0][3..];
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
    let text = String::from_utf8_lossy(bytes);
    let mut rest: &str = &text;
    while let Some(pos) = rest.find("$G") {
        if points.len() >= MAX_POINTS {
            return;
        }
        let tail = &rest[pos..];
        let end = tail.find(['\n', '\r']).unwrap_or(tail.len());
        let line = &tail[..end.min(200)];
        if line.len() > 10
            && line
                .get(3..)
                .is_some_and(|rest| rest.starts_with("RMC,") || rest.starts_with("GGA,"))
            && let Some(p) = parse_nmea(line)
        {
            points.push(p);
        }
        rest = &rest[pos + 2..];
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

/// Data/other stream indexes from ffprobe, plus codec names — camm and
/// vendor payloads are flagged unparsed, not decoded.
fn telemetry_streams(path: &Path) -> (Vec<u32>, Vec<String>) {
    let probe = crate::ffprobe::probe(path);
    let mut indexes = Vec::new();
    let mut unparsed = Vec::new();
    if let Some(raw) = probe.raw_json
        && let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw)
        && let Some(streams) = json.get("streams").and_then(|s| s.as_array())
    {
        for st in streams {
            let idx = st.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as u32;
            let codec_type = st.get("codec_type").and_then(|c| c.as_str()).unwrap_or("");
            let codec_name = st.get("codec_name").and_then(|c| c.as_str()).unwrap_or("");
            match codec_name {
                "camm" => unparsed.push(format!(
                    "stream #{idx} camm — binary Camera Motion metadata (GPS+IMU) present; decoder not validated against real samples"
                )),
                _ if codec_type == "data" || codec_type == "other" => indexes.push(idx),
                _ => {}
            }
        }
    }
    (indexes, unparsed)
}

/// Extracts a telemetry report for one video file.
pub fn extract_file(path: &Path) -> Result<TelemetryReport, String> {
    let (stream_indexes, mut unparsed) = telemetry_streams(path);
    let mut points = Vec::new();
    let mut source = "none".to_string();

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
            Ok(bytes) => scan_nmea_bytes(&bytes, &mut points),
            Err(_) => unparsed.push(format!("stream #{idx} — payload dump failed")),
        }
    }
    if !points.is_empty() {
        source = "nmea-data-stream".to_string();
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
            source = "nmea-file-scan".to_string();
        }
    }

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
        schema_version: 1,
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
        note: if source == "none" {
            "No NMEA telemetry found. Dashcams that record GPS only in binary/proprietary tracks report them as unparsed streams — absence here does not prove absence in the original.".to_string()
        } else {
            "NMEA-derived track points; timestamps are UTC per the RMC date field. Verify against recorder vendor tools before reporting positions as fact.".to_string()
        },
    })
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
}
