use crate::model::{ScanResult, VideoRecord};
use crate::util::{json_escape, path_to_file_url};
use rusqlite::{Connection, OpenFlags, OptionalExtension, Transaction, params};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const SCHEMA_VERSION: &str = "1";

#[derive(Debug, Clone)]
pub struct IndexedVideoRow {
    pub id: String,
    pub source_path: String,
    pub file_url: String,
    pub relative_path: String,
    pub extension: String,
    pub size_bytes: u64,
    pub modified_unix: Option<u64>,
    pub sha256: Option<String>,
    pub hash_status: String,
    pub confidence: String,
    pub source_profile_json: String,
    pub duration_seconds: Option<f64>,
    pub format_name: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub width: Option<u64>,
    pub height: Option<u64>,
    pub ffprobe_ok: bool,
    pub ffprobe_error: Option<String>,
    pub ffprobe_json: Option<String>,
    pub record_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoIdRow {
    pub id: String,
    pub source_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseDbSummary {
    pub path: PathBuf,
    pub video_count: u64,
    pub scan_run_count: u64,
}

pub fn case_db_path(case_dir: &Path) -> PathBuf {
    case_dir.join("db/case.db")
}

pub fn write_scan_index(
    case_dir: &Path,
    result: &ScanResult,
    merged_records: &[IndexedVideoRow],
) -> Result<(), String> {
    let mut conn = open_case_db(case_dir)?;
    init_schema(&conn)?;

    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start SQLite transaction: {err}"))?;
    insert_scan_run(&tx, result)?;
    for record in merged_records {
        upsert_indexed_record(&tx, record, result.scanned_unix)?;
    }
    for record in &result.records {
        upsert_scanned_record(&tx, record, result.scanned_unix)?;
    }
    tx.commit()
        .map_err(|err| format!("failed to commit SQLite index: {err}"))?;
    Ok(())
}

pub fn load_video_ids(case_dir: &Path) -> Result<Vec<VideoIdRow>, String> {
    let path = case_db_path(case_dir);
    if !path.is_file() {
        return Ok(Vec::new());
    }

    let conn = open_readonly_case_db(&path)?;
    if !table_exists(&conn, "videos")? {
        return Ok(Vec::new());
    }

    let mut stmt = conn
        .prepare("SELECT id, source_path FROM videos ORDER BY id")
        .map_err(|err| format!("failed to prepare SQLite video id query: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(VideoIdRow {
                id: row.get(0)?,
                source_path: row.get(1)?,
            })
        })
        .map_err(|err| format!("failed to query SQLite video ids: {err}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|err| format!("failed to read SQLite video id row: {err}"))?);
    }
    Ok(out)
}

pub fn summarize_case_db(case_dir: &Path) -> Result<Option<CaseDbSummary>, String> {
    let path = case_db_path(case_dir);
    if !path.is_file() {
        return Ok(None);
    }

    let conn = open_readonly_case_db(&path)?;
    if !table_exists(&conn, "videos")? {
        return Ok(Some(CaseDbSummary {
            path,
            video_count: 0,
            scan_run_count: 0,
        }));
    }

    let video_count = count_table_rows(&conn, "videos")?;
    let scan_run_count = if table_exists(&conn, "scan_runs")? {
        count_table_rows(&conn, "scan_runs")?
    } else {
        0
    };
    Ok(Some(CaseDbSummary {
        path,
        video_count,
        scan_run_count,
    }))
}

fn open_case_db(case_dir: &Path) -> Result<Connection, String> {
    fs::create_dir_all(case_dir.join("db"))
        .map_err(|err| format!("failed to create case db directory: {err}"))?;
    let conn = Connection::open(case_db_path(case_dir))
        .map_err(|err| format!("failed to open SQLite case db: {err}"))?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|err| format!("failed to configure SQLite busy timeout: {err}"))?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|err| format!("failed to enable SQLite foreign keys: {err}"))?;
    Ok(conn)
}

fn open_readonly_case_db(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|err| format!("failed to open SQLite case db {}: {err}", path.display()))?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|err| format!("failed to configure SQLite busy timeout: {err}"))?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS schema_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        INSERT OR IGNORE INTO schema_meta (key, value)
        VALUES ('schema_version', '1');

        CREATE TABLE IF NOT EXISTS scan_runs (
            run_pk INTEGER PRIMARY KEY AUTOINCREMENT,
            source_path TEXT NOT NULL,
            scanned_unix INTEGER NOT NULL,
            hash_files INTEGER NOT NULL,
            use_ffprobe INTEGER NOT NULL,
            max_depth INTEGER,
            video_count INTEGER NOT NULL,
            total_bytes INTEGER NOT NULL,
            warnings_json TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS scan_runs_scanned_unix_idx
            ON scan_runs (scanned_unix);

        CREATE TABLE IF NOT EXISTS videos (
            id TEXT PRIMARY KEY,
            source_path TEXT NOT NULL UNIQUE,
            file_url TEXT NOT NULL,
            relative_path TEXT NOT NULL,
            extension TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            modified_unix INTEGER,
            sha256 TEXT,
            hash_status TEXT NOT NULL,
            confidence TEXT NOT NULL,
            source_profile_json TEXT NOT NULL,
            duration_seconds REAL,
            format_name TEXT,
            video_codec TEXT,
            audio_codec TEXT,
            width INTEGER,
            height INTEGER,
            ffprobe_ok INTEGER NOT NULL,
            ffprobe_error TEXT,
            ffprobe_json TEXT,
            first_indexed_unix INTEGER NOT NULL,
            last_indexed_unix INTEGER NOT NULL,
            last_scanned_unix INTEGER,
            record_json TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS videos_sha256_idx
            ON videos (sha256);
        CREATE INDEX IF NOT EXISTS videos_extension_idx
            ON videos (extension);
        CREATE INDEX IF NOT EXISTS videos_last_indexed_idx
            ON videos (last_indexed_unix);
        "#,
    )
    .map_err(|err| format!("failed to initialize SQLite schema: {err}"))?;

    let stored: Option<String> = conn
        .query_row(
            "SELECT value FROM schema_meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("failed to read SQLite schema version: {err}"))?;
    if stored.as_deref() != Some(SCHEMA_VERSION) {
        return Err(format!(
            "unsupported SQLite schema version: {}",
            stored.unwrap_or_else(|| "missing".to_string())
        ));
    }
    Ok(())
}

fn insert_scan_run(tx: &Transaction<'_>, result: &ScanResult) -> Result<(), String> {
    tx.execute(
        r#"
        INSERT INTO scan_runs (
            source_path,
            scanned_unix,
            hash_files,
            use_ffprobe,
            max_depth,
            video_count,
            total_bytes,
            warnings_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        params![
            result.source_path.to_string_lossy().to_string(),
            u64_to_i64(result.scanned_unix),
            bool_to_i64(result.options.hash_files),
            bool_to_i64(result.options.use_ffprobe),
            result.options.max_depth.map(usize_to_i64),
            usize_to_i64(result.video_count),
            u64_to_i64(result.total_bytes),
            string_array_json(&result.warnings),
        ],
    )
    .map_err(|err| format!("failed to insert SQLite scan run: {err}"))?;
    Ok(())
}

fn upsert_indexed_record(
    tx: &Transaction<'_>,
    record: &IndexedVideoRow,
    indexed_unix: u64,
) -> Result<(), String> {
    tx.execute(
        r#"
        INSERT INTO videos (
            id,
            source_path,
            file_url,
            relative_path,
            extension,
            size_bytes,
            modified_unix,
            sha256,
            hash_status,
            confidence,
            source_profile_json,
            duration_seconds,
            format_name,
            video_codec,
            audio_codec,
            width,
            height,
            ffprobe_ok,
            ffprobe_error,
            ffprobe_json,
            first_indexed_unix,
            last_indexed_unix,
            last_scanned_unix,
            record_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, NULL, ?23)
        ON CONFLICT(source_path) DO UPDATE SET
            id = excluded.id,
            file_url = excluded.file_url,
            relative_path = excluded.relative_path,
            extension = excluded.extension,
            size_bytes = excluded.size_bytes,
            modified_unix = excluded.modified_unix,
            sha256 = excluded.sha256,
            hash_status = excluded.hash_status,
            confidence = excluded.confidence,
            source_profile_json = excluded.source_profile_json,
            duration_seconds = excluded.duration_seconds,
            format_name = excluded.format_name,
            video_codec = excluded.video_codec,
            audio_codec = excluded.audio_codec,
            width = excluded.width,
            height = excluded.height,
            ffprobe_ok = excluded.ffprobe_ok,
            ffprobe_error = excluded.ffprobe_error,
            ffprobe_json = excluded.ffprobe_json,
            last_indexed_unix = excluded.last_indexed_unix,
            record_json = excluded.record_json
        "#,
        params![
            record.id.as_str(),
            record.source_path.as_str(),
            record.file_url.as_str(),
            record.relative_path.as_str(),
            record.extension.as_str(),
            u64_to_i64(record.size_bytes),
            record.modified_unix.map(u64_to_i64),
            record.sha256.as_deref(),
            record.hash_status.as_str(),
            record.confidence.as_str(),
            record.source_profile_json.as_str(),
            record.duration_seconds,
            record.format_name.as_deref(),
            record.video_codec.as_deref(),
            record.audio_codec.as_deref(),
            record.width.map(u64_to_i64),
            record.height.map(u64_to_i64),
            bool_to_i64(record.ffprobe_ok),
            record.ffprobe_error.as_deref(),
            record.ffprobe_json.as_deref(),
            u64_to_i64(indexed_unix),
            u64_to_i64(indexed_unix),
            record.record_json.as_str(),
        ],
    )
    .map_err(|err| format!("failed to upsert SQLite indexed video {}: {err}", record.id))?;
    Ok(())
}

fn upsert_scanned_record(
    tx: &Transaction<'_>,
    record: &VideoRecord,
    scanned_unix: u64,
) -> Result<(), String> {
    tx.execute(
        r#"
        INSERT INTO videos (
            id,
            source_path,
            file_url,
            relative_path,
            extension,
            size_bytes,
            modified_unix,
            sha256,
            hash_status,
            confidence,
            source_profile_json,
            duration_seconds,
            format_name,
            video_codec,
            audio_codec,
            width,
            height,
            ffprobe_ok,
            ffprobe_error,
            ffprobe_json,
            first_indexed_unix,
            last_indexed_unix,
            last_scanned_unix,
            record_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)
        ON CONFLICT(source_path) DO UPDATE SET
            id = excluded.id,
            file_url = excluded.file_url,
            relative_path = excluded.relative_path,
            extension = excluded.extension,
            size_bytes = excluded.size_bytes,
            modified_unix = excluded.modified_unix,
            sha256 = excluded.sha256,
            hash_status = excluded.hash_status,
            confidence = excluded.confidence,
            source_profile_json = excluded.source_profile_json,
            duration_seconds = excluded.duration_seconds,
            format_name = excluded.format_name,
            video_codec = excluded.video_codec,
            audio_codec = excluded.audio_codec,
            width = excluded.width,
            height = excluded.height,
            ffprobe_ok = excluded.ffprobe_ok,
            ffprobe_error = excluded.ffprobe_error,
            ffprobe_json = excluded.ffprobe_json,
            last_indexed_unix = excluded.last_indexed_unix,
            last_scanned_unix = excluded.last_scanned_unix,
            record_json = excluded.record_json
        "#,
        params![
            record.id.as_str(),
            record.source_path.to_string_lossy().to_string(),
            path_to_file_url(&record.source_path),
            record.relative_path.as_str(),
            record.extension.as_str(),
            u64_to_i64(record.size_bytes),
            record.modified_unix.map(u64_to_i64),
            record.sha256.as_deref(),
            record.hash_status.as_str(),
            record.confidence.as_str(),
            record.source_profile.to_json(),
            record.probe.duration_seconds,
            record.probe.format_name.as_deref(),
            record.probe.video_codec.as_deref(),
            record.probe.audio_codec.as_deref(),
            record.probe.width.map(u32_to_i64),
            record.probe.height.map(u32_to_i64),
            bool_to_i64(record.probe.ok),
            record.probe.error.as_deref(),
            record.probe.raw_json.as_deref(),
            u64_to_i64(scanned_unix),
            u64_to_i64(scanned_unix),
            u64_to_i64(scanned_unix),
            record.to_json(),
        ],
    )
    .map_err(|err| format!("failed to upsert SQLite scanned video {}: {err}", record.id))?;
    Ok(())
}

fn table_exists(conn: &Connection, table_name: &str) -> Result<bool, String> {
    let found: Option<String> = conn
        .query_row(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table_name],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("failed to inspect SQLite schema: {err}"))?;
    Ok(found.is_some())
}

fn count_table_rows(conn: &Connection, table_name: &str) -> Result<u64, String> {
    let sql = match table_name {
        "videos" => "SELECT COUNT(*) FROM videos",
        "scan_runs" => "SELECT COUNT(*) FROM scan_runs",
        _ => return Err(format!("unsupported SQLite count table: {table_name}")),
    };
    let count: i64 = conn
        .query_row(sql, [], |row| row.get(0))
        .map_err(|err| format!("failed to count SQLite table {table_name}: {err}"))?;
    Ok(count.max(0) as u64)
}

fn string_array_json(values: &[String]) -> String {
    let body = values
        .iter()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{body}]")
}

fn bool_to_i64(value: bool) -> i64 {
    i64::from(value)
}

fn u32_to_i64(value: u32) -> i64 {
    i64::from(value)
}

fn u64_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{load_video_ids, summarize_case_db, write_scan_index};
    use crate::case_db::IndexedVideoRow;
    use crate::model::{ProbeSummary, ScanOptions, ScanResult, SourceProfile, VideoRecord};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn writes_scan_records_to_sqlite_without_duplicate_rows() {
        let case_dir =
            std::env::temp_dir().join(format!("frametrace-case-db-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&case_dir);
        fs::create_dir_all(case_dir.join("db")).unwrap();

        let first = video("vid_000001", "/evidence/one.mp4", None);
        let first_row = indexed_row(&first);
        let result = scan_result(vec![first.clone()], 1);
        write_scan_index(&case_dir, &result, &[first_row]).unwrap();

        let rescanned = VideoRecord {
            sha256: Some("abc".to_string()),
            hash_status: "complete".to_string(),
            ..first
        };
        let result = scan_result(vec![rescanned.clone()], 2);
        write_scan_index(&case_dir, &result, &[indexed_row(&rescanned)]).unwrap();

        let summary = summarize_case_db(&case_dir).unwrap().unwrap();
        assert_eq!(summary.video_count, 1);
        assert_eq!(summary.scan_run_count, 2);

        let ids = load_video_ids(&case_dir).unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0].id, "vid_000001");
        assert_eq!(ids[0].source_path, "/evidence/one.mp4");

        let _ = fs::remove_dir_all(case_dir);
    }

    fn video(id: &str, source_path: &str, sha256: Option<String>) -> VideoRecord {
        VideoRecord {
            id: id.to_string(),
            source_path: PathBuf::from(source_path),
            relative_path: PathBuf::from(source_path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            extension: "mp4".to_string(),
            size_bytes: 42,
            modified_unix: Some(10),
            sha256,
            hash_status: "skipped".to_string(),
            probe: ProbeSummary::skipped(),
            confidence: "extension-candidate".to_string(),
            source_profile: SourceProfile::generic_media("test"),
        }
    }

    fn indexed_row(record: &VideoRecord) -> IndexedVideoRow {
        IndexedVideoRow {
            id: record.id.clone(),
            source_path: record.source_path.to_string_lossy().to_string(),
            file_url: format!("file://{}", record.source_path.display()),
            relative_path: record.relative_path.clone(),
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

    fn scan_result(records: Vec<VideoRecord>, scanned_unix: u64) -> ScanResult {
        ScanResult {
            source_path: PathBuf::from("/evidence"),
            scanned_unix,
            video_count: records.len(),
            total_bytes: records.iter().map(|record| record.size_bytes).sum(),
            warnings: Vec::new(),
            options: ScanOptions::default(),
            records,
        }
    }
}
