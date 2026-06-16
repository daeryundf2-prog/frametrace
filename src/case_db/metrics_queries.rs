use rusqlite::{Connection, params};
use std::time::Instant;

pub(crate) fn benchmark_indexed_queries(
    conn: &Connection,
    rows: usize,
) -> Result<(usize, u128, usize), String> {
    let mut query_count = 0usize;
    let mut max_query_ms = 0u128;
    let mut query_rows_returned = 0usize;

    record_query(
        &mut query_count,
        &mut max_query_ms,
        &mut query_rows_returned,
        || {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM videos WHERE extension = ?1",
                    params!["mp4"],
                    |row| row.get(0),
                )
                .map_err(|err| format!("failed benchmark extension query: {err}"))?;
            Ok(count.max(0) as usize)
        },
    )?;

    let midpoint = rows.saturating_sub(1) / 2;
    let source_path = format!("C:/Evidence/bench/clip_{midpoint:08}.mp4");
    record_query(
        &mut query_count,
        &mut max_query_ms,
        &mut query_rows_returned,
        || {
            let found: String = conn
                .query_row(
                    "SELECT id FROM videos WHERE source_path = ?1",
                    params![source_path],
                    |row| row.get(0),
                )
                .map_err(|err| format!("failed benchmark source lookup query: {err}"))?;
            Ok(usize::from(!found.is_empty()))
        },
    )?;

    let sha = format!("benchmark_hash_{midpoint:08}");
    record_query(
        &mut query_count,
        &mut max_query_ms,
        &mut query_rows_returned,
        || {
            let found: String = conn
                .query_row(
                    "SELECT id FROM videos WHERE sha256 = ?1",
                    params![sha],
                    |row| row.get(0),
                )
                .map_err(|err| format!("failed benchmark sha256 lookup query: {err}"))?;
            Ok(usize::from(!found.is_empty()))
        },
    )?;

    record_query(
        &mut query_count,
        &mut max_query_ms,
        &mut query_rows_returned,
        || {
            count_query_rows(
                conn,
                "SELECT id FROM videos WHERE extension = ?1 \
                 ORDER BY modified_unix DESC LIMIT 100",
                params!["mp4"],
                "timeline",
            )
        },
    )?;

    record_query(
        &mut query_count,
        &mut max_query_ms,
        &mut query_rows_returned,
        || {
            count_query_rows(
                conn,
                "SELECT id FROM videos WHERE ffprobe_ok = ?1 \
                 ORDER BY modified_unix ASC, id ASC LIMIT 100",
                params![0],
                "inventory validation candidates",
            )
        },
    )?;

    record_query(
        &mut query_count,
        &mut max_query_ms,
        &mut query_rows_returned,
        || {
            count_query_rows(
                conn,
                "SELECT id FROM videos WHERE hash_status = ?1 ORDER BY id LIMIT 100",
                params!["benchmark"],
                "inventory hash state",
            )
        },
    )?;

    record_query(
        &mut query_count,
        &mut max_query_ms,
        &mut query_rows_returned,
        || {
            count_query_rows(
                conn,
                "SELECT id FROM videos WHERE relative_path >= ?1 AND relative_path < ?2 \
                 ORDER BY relative_path LIMIT 100",
                params!["clip_000000", "clip_000001"],
                "inventory path prefix",
            )
        },
    )?;

    record_query(
        &mut query_count,
        &mut max_query_ms,
        &mut query_rows_returned,
        || {
            count_query_rows(
                conn,
                "SELECT id FROM videos WHERE modified_unix >= ?1 \
                 ORDER BY modified_unix ASC LIMIT 100",
                params![0],
                "inventory recent since",
            )
        },
    )?;

    Ok((query_count, max_query_ms, query_rows_returned))
}

fn count_query_rows<P: rusqlite::Params>(
    conn: &Connection,
    sql: &str,
    params: P,
    label: &str,
) -> Result<usize, String> {
    let mut stmt = conn
        .prepare(sql)
        .map_err(|err| format!("failed to prepare benchmark {label} query: {err}"))?;
    let rows = stmt
        .query_map(params, |row| row.get::<_, String>(0))
        .map_err(|err| format!("failed benchmark {label} query: {err}"))?;
    let mut count = 0usize;
    for row in rows {
        row.map_err(|err| format!("failed benchmark {label} row read: {err}"))?;
        count += 1;
    }
    Ok(count)
}

fn record_query(
    query_count: &mut usize,
    max_query_ms: &mut u128,
    query_rows_returned: &mut usize,
    run: impl FnOnce() -> Result<usize, String>,
) -> Result<(), String> {
    let started = Instant::now();
    let rows = run()?;
    *query_count += 1;
    *max_query_ms = (*max_query_ms).max(started.elapsed().as_millis());
    *query_rows_returned += rows;
    Ok(())
}
