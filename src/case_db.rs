use crate::model::{ScanResult, VideoRecord};
use crate::util::{json_escape, now_unix, path_to_file_url};
use rusqlite::{Connection, OpenFlags, OptionalExtension, Transaction, params};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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
    pub evidence_source_count: u64,
    pub job_count: u64,
    pub active_job_count: u64,
}

#[derive(Debug, Clone)]
pub struct EvidenceSourceInput {
    pub kind: String,
    pub path: PathBuf,
    pub source_id: Option<String>,
    pub write_protect: Option<String>,
    pub acquisition_tool: Option<String>,
    pub evidence_hash: Option<String>,
    pub notes: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EvidenceSourceRow {
    pub source_id: String,
    pub kind: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct JobRecord {
    pub job_id: String,
    pub job_type: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct DbBenchmarkResult {
    pub path: PathBuf,
    pub rows: usize,
    pub elapsed_ms: u128,
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

pub fn register_evidence_source(
    case_dir: &Path,
    input: &EvidenceSourceInput,
) -> Result<EvidenceSourceRow, String> {
    let conn = open_case_db(case_dir)?;
    init_schema(&conn)?;
    let now = now_unix()?;
    let path = input.path.to_string_lossy().to_string();
    let source_id = input
        .source_id
        .clone()
        .unwrap_or_else(|| stable_source_id(&input.kind, &path));
    conn.execute(
        r#"
        INSERT INTO evidence_sources (
            source_id,
            kind,
            path,
            registered_unix,
            last_seen_unix,
            write_protect,
            acquisition_tool,
            evidence_hash,
            notes,
            metadata_json
        )
        VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(source_id) DO UPDATE SET
            kind = excluded.kind,
            path = excluded.path,
            last_seen_unix = excluded.last_seen_unix,
            write_protect = excluded.write_protect,
            acquisition_tool = excluded.acquisition_tool,
            evidence_hash = excluded.evidence_hash,
            notes = excluded.notes,
            metadata_json = excluded.metadata_json
        ON CONFLICT(kind, path) DO UPDATE SET
            last_seen_unix = excluded.last_seen_unix,
            write_protect = COALESCE(excluded.write_protect, evidence_sources.write_protect),
            acquisition_tool = COALESCE(excluded.acquisition_tool, evidence_sources.acquisition_tool),
            evidence_hash = COALESCE(excluded.evidence_hash, evidence_sources.evidence_hash),
            notes = COALESCE(excluded.notes, evidence_sources.notes),
            metadata_json = excluded.metadata_json
        "#,
        params![
            source_id.as_str(),
            input.kind.as_str(),
            path.as_str(),
            u64_to_i64(now),
            input.write_protect.as_deref(),
            input.acquisition_tool.as_deref(),
            input.evidence_hash.as_deref(),
            input.notes.as_deref(),
            input.metadata_json.as_deref().unwrap_or("{}"),
        ],
    )
    .map_err(|err| format!("failed to register evidence source: {err}"))?;

    conn.query_row(
        "SELECT source_id, kind, path FROM evidence_sources WHERE kind = ?1 AND path = ?2",
        params![input.kind.as_str(), path.as_str()],
        |row| {
            Ok(EvidenceSourceRow {
                source_id: row.get(0)?,
                kind: row.get(1)?,
                path: row.get(2)?,
            })
        },
    )
    .map_err(|err| format!("failed to read registered evidence source: {err}"))
}

pub fn start_job(
    case_dir: &Path,
    job_type: &str,
    subject_path: &Path,
    total_units: Option<u64>,
    options_json: &str,
) -> Result<JobRecord, String> {
    let conn = open_case_db(case_dir)?;
    init_schema(&conn)?;
    let now = now_unix()?;
    let job_number = count_table_rows(&conn, "jobs")?.saturating_add(1);
    let job_id = format!("job_{now}_{job_number:06}");
    conn.execute(
        r#"
        INSERT INTO jobs (
            job_id,
            job_type,
            status,
            subject_path,
            started_unix,
            updated_unix,
            total_units,
            completed_units,
            options_json
        )
        VALUES (?1, ?2, 'running', ?3, ?4, ?4, ?5, 0, ?6)
        "#,
        params![
            job_id.as_str(),
            job_type,
            subject_path.to_string_lossy().to_string(),
            u64_to_i64(now),
            total_units.map(u64_to_i64),
            options_json,
        ],
    )
    .map_err(|err| format!("failed to start SQLite job: {err}"))?;
    append_job_event(case_dir, &job_id, "started", "job started", Some(0))?;
    Ok(JobRecord {
        job_id,
        job_type: job_type.to_string(),
        status: "running".to_string(),
    })
}

pub fn update_job_progress(
    case_dir: &Path,
    job_id: &str,
    completed_units: u64,
    message: &str,
) -> Result<(), String> {
    let conn = open_case_db(case_dir)?;
    init_schema(&conn)?;
    let now = now_unix()?;
    conn.execute(
        "UPDATE jobs SET completed_units = ?1, updated_unix = ?2 WHERE job_id = ?3",
        params![u64_to_i64(completed_units), u64_to_i64(now), job_id],
    )
    .map_err(|err| format!("failed to update SQLite job progress: {err}"))?;
    append_job_event(case_dir, job_id, "progress", message, Some(completed_units))
}

pub fn complete_job(
    case_dir: &Path,
    job_id: &str,
    completed_units: u64,
    message: &str,
) -> Result<(), String> {
    finish_job(case_dir, job_id, "complete", completed_units, message, None)
}

pub fn fail_job(case_dir: &Path, job_id: &str, error: &str) -> Result<(), String> {
    finish_job(case_dir, job_id, "failed", 0, "job failed", Some(error))
}

pub fn benchmark_case_db(output_dir: &Path, rows: usize) -> Result<DbBenchmarkResult, String> {
    fs::create_dir_all(output_dir)
        .map_err(|err| format!("failed to create benchmark directory: {err}"))?;
    let mut conn = open_case_db(output_dir)?;
    init_schema(&conn)?;
    let started = Instant::now();
    let now = now_unix()?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start benchmark transaction: {err}"))?;
    for index in 0..rows {
        let id = format!("bench_{index:08}");
        let source_path = format!("C:/Evidence/bench/clip_{index:08}.mp4");
        let record = IndexedVideoRow {
            id,
            source_path: source_path.clone(),
            file_url: format!("file:///{source_path}"),
            relative_path: format!("clip_{index:08}.mp4"),
            extension: "mp4".to_string(),
            size_bytes: 1024 + index as u64,
            modified_unix: None,
            sha256: None,
            hash_status: "benchmark".to_string(),
            confidence: "benchmark".to_string(),
            source_profile_json: "{\"lane\":\"benchmark\",\"vendor\":\"Benchmark\",\"parser\":\"benchmark\",\"confidence\":\"benchmark\",\"recommended_action\":\"Synthetic row only\",\"evidence\":[]}".to_string(),
            duration_seconds: None,
            format_name: None,
            video_codec: None,
            audio_codec: None,
            width: None,
            height: None,
            ffprobe_ok: false,
            ffprobe_error: None,
            ffprobe_json: None,
            record_json: format!(
                "{{\"id\":\"bench_{index:08}\",\"source_path\":\"{}\",\"relative_path\":\"clip_{index:08}.mp4\",\"extension\":\"mp4\",\"size_bytes\":{},\"source_profile\":{{\"vendor\":\"Benchmark\",\"parser\":\"benchmark\"}}}}",
                json_escape(&source_path),
                1024 + index as u64
            ),
        };
        upsert_indexed_record(&tx, &record, now)?;
    }
    tx.commit()
        .map_err(|err| format!("failed to commit benchmark transaction: {err}"))?;
    Ok(DbBenchmarkResult {
        path: case_db_path(output_dir),
        rows,
        elapsed_ms: started.elapsed().as_millis(),
    })
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
            evidence_source_count: 0,
            job_count: 0,
            active_job_count: 0,
        }));
    }

    let video_count = count_table_rows(&conn, "videos")?;
    let scan_run_count = if table_exists(&conn, "scan_runs")? {
        count_table_rows(&conn, "scan_runs")?
    } else {
        0
    };
    let evidence_source_count = if table_exists(&conn, "evidence_sources")? {
        count_table_rows(&conn, "evidence_sources")?
    } else {
        0
    };
    let job_count = if table_exists(&conn, "jobs")? {
        count_table_rows(&conn, "jobs")?
    } else {
        0
    };
    let active_job_count = if table_exists(&conn, "jobs")? {
        count_jobs_by_status(&conn, "running")?
    } else {
        0
    };
    Ok(Some(CaseDbSummary {
        path,
        video_count,
        scan_run_count,
        evidence_source_count,
        job_count,
        active_job_count,
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

        CREATE TABLE IF NOT EXISTS evidence_sources (
            source_id TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            path TEXT NOT NULL,
            registered_unix INTEGER NOT NULL,
            last_seen_unix INTEGER NOT NULL,
            write_protect TEXT,
            acquisition_tool TEXT,
            evidence_hash TEXT,
            notes TEXT,
            metadata_json TEXT NOT NULL
        );

        CREATE UNIQUE INDEX IF NOT EXISTS evidence_sources_kind_path_idx
            ON evidence_sources (kind, path);
        CREATE INDEX IF NOT EXISTS evidence_sources_hash_idx
            ON evidence_sources (evidence_hash);

        CREATE TABLE IF NOT EXISTS jobs (
            job_id TEXT PRIMARY KEY,
            job_type TEXT NOT NULL,
            status TEXT NOT NULL,
            subject_path TEXT NOT NULL,
            started_unix INTEGER NOT NULL,
            updated_unix INTEGER NOT NULL,
            completed_unix INTEGER,
            total_units INTEGER,
            completed_units INTEGER NOT NULL DEFAULT 0,
            options_json TEXT NOT NULL,
            error TEXT
        );

        CREATE INDEX IF NOT EXISTS jobs_status_idx
            ON jobs (status);
        CREATE INDEX IF NOT EXISTS jobs_type_started_idx
            ON jobs (job_type, started_unix);

        CREATE TABLE IF NOT EXISTS job_events (
            event_pk INTEGER PRIMARY KEY AUTOINCREMENT,
            job_id TEXT NOT NULL,
            event_unix INTEGER NOT NULL,
            event_type TEXT NOT NULL,
            message TEXT NOT NULL,
            completed_units INTEGER,
            FOREIGN KEY(job_id) REFERENCES jobs(job_id)
        );

        CREATE INDEX IF NOT EXISTS job_events_job_idx
            ON job_events (job_id, event_unix);
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

fn finish_job(
    case_dir: &Path,
    job_id: &str,
    status: &str,
    completed_units: u64,
    message: &str,
    error: Option<&str>,
) -> Result<(), String> {
    let conn = open_case_db(case_dir)?;
    init_schema(&conn)?;
    let now = now_unix()?;
    conn.execute(
        r#"
        UPDATE jobs
        SET status = ?1,
            updated_unix = ?2,
            completed_unix = ?2,
            completed_units = CASE WHEN ?3 > 0 THEN ?3 ELSE completed_units END,
            error = ?4
        WHERE job_id = ?5
        "#,
        params![
            status,
            u64_to_i64(now),
            u64_to_i64(completed_units),
            error,
            job_id,
        ],
    )
    .map_err(|err| format!("failed to finish SQLite job: {err}"))?;
    append_job_event(case_dir, job_id, status, message, Some(completed_units))
}

fn append_job_event(
    case_dir: &Path,
    job_id: &str,
    event_type: &str,
    message: &str,
    completed_units: Option<u64>,
) -> Result<(), String> {
    let conn = open_case_db(case_dir)?;
    init_schema(&conn)?;
    conn.execute(
        r#"
        INSERT INTO job_events (
            job_id,
            event_unix,
            event_type,
            message,
            completed_units
        )
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        params![
            job_id,
            u64_to_i64(now_unix()?),
            event_type,
            message,
            completed_units.map(u64_to_i64),
        ],
    )
    .map_err(|err| format!("failed to append SQLite job event: {err}"))?;
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
        "evidence_sources" => "SELECT COUNT(*) FROM evidence_sources",
        "jobs" => "SELECT COUNT(*) FROM jobs",
        _ => return Err(format!("unsupported SQLite count table: {table_name}")),
    };
    let count: i64 = conn
        .query_row(sql, [], |row| row.get(0))
        .map_err(|err| format!("failed to count SQLite table {table_name}: {err}"))?;
    Ok(count.max(0) as u64)
}

fn count_jobs_by_status(conn: &Connection, status: &str) -> Result<u64, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM jobs WHERE status = ?1",
            [status],
            |row| row.get(0),
        )
        .map_err(|err| format!("failed to count SQLite jobs with status {status}: {err}"))?;
    Ok(count.max(0) as u64)
}

fn stable_source_id(kind: &str, path: &str) -> String {
    let digest = crate::sha256::digest_bytes(format!("{kind}\n{path}").as_bytes());
    format!("src_{}", &digest[..16])
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
    use super::{
        EvidenceSourceInput, load_video_ids, register_evidence_source, summarize_case_db,
        write_scan_index,
    };
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

    #[test]
    fn preserves_manual_source_id_when_auto_registering_same_path() {
        let case_dir =
            std::env::temp_dir().join(format!("frametrace-source-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&case_dir);
        fs::create_dir_all(case_dir.join("db")).unwrap();
        let source_path = PathBuf::from("/evidence/source");

        let manual = register_evidence_source(
            &case_dir,
            &EvidenceSourceInput {
                kind: "folder".to_string(),
                path: source_path.clone(),
                source_id: Some("SD001".to_string()),
                write_protect: Some("hardware".to_string()),
                acquisition_tool: None,
                evidence_hash: None,
                notes: Some("intake".to_string()),
                metadata_json: None,
            },
        )
        .unwrap();
        let auto = register_evidence_source(
            &case_dir,
            &EvidenceSourceInput {
                kind: "folder".to_string(),
                path: source_path,
                source_id: None,
                write_protect: None,
                acquisition_tool: None,
                evidence_hash: None,
                notes: Some("auto".to_string()),
                metadata_json: None,
            },
        )
        .unwrap();

        assert_eq!(manual.source_id, "SD001");
        assert_eq!(auto.source_id, "SD001");
        assert_eq!(
            summarize_case_db(&case_dir)
                .unwrap()
                .unwrap()
                .evidence_source_count,
            1
        );

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
