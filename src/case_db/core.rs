use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(crate) const SCHEMA_VERSION: &str = "3";

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
    match stored.as_deref() {
        None => {
            apply_schema_v2_indexes(conn)?;
            apply_schema_v3_tables(conn)?;
            conn.execute(
                "INSERT INTO schema_meta (key, value) VALUES ('schema_version', ?1)",
                [SCHEMA_VERSION],
            )
            .map_err(|err| format!("failed to store SQLite schema version: {err}"))?;
        }
        Some("1") => {
            migrate_v1_to_v2(conn)?;
            migrate_v2_to_v3(conn)?;
        }
        Some("2") => migrate_v2_to_v3(conn)?,
        Some(SCHEMA_VERSION) => {
            apply_schema_v2_indexes(conn)?;
            apply_schema_v3_tables(conn)?;
        }
        Some(version) => {
            return Err(format!("unsupported SQLite schema version: {version}"));
        }
    }
    Ok(())
}

fn migrate_v2_to_v3(conn: &Connection) -> Result<(), String> {
    backup_database(conn, "2", "3")?;
    apply_schema_v3_tables(conn)?;
    conn.execute(
        "UPDATE schema_meta SET value = ?1 WHERE key = 'schema_version'",
        [SCHEMA_VERSION],
    )
    .map_err(|err| format!("failed to update SQLite schema version: {err}"))?;
    Ok(())
}

fn apply_schema_v3_tables(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS review_marks (
            record_id TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            marked_unix INTEGER NOT NULL,
            record_path TEXT,
            examiner TEXT
        );

        CREATE INDEX IF NOT EXISTS review_marks_status_idx
            ON review_marks (status);
        "#,
    )
    .map_err(|err| format!("failed to apply SQLite v3 review marks schema: {err}"))
}

fn migrate_v1_to_v2(conn: &Connection) -> Result<(), String> {
    backup_database(conn, "1", "2")?;
    apply_schema_v2_indexes(conn)?;
    conn.execute(
        "UPDATE schema_meta SET value = ?1 WHERE key = 'schema_version'",
        [SCHEMA_VERSION],
    )
    .map_err(|err| format!("failed to update SQLite schema version: {err}"))?;
    Ok(())
}

fn apply_schema_v2_indexes(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS videos_modified_unix_idx
            ON videos (modified_unix);
        CREATE INDEX IF NOT EXISTS videos_ffprobe_ok_idx
            ON videos (ffprobe_ok);
        CREATE INDEX IF NOT EXISTS videos_confidence_idx
            ON videos (confidence);
        CREATE INDEX IF NOT EXISTS videos_extension_modified_idx
            ON videos (extension, modified_unix);
        CREATE INDEX IF NOT EXISTS videos_last_scanned_idx
            ON videos (last_scanned_unix);
        "#,
    )
    .map_err(|err| format!("failed to apply SQLite v2 indexes: {err}"))
}

fn backup_database(conn: &Connection, from_version: &str, to_version: &str) -> Result<(), String> {
    let Some(db_path) = main_database_path(conn)? else {
        return Ok(());
    };
    if !db_path.is_file() {
        return Ok(());
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("system time before UNIX epoch: {err}"))?
        .as_secs();
    let backup_name = format!("case.db.backup-v{from_version}-to-v{to_version}-{timestamp}");
    let backup_path = db_path.with_file_name(backup_name);
    fs::copy(&db_path, &backup_path).map_err(|err| {
        format!(
            "failed to create SQLite migration backup {}: {err}",
            backup_path.display()
        )
    })?;
    Ok(())
}

fn main_database_path(conn: &Connection) -> Result<Option<PathBuf>, String> {
    let mut stmt = conn
        .prepare("PRAGMA database_list")
        .map_err(|err| format!("failed to inspect SQLite database list: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            let name: String = row.get(1)?;
            let path: String = row.get(2)?;
            Ok((name, path))
        })
        .map_err(|err| format!("failed to query SQLite database list: {err}"))?;
    for row in rows {
        let (name, path) =
            row.map_err(|err| format!("failed to read SQLite database list row: {err}"))?;
        if name == "main" && !path.is_empty() {
            return Ok(Some(PathBuf::from(path)));
        }
    }
    Ok(None)
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

#[cfg(test)]
mod tests {
    use super::{SCHEMA_VERSION, case_db_path, init_schema, open_case_db, table_exists};
    use rusqlite::{Connection, OptionalExtension};
    use std::fs;

    #[test]
    fn initializes_new_databases_at_current_schema_version() {
        let case_dir =
            std::env::temp_dir().join(format!("frametrace-schema-new-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&case_dir);

        let conn = open_case_db(&case_dir).unwrap();
        init_schema(&conn).unwrap();
        assert_eq!(read_schema_version(&conn).as_deref(), Some(SCHEMA_VERSION));
        assert!(index_exists(&conn, "videos_modified_unix_idx"));

        let _ = fs::remove_dir_all(case_dir);
    }

    #[test]
    fn migrates_v1_schema_to_current_with_backup() {
        let case_dir = std::env::temp_dir().join(format!(
            "frametrace-schema-migrate-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&case_dir);
        fs::create_dir_all(case_dir.join("db")).unwrap();
        let db_path = case_db_path(&case_dir);
        let seed = Connection::open(&db_path).unwrap();
        seed.execute_batch(
            r#"
            CREATE TABLE schema_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            INSERT INTO schema_meta (key, value) VALUES ('schema_version', '1');
            "#,
        )
        .unwrap();
        drop(seed);

        let conn = open_case_db(&case_dir).unwrap();
        init_schema(&conn).unwrap();
        assert_eq!(read_schema_version(&conn).as_deref(), Some(SCHEMA_VERSION));
        assert!(index_exists(&conn, "videos_extension_modified_idx"));
        assert!(table_exists(&conn, "review_marks").unwrap());
        let backup_exists = fs::read_dir(case_dir.join("db"))
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("case.db.backup-v1-to-v2-")
            });
        assert!(backup_exists);

        let _ = fs::remove_dir_all(case_dir);
    }

    #[test]
    fn migrates_v2_schema_to_v3_with_review_marks() {
        let case_dir = std::env::temp_dir().join(format!(
            "frametrace-schema-v2-migrate-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&case_dir);
        fs::create_dir_all(case_dir.join("db")).unwrap();
        let db_path = case_db_path(&case_dir);
        let seed = Connection::open(&db_path).unwrap();
        seed.execute_batch(
            r#"
            CREATE TABLE schema_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            INSERT INTO schema_meta (key, value) VALUES ('schema_version', '2');
            "#,
        )
        .unwrap();
        drop(seed);

        let conn = open_case_db(&case_dir).unwrap();
        init_schema(&conn).unwrap();
        assert_eq!(read_schema_version(&conn).as_deref(), Some(SCHEMA_VERSION));
        assert!(table_exists(&conn, "review_marks").unwrap());
        let backup_exists = fs::read_dir(case_dir.join("db"))
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("case.db.backup-v2-to-v3-")
            });
        assert!(backup_exists);

        let _ = fs::remove_dir_all(case_dir);
    }

    fn read_schema_version(conn: &Connection) -> Option<String> {
        conn.query_row(
            "SELECT value FROM schema_meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .optional()
        .unwrap()
    }

    fn index_exists(conn: &Connection, index_name: &str) -> bool {
        conn.query_row(
            "SELECT name FROM sqlite_master WHERE type = 'index' AND name = ?1",
            [index_name],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .unwrap()
        .is_some()
    }
}
