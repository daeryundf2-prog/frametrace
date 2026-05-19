use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub(crate) const SCHEMA_VERSION: &str = "1";

pub fn case_db_path(case_dir: &Path) -> PathBuf {
    case_dir.join("db/case.db")
}

pub(crate) fn open_case_db(case_dir: &Path) -> Result<Connection, String> {
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

pub(crate) fn open_readonly_case_db(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|err| format!("failed to open SQLite case db {}: {err}", path.display()))?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|err| format!("failed to configure SQLite busy timeout: {err}"))?;
    Ok(conn)
}

pub(crate) fn init_schema(conn: &Connection) -> Result<(), String> {
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

pub(crate) fn table_exists(conn: &Connection, table_name: &str) -> Result<bool, String> {
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
