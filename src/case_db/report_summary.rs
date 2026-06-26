use super::inventory::list_inventory;
use super::inventory_query::{open_inventory_db, scalar_count};
use super::inventory_types::InventoryListQuery;
use crate::util::json_escape;
use rusqlite::OptionalExtension;
use std::path::Path;

pub const REPORT_VIDEO_SAMPLE_LIMIT: usize = 100;

pub fn bounded_report_index_json(case_dir: &Path) -> Result<Option<String>, String> {
    let Some(conn) = open_inventory_db(case_dir)? else {
        return Ok(None);
    };
    let total_bytes = scalar_count(&conn, "SELECT COALESCE(SUM(size_bytes), 0) FROM videos", [])?;
    let source_count = scalar_count(
        &conn,
        "SELECT COUNT(DISTINCT source_profile_json) FROM videos",
        [],
    )?;
    let latest_scan = latest_scan_run(&conn)?;
    let page = list_inventory(
        case_dir,
        &InventoryListQuery {
            page_size: REPORT_VIDEO_SAMPLE_LIMIT,
            sort: Some("path-asc".to_string()),
            ..InventoryListQuery::default()
        },
    )?;
    let videos = page
        .rows
        .iter()
        .map(sample_video_json)
        .collect::<Vec<_>>()
        .join(",");
    let warnings = latest_scan
        .as_ref()
        .map(|scan| scan.warnings_json.as_str())
        .unwrap_or("[]");
    let options = latest_scan
        .as_ref()
        .map(scan_options_json)
        .unwrap_or_else(|| {
            "{\"hash_files\":false,\"use_ffprobe\":false,\"max_depth\":null}".to_string()
        });
    let source_path = latest_scan
        .as_ref()
        .map(|scan| scan.source_path.as_str())
        .unwrap_or("");
    let scanned_unix = latest_scan
        .as_ref()
        .map(|scan| scan.scanned_unix)
        .unwrap_or(0);
    let confirmed_count = confirmed_count(&conn)?;
    Ok(Some(format!(
        "{{\"schema_version\":2,\"source\":\"sqlite-bounded-report\",\
\"source_path\":\"{}\",\"scanned_unix\":{},\"video_count\":{},\"total_bytes\":{},\
\"confirmed_count\":{},\"candidate_count\":{},\"source_count\":{},\"warnings\":{},\
\"options\":{},\"sample_limit\":{},\"videos_truncated\":{},\"videos\":[{}],\
\"report_summary\":{{\"bounded\":true,\"sample_count\":{},\"total_rows\":{},\
\"inventory_command\":\"frametrace inventory <case_dir> --limit 500\",\
\"review_command\":\"frametrace make-review <case_dir>\"}}}}",
        json_escape(source_path),
        scanned_unix,
        page.total_rows,
        total_bytes,
        confirmed_count,
        page.total_rows.saturating_sub(confirmed_count),
        source_count,
        warnings,
        options,
        REPORT_VIDEO_SAMPLE_LIMIT,
        page.total_rows > page.rows.len(),
        videos,
        page.rows.len(),
        page.total_rows
    )))
}

struct LatestScanRun {
    source_path: String,
    scanned_unix: usize,
    hash_files: bool,
    use_ffprobe: bool,
    max_depth: Option<usize>,
    warnings_json: String,
}

fn latest_scan_run(conn: &rusqlite::Connection) -> Result<Option<LatestScanRun>, String> {
    conn.query_row(
        "SELECT source_path, scanned_unix, hash_files, use_ffprobe, max_depth, warnings_json \
         FROM scan_runs ORDER BY scanned_unix DESC, run_pk DESC LIMIT 1",
        [],
        |row| {
            Ok(LatestScanRun {
                source_path: row.get(0)?,
                scanned_unix: nonnegative_usize(row.get::<_, i64>(1)?),
                hash_files: row.get::<_, i64>(2)? != 0,
                use_ffprobe: row.get::<_, i64>(3)? != 0,
                max_depth: row.get::<_, Option<i64>>(4)?.map(nonnegative_usize),
                warnings_json: row.get(5)?,
            })
        },
    )
    .optional()
    .map_err(|err| format!("failed to query latest SQLite scan run: {err}"))
}

fn confirmed_count(conn: &rusqlite::Connection) -> Result<usize, String> {
    scalar_count(conn, "SELECT COUNT(*) FROM videos WHERE ffprobe_ok = 1", [])
}

fn scan_options_json(scan: &LatestScanRun) -> String {
    let max_depth = scan
        .max_depth
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string());
    format!(
        "{{\"hash_files\":{},\"use_ffprobe\":{},\"max_depth\":{max_depth}}}",
        scan.hash_files, scan.use_ffprobe
    )
}

fn sample_video_json(row: &super::InventoryRow) -> String {
    let sha256 = row
        .sha256
        .as_deref()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .unwrap_or_else(|| "null".to_string());
    format!(
        "{{\"id\":\"{}\",\"source_path\":\"{}\",\"relative_path\":\"{}\",\
\"extension\":\"{}\",\"size_bytes\":{},\"modified_unix\":{},\"sha256\":{},\
\"hash_status\":\"{}\",\"ffprobe_ok\":{},\"source_profile\":{{\"vendor\":\"{}\",\
\"parser\":\"{}\",\"lane\":\"{}\",\"confidence\":\"bounded-summary\",\
\"recommended_action\":\"Open bounded inventory or review commands for row-level triage.\"}}}}",
        json_escape(&row.file_id),
        json_escape(&row.full_path),
        json_escape(&row.relative_path),
        json_escape(&row.extension),
        row.size_bytes,
        row.timestamp_start
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string()),
        sha256,
        json_escape(&row.hash_state),
        row.validation_state == "ffprobe-video-stream-confirmed",
        "SQLite bounded inventory",
        json_escape(&row.parser_lane),
        json_escape(&row.parser_lane)
    )
}

fn nonnegative_usize(value: i64) -> usize {
    value.max(0) as usize
}
