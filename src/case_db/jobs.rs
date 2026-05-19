use super::*;
use crate::model::{ScanResult, VideoRecord};
use crate::util::{json_escape, now_unix, path_to_file_url};
use rusqlite::{Connection, OpenFlags, OptionalExtension, Transaction, params};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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

pub(crate) fn finish_job(
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

pub(crate) fn append_job_event(
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

pub(crate) fn count_jobs_by_status(conn: &Connection, status: &str) -> Result<u64, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM jobs WHERE status = ?1",
            [status],
            |row| row.get(0),
        )
        .map_err(|err| format!("failed to count SQLite jobs with status {status}: {err}"))?;
    Ok(count.max(0) as u64)
}
