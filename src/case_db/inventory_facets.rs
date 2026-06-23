use super::inventory_query::{nonnegative_count, open_inventory_db, scalar_count};
use super::inventory_types::{InventoryFacet, InventoryFacetCounts};
use std::path::Path;

pub fn inventory_facets(case_dir: &Path) -> Result<InventoryFacetCounts, String> {
    let Some(conn) = open_inventory_db(case_dir)? else {
        return Ok(empty_facets());
    };
    let total_rows = scalar_count(&conn, "SELECT COUNT(*) FROM videos", [])?;
    let confirmed_count = scalar_count(
        &conn,
        "SELECT COUNT(*) FROM videos WHERE ffprobe_ok = 1",
        [],
    )?;
    let candidate_count = scalar_count(
        &conn,
        "SELECT COUNT(*) FROM videos WHERE ffprobe_ok = 0",
        [],
    )?;
    Ok(InventoryFacetCounts {
        total_rows,
        confirmed_count,
        candidate_count,
        by_extension: query_facets(
            &conn,
            "SELECT extension, COUNT(*) FROM videos \
             GROUP BY extension ORDER BY COUNT(*) DESC, extension ASC LIMIT 50",
        )?,
        by_source: query_facets(
            &conn,
            "SELECT source_path, COUNT(*) FROM videos \
             GROUP BY source_path ORDER BY COUNT(*) DESC, source_path ASC LIMIT 50",
        )?,
        by_type: static_facet("video", total_rows),
        by_parser_lane: query_facets(
            &conn,
            r#"SELECT CASE WHEN source_profile_json LIKE '%"parser":"benchmark"%' THEN 'benchmark'
               ELSE 'video-index' END, COUNT(*) FROM videos
               GROUP BY 1 ORDER BY COUNT(*) DESC, 1 ASC LIMIT 50"#,
        )?,
        by_validation_state: query_facets(
            &conn,
            "SELECT CASE WHEN ffprobe_ok = 1 THEN 'ffprobe-video-stream-confirmed' \
             ELSE 'candidate-unvalidated' END, COUNT(*) FROM videos \
             GROUP BY 1 ORDER BY COUNT(*) DESC, 1 ASC",
        )?,
        by_review_state: static_facet("unreviewed", total_rows),
        by_report_state: static_facet("not-selected", total_rows),
        by_hash_state: query_facets(
            &conn,
            "SELECT hash_status, COUNT(*) FROM videos \
             GROUP BY hash_status ORDER BY COUNT(*) DESC, hash_status ASC LIMIT 50",
        )?,
    })
}

fn empty_facets() -> InventoryFacetCounts {
    InventoryFacetCounts {
        total_rows: 0,
        confirmed_count: 0,
        candidate_count: 0,
        by_extension: Vec::new(),
        by_source: Vec::new(),
        by_type: Vec::new(),
        by_parser_lane: Vec::new(),
        by_validation_state: Vec::new(),
        by_review_state: Vec::new(),
        by_report_state: Vec::new(),
        by_hash_state: Vec::new(),
    }
}

fn query_facets(conn: &rusqlite::Connection, sql: &str) -> Result<Vec<InventoryFacet>, String> {
    let mut stmt = conn
        .prepare(sql)
        .map_err(|err| format!("failed to prepare inventory facet query: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(InventoryFacet {
                value: row.get(0)?,
                count: nonnegative_count(row.get::<_, i64>(1)?),
            })
        })
        .map_err(|err| format!("failed to query inventory facets: {err}"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|err| format!("failed to read inventory facet: {err}"))?);
    }
    Ok(out)
}

fn static_facet(value: &str, count: usize) -> Vec<InventoryFacet> {
    if count == 0 {
        Vec::new()
    } else {
        vec![InventoryFacet {
            value: value.to_string(),
            count,
        }]
    }
}
