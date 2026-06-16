use crate::case_db::{InventoryListQuery, InventoryRow, list_inventory};
use crate::util::{
    compact_json_value_if_well_formed, json_escape, path_to_file_url, read_to_string,
};
use std::io::ErrorKind;
use std::path::Path;

pub const REVIEW_HTML_MAX_EMBEDDED_ROWS: usize = 500;
const INVENTORY_QUERY_CONTRACT: &str = "frametrace inventory <case_dir> --limit 500 --offset <n>";

pub fn bounded_review_index_json(case_dir: &Path) -> Result<String, String> {
    if case_dir.join("db/case.db").is_file() {
        return sqlite_review_index_json(case_dir);
    }
    legacy_jsonl_review_index_json(case_dir)
}

fn sqlite_review_index_json(case_dir: &Path) -> Result<String, String> {
    let page = list_inventory(
        case_dir,
        &InventoryListQuery {
            page_size: REVIEW_HTML_MAX_EMBEDDED_ROWS,
            ..InventoryListQuery::default()
        },
    )?;
    let mut warnings = Vec::new();
    if page.total_rows > page.rows.len() {
        warnings.push(format!(
            "Review HTML embeds {} of {} rows; use {INVENTORY_QUERY_CONTRACT} for paged SQLite-backed inventory.",
            page.rows.len(),
            page.total_rows
        ));
    }
    let embedded_rows = page.rows.iter().map(sqlite_row_json).collect::<Vec<_>>();
    Ok(render_index_json(
        "case.db/videos",
        page.total_rows,
        &embedded_rows,
        warnings,
    ))
}

fn legacy_jsonl_review_index_json(case_dir: &Path) -> Result<String, String> {
    let jsonl_path = case_dir.join("db/videos.jsonl");
    let mut warnings = Vec::new();
    let text = match read_to_string(&jsonl_path) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            warnings.push(format!(
                "No db/videos.jsonl found; generated review embeds 0 rows. Use {INVENTORY_QUERY_CONTRACT} after scanning."
            ));
            return Ok(render_index_json("db/videos.jsonl", 0, &[], warnings));
        }
        Err(err) => {
            return Err(format!("failed to read {}: {err}", jsonl_path.display()));
        }
    };

    let mut valid_count = 0usize;
    let mut malformed_count = 0usize;
    let mut embedded_rows = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        match compact_json_value_if_well_formed(line) {
            Some(compact) if compact.starts_with('{') => {
                valid_count += 1;
                if embedded_rows.len() < REVIEW_HTML_MAX_EMBEDDED_ROWS {
                    embedded_rows.push(compact);
                }
            }
            _ => malformed_count += 1,
        }
    }

    if valid_count > embedded_rows.len() {
        warnings.push(format!(
            "Review HTML embeds {} of {} rows; use {INVENTORY_QUERY_CONTRACT} for paged SQLite-backed inventory.",
            embedded_rows.len(),
            valid_count
        ));
    }
    if malformed_count > 0 {
        warnings.push(format!(
            "Skipped {} malformed videos.jsonl {} while building bounded review inventory.",
            malformed_count,
            if malformed_count == 1 { "row" } else { "rows" }
        ));
    }

    Ok(render_index_json(
        "db/videos.jsonl",
        valid_count,
        &embedded_rows,
        warnings,
    ))
}

fn render_index_json(
    inventory_source: &str,
    video_count: usize,
    embedded_rows: &[String],
    warnings: Vec<String>,
) -> String {
    format!(
        "{{\"schema_version\":1,\"source_path\":\"{}\",\
\"inventory_source\":\"{}\",\"video_count\":{},\"embedded_video_count\":{},\
\"inventory_truncated\":{},\"inventory_limit\":{},\"inventory_query_contract\":\"{}\",\
\"warnings\":[{}],\"options\":{{\"hash_files\":false,\"use_ffprobe\":false,\"max_depth\":null}},\
\"videos\":[{}]}}",
        "SQLite-backed bounded review inventory",
        json_escape(inventory_source),
        video_count,
        embedded_rows.len(),
        if video_count > embedded_rows.len() {
            "true"
        } else {
            "false"
        },
        REVIEW_HTML_MAX_EMBEDDED_ROWS,
        json_escape(INVENTORY_QUERY_CONTRACT),
        warnings
            .iter()
            .map(|warning| format!("\"{}\"", json_escape(warning)))
            .collect::<Vec<_>>()
            .join(","),
        embedded_rows.join(",")
    )
}

fn sqlite_row_json(row: &InventoryRow) -> String {
    let file_url = path_to_file_url(Path::new(&row.full_path));
    format!(
        "{{\"id\":\"{}\",\"source_path\":\"{}\",\"file_url\":\"{}\",\
\"relative_path\":\"{}\",\"extension\":\"{}\",\"size_bytes\":{},\"modified_unix\":{},\
\"sha256\":{},\"hash_status\":\"{}\",\"confidence\":\"{}\",\"ffprobe_ok\":{},\
\"source_profile\":{{\"vendor\":\"{}\",\"parser\":\"{}\",\"lane\":\"{}\",\
\"confidence\":\"{}\",\"recommended_action\":\"{}\"}}}}",
        json_escape(&row.file_id),
        json_escape(&row.full_path),
        json_escape(&file_url),
        json_escape(&row.relative_path),
        json_escape(&row.extension),
        row.size_bytes,
        optional_u64(row.timestamp_start),
        optional_json_string(row.sha256.as_deref()),
        json_escape(&row.hash_state),
        json_escape(&row.validation_state),
        if row.validation_state == "ffprobe-video-stream-confirmed" {
            "true"
        } else {
            "false"
        },
        json_escape(&row.source_label),
        json_escape(&row.parser_lane),
        json_escape(&row.parser_lane),
        json_escape(&row.validation_state),
        json_escape(&row.review_state)
    )
}

fn optional_json_string(value: Option<&str>) -> String {
    value
        .map(|inner| format!("\"{}\"", json_escape(inner)))
        .unwrap_or_else(|| "null".to_string())
}

fn optional_u64(value: Option<u64>) -> String {
    value
        .map(|inner| inner.to_string())
        .unwrap_or_else(|| "null".to_string())
}

#[cfg(test)]
mod tests {
    use super::{REVIEW_HTML_MAX_EMBEDDED_ROWS, bounded_review_index_json};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn bounds_embedded_review_inventory_and_discloses_truncation() {
        let case_dir = unique_temp_dir("review-bundle-bounded");
        let db_dir = case_dir.join("db");
        fs::create_dir_all(&db_dir).expect("db directory should be created");
        let rows = (0..(REVIEW_HTML_MAX_EMBEDDED_ROWS + 2))
            .map(|index| {
                format!(
                    r#"{{"id":"vid_{index:06}","source_path":"/evidence/{index}.mp4","relative_path":"{index}.mp4"}}"#
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(db_dir.join("videos.jsonl"), rows).expect("video jsonl should be written");

        let json = bounded_review_index_json(&case_dir).expect("bounded bundle should render");

        assert!(json.contains("\"schema_version\":1"));
        assert!(json.contains("\"video_count\":502"));
        assert!(json.contains("\"embedded_video_count\":500"));
        assert!(json.contains("\"inventory_truncated\":true"));
        assert!(json.contains("\"inventory_limit\":500"));
        assert!(json.contains("frametrace inventory <case_dir> --limit 500 --offset <n>"));
        assert!(json.contains("Review HTML embeds 500 of 502 rows"));
        assert!(json.contains("\"id\":\"vid_000000\""));
        assert!(json.contains("\"id\":\"vid_000499\""));
        assert!(!json.contains("\"id\":\"vid_000500\""));

        let _ = fs::remove_dir_all(case_dir);
    }

    #[test]
    fn malformed_rows_are_excluded_and_disclosed() {
        let case_dir = unique_temp_dir("review-bundle-malformed");
        let db_dir = case_dir.join("db");
        fs::create_dir_all(&db_dir).expect("db directory should be created");
        fs::write(
            db_dir.join("videos.jsonl"),
            "{\"id\":\"vid_000001\"}\nnot-json\n{\"id\":\"vid_000002\"}\n",
        )
        .expect("video jsonl should be written");

        let json = bounded_review_index_json(&case_dir).expect("bounded bundle should render");

        assert!(json.contains("\"video_count\":2"));
        assert!(json.contains("\"embedded_video_count\":2"));
        assert!(json.contains("\"inventory_truncated\":false"));
        assert!(json.contains("Skipped 1 malformed videos.jsonl row"));
        assert!(json.contains("\"id\":\"vid_000001\""));
        assert!(json.contains("\"id\":\"vid_000002\""));
        assert!(!json.contains("not-json"));

        let _ = fs::remove_dir_all(case_dir);
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after UNIX epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("frametrace-{name}-{}-{nanos}", std::process::id()))
    }
}
