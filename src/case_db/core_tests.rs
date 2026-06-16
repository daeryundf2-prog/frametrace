use super::{SCHEMA_VERSION, case_db_path, init_schema, open_case_db};
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
    assert!(index_exists(&conn, "videos_inventory_default_idx"));

    let _ = fs::remove_dir_all(case_dir);
}

#[test]
fn migrates_v1_schema_to_current_with_ordered_backups() {
    let case_dir =
        std::env::temp_dir().join(format!("frametrace-schema-v1-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&case_dir);
    seed_schema_version(&case_dir, "1");

    let conn = open_case_db(&case_dir).unwrap();
    init_schema(&conn).unwrap();

    assert_eq!(read_schema_version(&conn).as_deref(), Some(SCHEMA_VERSION));
    assert!(index_exists(&conn, "videos_extension_modified_idx"));
    assert!(index_exists(&conn, "videos_inventory_default_idx"));
    assert!(backup_exists(&case_dir, "case.db.backup-v1-to-v2-"));
    assert!(backup_exists(&case_dir, "case.db.backup-v2-to-v3-"));

    let _ = fs::remove_dir_all(case_dir);
}

#[test]
fn migrates_v2_schema_to_inventory_contract_with_backup() {
    let case_dir =
        std::env::temp_dir().join(format!("frametrace-schema-v2-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&case_dir);
    seed_schema_version(&case_dir, "2");

    let conn = open_case_db(&case_dir).unwrap();
    init_schema(&conn).unwrap();

    assert_eq!(read_schema_version(&conn).as_deref(), Some(SCHEMA_VERSION));
    assert!(index_exists(&conn, "videos_relative_path_idx"));
    assert!(index_exists(&conn, "videos_hash_status_idx"));
    assert!(backup_exists(&case_dir, "case.db.backup-v2-to-v3-"));

    let _ = fs::remove_dir_all(case_dir);
}

fn seed_schema_version(case_dir: &std::path::Path, version: &str) {
    fs::create_dir_all(case_dir.join("db")).unwrap();
    let db_path = case_db_path(case_dir);
    let seed = Connection::open(&db_path).unwrap();
    seed.execute_batch(
        r#"
        CREATE TABLE schema_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    )
    .unwrap();
    seed.execute(
        "INSERT INTO schema_meta (key, value) VALUES ('schema_version', ?1)",
        [version],
    )
    .unwrap();
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

fn backup_exists(case_dir: &std::path::Path, prefix: &str) -> bool {
    fs::read_dir(case_dir.join("db"))
        .unwrap()
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_string_lossy().starts_with(prefix))
}
