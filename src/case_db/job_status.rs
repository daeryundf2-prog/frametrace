use super::*;
use crate::util::json_escape;
use rusqlite::Connection;
use std::path::Path;

const JOB_STATUS_LIMIT: usize = 20;

pub fn runtime_jobs_json(case_dir: &Path) -> Result<String, String> {
    if !case_db_path(case_dir).is_file() {
        return Ok(empty_runtime_jobs_json());
    }
    let conn = open_case_db(case_dir)?;
    init_schema(&conn)?;
    let rows = job_status_rows(&conn, JOB_STATUS_LIMIT)?;
    let running_count = count_jobs_by_status(&conn, "running")?;
    let interrupted_count = count_jobs_by_status(&conn, "interrupted")?;
    let completed_count = count_jobs_by_status(&conn, "complete")?;
    let failed_count = count_jobs_by_status(&conn, "failed")?;
    let recent = rows
        .iter()
        .map(job_status_json)
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!(
        "{{\"recent_limit\":{},\"total_count\":{},\"running_count\":{},\
\"interrupted_count\":{},\"completed_count\":{},\"failed_count\":{},\
\"resume_policy\":\"blocked-unless-idempotence-proven\",\"recent\":[{}]}}",
        JOB_STATUS_LIMIT,
        count_table_rows(&conn, "jobs")?,
        running_count,
        interrupted_count,
        completed_count,
        failed_count,
        recent
    ))
}

fn empty_runtime_jobs_json() -> String {
    format!(
        "{{\"recent_limit\":{},\"total_count\":0,\"running_count\":0,\
\"interrupted_count\":0,\"completed_count\":0,\"failed_count\":0,\
\"resume_policy\":\"blocked-unless-idempotence-proven\",\"recent\":[]}}",
        JOB_STATUS_LIMIT
    )
}

fn job_status_rows(conn: &Connection, limit: usize) -> Result<Vec<JobStatusRow>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT job_id, job_type, status, subject_path, started_unix, updated_unix,
                   completed_unix, total_units, completed_units, error
            FROM jobs
            ORDER BY updated_unix DESC, started_unix DESC, job_id DESC
            LIMIT ?1
            "#,
        )
        .map_err(|err| format!("failed to prepare job status query: {err}"))?;
    let rows = stmt
        .query_map([usize_to_i64(limit)], |row| {
            Ok(JobStatusRow {
                job_id: row.get(0)?,
                job_type: row.get(1)?,
                status: row.get(2)?,
                subject_path: row.get(3)?,
                started_unix: i64_to_u64(row.get::<_, i64>(4)?),
                updated_unix: i64_to_u64(row.get::<_, i64>(5)?),
                completed_unix: i64_option_to_u64(row.get::<_, Option<i64>>(6)?),
                total_units: i64_option_to_u64(row.get::<_, Option<i64>>(7)?),
                completed_units: i64_to_u64(row.get::<_, i64>(8)?),
                error: row.get(9)?,
            })
        })
        .map_err(|err| format!("failed to query job status rows: {err}"))?;
    let mut jobs = Vec::new();
    for row in rows {
        jobs.push(row.map_err(|err| format!("failed to read job status row: {err}"))?);
    }
    Ok(jobs)
}

fn job_status_json(row: &JobStatusRow) -> String {
    format!(
        "{{\"job_id\":\"{}\",\"job_type\":\"{}\",\"status\":\"{}\",\
\"subject_path\":\"{}\",\"started_unix\":{},\"updated_unix\":{},\
\"completed_unix\":{},\"total_units\":{},\"completed_units\":{},\
\"progress_percent\":{},\"eta_seconds\":{},\"eta_state\":\"{}\",\
\"resume_blocker\":{},\"error\":{}}}",
        json_escape(&row.job_id),
        json_escape(&row.job_type),
        json_escape(&row.status),
        json_escape(&row.subject_path),
        row.started_unix,
        row.updated_unix,
        optional_u64_json(row.completed_unix),
        optional_u64_json(row.total_units),
        row.completed_units,
        optional_string_json(progress_percent(row)),
        optional_u64_json(eta_seconds(row)),
        eta_state(row),
        resume_blocker_json(row),
        optional_str_json(row.error.as_deref())
    )
}

fn progress_percent(row: &JobStatusRow) -> Option<String> {
    let total_units = row.total_units?;
    if total_units == 0 {
        return None;
    }
    let completed = row.completed_units.min(total_units);
    let tenths = completed.saturating_mul(1000) / total_units;
    Some(format!("{}.{}", tenths / 10, tenths % 10))
}

fn eta_seconds(row: &JobStatusRow) -> Option<u64> {
    if row.status != "running" {
        return None;
    }
    let total_units = row.total_units?;
    let remaining = total_units.checked_sub(row.completed_units)?;
    if remaining == 0 || row.completed_units == 0 {
        return None;
    }
    let elapsed = row.updated_unix.saturating_sub(row.started_unix);
    if elapsed == 0 {
        return None;
    }
    Some(remaining.saturating_mul(elapsed) / row.completed_units)
}

fn eta_state(row: &JobStatusRow) -> &'static str {
    match row.status.as_str() {
        "running" if eta_seconds(row).is_some() => "estimated",
        "running" => "calculating",
        _ => "not-applicable",
    }
}

fn resume_blocker_json(row: &JobStatusRow) -> String {
    match row.status.as_str() {
        "interrupted" => "\"resume-disabled-idempotence-not-proven\"".to_string(),
        _ => "null".to_string(),
    }
}

fn optional_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn optional_string_json(value: Option<String>) -> String {
    value
        .map(|value| format!("\"{}\"", json_escape(&value)))
        .unwrap_or_else(|| "null".to_string())
}

fn optional_str_json(value: Option<&str>) -> String {
    value
        .map(|value| format!("\"{}\"", json_escape(value)))
        .unwrap_or_else(|| "null".to_string())
}

fn i64_to_u64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

fn i64_option_to_u64(value: Option<i64>) -> Option<u64> {
    value.and_then(|value| u64::try_from(value).ok())
}
