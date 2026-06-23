use crate::tool_policy::require_case_output_path;
use rusqlite::{Connection, OpenFlags};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn case_db_path(case_dir: &Path) -> PathBuf {
    case_dir.join("db/case.db")
}

pub(crate) fn open_case_db(case_dir: &Path) -> Result<Connection, String> {
    fs::create_dir_all(case_dir)
        .map_err(|err| format!("failed to create case directory: {err}"))?;
    let db_path = case_db_path(case_dir);
    require_case_output_path(case_dir, &db_path, "SQLite case db")?;
    fs::create_dir_all(db_path.parent().unwrap_or(case_dir))
        .map_err(|err| format!("failed to create case db directory: {err}"))?;
    let conn =
        Connection::open(db_path).map_err(|err| format!("failed to open SQLite case db: {err}"))?;
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
