use super::DbBenchmarkQueryPlan;
use rusqlite::{Connection, Params, params};

pub(crate) fn benchmark_query_plans(
    conn: &Connection,
) -> Result<Vec<DbBenchmarkQueryPlan>, String> {
    let queries = [
        collect_query_plan(
            conn,
            "extension_count",
            "SELECT COUNT(*) FROM videos WHERE extension = ?1",
            params!["mp4"],
        )?,
        collect_query_plan(
            conn,
            "source_lookup",
            "SELECT id FROM videos WHERE source_path = ?1",
            params!["C:/Evidence/bench/clip_00000000.mp4"],
        )?,
        collect_query_plan(
            conn,
            "sha256_lookup",
            "SELECT id FROM videos WHERE sha256 = ?1",
            params!["benchmark_hash_00000000"],
        )?,
        collect_query_plan(
            conn,
            "timeline_recent",
            "SELECT id FROM videos WHERE extension = ?1 ORDER BY modified_unix DESC LIMIT 100",
            params!["mp4"],
        )?,
        collect_query_plan(
            conn,
            "inventory_validation_candidates",
            "SELECT id FROM videos WHERE ffprobe_ok = ?1 ORDER BY modified_unix ASC, id ASC LIMIT 100",
            params![0],
        )?,
        collect_query_plan(
            conn,
            "inventory_hash_state",
            "SELECT id FROM videos WHERE hash_status = ?1 ORDER BY id LIMIT 100",
            params!["benchmark"],
        )?,
        collect_query_plan(
            conn,
            "inventory_path_prefix",
            "SELECT id FROM videos WHERE relative_path >= ?1 AND relative_path < ?2 ORDER BY relative_path LIMIT 100",
            params!["clip_000000", "clip_000001"],
        )?,
        collect_query_plan(
            conn,
            "inventory_recent_since",
            "SELECT id FROM videos WHERE modified_unix >= ?1 ORDER BY modified_unix ASC LIMIT 100",
            params![0],
        )?,
    ];
    Ok(queries.into_iter().collect())
}

fn collect_query_plan<P: Params>(
    conn: &Connection,
    label: &str,
    sql: &str,
    params: P,
) -> Result<DbBenchmarkQueryPlan, String> {
    let explain_sql = format!("EXPLAIN QUERY PLAN {sql}");
    let mut stmt = conn
        .prepare(&explain_sql)
        .map_err(|err| format!("failed to prepare query plan for {label}: {err}"))?;
    let rows = stmt
        .query_map(params, |row| row.get::<_, String>(3))
        .map_err(|err| format!("failed to query plan for {label}: {err}"))?;
    let mut details = Vec::new();
    for row in rows {
        details.push(row.map_err(|err| format!("failed to read query plan for {label}: {err}"))?);
    }
    Ok(DbBenchmarkQueryPlan {
        label: label.to_string(),
        sql: sql.to_string(),
        detail: details.join(" | "),
    })
}
