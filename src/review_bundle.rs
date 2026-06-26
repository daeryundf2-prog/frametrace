use crate::case_db::{InventoryListQuery, InventoryRow, list_inventory};
use crate::distributable_redaction::{
    RedactionPolicy, privacy_metadata_fields, redact_json_for_distributable,
};
use crate::util::{
    compact_json_value_if_well_formed, json_escape, path_to_file_url, read_to_string,
};
use std::io::ErrorKind;
use std::path::Path;

pub const REVIEW_HTML_MAX_EMBEDDED_ROWS: usize = 500;
const INVENTORY_QUERY_CONTRACT: &str = "frametrace inventory <case_dir> --limit 500 --offset <n>";

pub fn bounded_review_index_json(case_dir: &Path) -> Result<String, String> {
    bounded_review_index_json_with_policy(case_dir, RedactionPolicy::redacted())
}

pub fn bounded_review_index_json_with_policy(
    case_dir: &Path,
    policy: RedactionPolicy,
) -> Result<String, String> {
    if case_dir.join("db/case.db").is_file() {
        return sqlite_review_index_json(case_dir, policy);
    }
    legacy_jsonl_review_index_json(case_dir, policy)
}

fn sqlite_review_index_json(case_dir: &Path, policy: RedactionPolicy) -> Result<String, String> {
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
    let embedded_rows = page
        .rows
        .iter()
        .map(|row| {
            let row_json = sqlite_row_json(row);
            redact_json_for_distributable(case_dir, &row_json, policy)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(render_index_json(
        policy,
        "case.db/videos",
        page.total_rows,
        &embedded_rows,
        warnings,
    ))
}

fn legacy_jsonl_review_index_json(
    case_dir: &Path,
    policy: RedactionPolicy,
) -> Result<String, String> {
    let jsonl_path = case_dir.join("db/videos.jsonl");
    let mut warnings = Vec::new();
    let text = match read_to_string(&jsonl_path) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            warnings.push(format!(
                "No db/videos.jsonl found; generated review embeds 0 rows. Use {INVENTORY_QUERY_CONTRACT} after scanning."
            ));
            return Ok(render_index_json(
                policy,
                "db/videos.jsonl",
                0,
                &[],
                warnings,
            ));
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
                    embedded_rows.push(redact_json_for_distributable(case_dir, &compact, policy)?);
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
        policy,
        "db/videos.jsonl",
        valid_count,
        &embedded_rows,
        warnings,
    ))
}

fn render_index_json(
    policy: RedactionPolicy,
    inventory_source: &str,
    video_count: usize,
    embedded_rows: &[String],
    warnings: Vec<String>,
) -> String {
    format!(
        "{{\"schema_version\":1,{},\"source_path\":\"{}\",\
\"inventory_source\":\"{}\",\"video_count\":{},\"embedded_video_count\":{},\
\"inventory_truncated\":{},\"inventory_limit\":{},\"inventory_query_contract\":\"{}\",\
\"warnings\":[{}],\"options\":{{\"hash_files\":false,\"use_ffprobe\":false,\"max_depth\":null}},\
\"videos\":[{}]}}",
        privacy_metadata_fields(policy),
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
        json_escape(&row.file_id),
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
    use super::{
        REVIEW_HTML_MAX_EMBEDDED_ROWS, bounded_review_index_json,
        bounded_review_index_json_with_policy,
    };
    use crate::case_db::{IndexedVideoRow, write_scan_index};
    use crate::distributable_redaction::RedactionPolicy;
    use crate::model::{ProbeSummary, ScanOptions, ScanResult, SourceProfile, VideoRecord};
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

    #[test]
    fn default_review_bundle_redacts_absolute_source_paths() {
        let case_dir = unique_temp_dir("review-bundle-redaction");
        let db_dir = case_dir.join("db");
        fs::create_dir_all(&db_dir).expect("db directory should be created");
        fs::write(
            db_dir.join("videos.jsonl"),
            r#"{"id":"vid_000001","source_path":"/tmp/Client ACME 유출/source clip.mp4","file_url":"file:///tmp/Client ACME 유출/source clip.mp4","relative_path":"Camera 01/source clip.mp4"}"#,
        )
        .expect("video jsonl should be written");

        let json = bounded_review_index_json(&case_dir).expect("bounded bundle should render");

        assert!(!json.contains("/tmp/Client ACME"));
        assert!(!json.contains("file:///tmp/Client"));
        assert!(json.contains("[redacted-source:vid_000001]"));
        assert!(json.contains("\"path_disclosure_mode\":\"redacted\""));

        let _ = fs::remove_dir_all(case_dir);
    }

    #[test]
    fn sqlite_review_bundle_redacts_absolute_source_paths_by_default() {
        let case_dir = unique_temp_dir("review-bundle-sqlite-redaction");
        fs::create_dir_all(case_dir.join("db")).expect("db directory should be created");
        let source_path = "/tmp/Client ACME SQLite 유출/source clip.mp4";
        let record = video_record("vid_sql_000001", source_path);
        let result = scan_result(vec![record.clone()]);
        write_scan_index(&case_dir, &result, &[indexed_row(&record)])
            .expect("SQLite scan index should be written");

        let json = bounded_review_index_json(&case_dir).expect("bounded bundle should render");

        assert!(json.contains("\"inventory_source\":\"case.db/videos\""));
        assert!(!json.contains("/tmp/Client ACME SQLite"));
        assert!(!json.contains("file:///tmp/Client"));
        assert!(json.contains("[redacted-source:vid_sql_000001]"));
        assert!(json.contains("\"file_url\":\"\""));
        assert!(json.contains("\"path_disclosure_mode\":\"redacted\""));

        let _ = fs::remove_dir_all(case_dir);
    }

    #[test]
    fn sqlite_review_bundle_opt_in_keeps_full_source_paths() {
        let case_dir = unique_temp_dir("review-bundle-sqlite-opt-in");
        fs::create_dir_all(case_dir.join("db")).expect("db directory should be created");
        let source_path = "/tmp/Client ACME SQLite OptIn 유출/source clip.mp4";
        let record = video_record("vid_sql_000099", source_path);
        let result = scan_result(vec![record.clone()]);
        write_scan_index(&case_dir, &result, &[indexed_row(&record)])
            .expect("SQLite scan index should be written");

        let json = bounded_review_index_json_with_policy(
            &case_dir,
            RedactionPolicy::local_operator_full_paths(),
        )
        .expect("bounded bundle should render");

        assert!(json.contains(source_path));
        assert!(json.contains("\"file_url\":\"file://"));
        assert!(json.contains("Client%20ACME%20SQLite%20OptIn"));
        assert!(json.contains("\"path_disclosure_mode\":\"local_operator_full_paths\""));
        assert!(json.contains("\"local_operator_full_path_disclosure\":true"));

        let _ = fs::remove_dir_all(case_dir);
    }

    fn video_record(id: &str, source_path: &str) -> VideoRecord {
        VideoRecord {
            id: id.to_string(),
            source_path: PathBuf::from(source_path),
            relative_path: "Camera 01/source clip.mp4".to_string(),
            extension: "mp4".to_string(),
            size_bytes: 5,
            modified_unix: Some(10),
            sha256: None,
            hash_status: "not-hashed".to_string(),
            probe: ProbeSummary::skipped(),
            confidence: "candidate".to_string(),
            source_profile: SourceProfile {
                vendor: "ACME".to_string(),
                parser: "synthetic".to_string(),
                lane: "fixture".to_string(),
                confidence: "candidate".to_string(),
                recommended_action: "review".to_string(),
                evidence: vec!["sqlite-redaction-fixture".to_string()],
            },
        }
    }

    fn indexed_row(record: &VideoRecord) -> IndexedVideoRow {
        IndexedVideoRow {
            id: record.id.clone(),
            source_path: record.source_path.to_string_lossy().to_string(),
            file_url: format!("file://{}", record.source_path.to_string_lossy()),
            relative_path: record.relative_path.to_string(),
            extension: record.extension.clone(),
            size_bytes: record.size_bytes,
            modified_unix: record.modified_unix,
            sha256: record.sha256.clone(),
            hash_status: record.hash_status.clone(),
            confidence: record.confidence.clone(),
            source_profile_json: record.source_profile.to_json(),
            duration_seconds: record.probe.duration_seconds,
            format_name: record.probe.format_name.clone(),
            video_codec: record.probe.video_codec.clone(),
            audio_codec: record.probe.audio_codec.clone(),
            width: record.probe.width.map(u64::from),
            height: record.probe.height.map(u64::from),
            ffprobe_ok: record.probe.ok,
            ffprobe_error: record.probe.error.clone(),
            ffprobe_json: record.probe.raw_json.clone(),
            record_json: record.to_json(),
        }
    }

    fn scan_result(records: Vec<VideoRecord>) -> ScanResult {
        ScanResult {
            source_path: PathBuf::from("/tmp/Client ACME SQLite 유출"),
            options: ScanOptions::default(),
            records,
            video_count: 1,
            total_bytes: 5,
            scanned_unix: 1,
            warnings: Vec::new(),
        }
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after UNIX epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("frametrace-{name}-{}-{nanos}", std::process::id()))
    }
}
