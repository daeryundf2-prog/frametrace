use crate::case_db::{self, DbBenchmarkQueryPlan};
use crate::qa::QaReport;
use crate::util::{json_escape, write_text};
use std::path::Path;

const PERFORMANCE_ROWS_PER_MINUTE_TARGET: u128 = 50_000;
const PERFORMANCE_QUERY_LATENCY_TARGET_MS: u128 = 2_000;

pub fn performance_report(output_dir: &Path, rows: usize) -> Result<QaReport, String> {
    let result = case_db::benchmark_case_db(output_dir, rows)?;
    let json_path = output_dir.join("performance-report.json");
    let markdown_path = output_dir.join("performance-report.md");
    let rows_per_minute = if result.elapsed_ms == 0 {
        rows as u128 * 60_000
    } else {
        rows as u128 * 60_000 / result.elapsed_ms
    };
    let full_scan_count = count_full_video_scans(&result.query_plans);
    let passed = rows_per_minute >= PERFORMANCE_ROWS_PER_MINUTE_TARGET
        && result.max_query_ms <= PERFORMANCE_QUERY_LATENCY_TARGET_MS
        && full_scan_count == 0;

    write_text(
        &json_path,
        &performance_json(
            passed,
            &result,
            rows_per_minute,
            full_scan_count,
            &markdown_path,
        ),
    )
    .map_err(|err| format!("failed to write performance report: {err}"))?;
    write_text(
        &markdown_path,
        &performance_markdown(passed, &result, rows_per_minute, full_scan_count),
    )
    .map_err(|err| format!("failed to write performance markdown: {err}"))?;
    if passed {
        Ok(QaReport {
            report_path: json_path,
            passed,
        })
    } else {
        Err(format!(
            "performance QA failed: rows_per_minute={rows_per_minute}, target={PERFORMANCE_ROWS_PER_MINUTE_TARGET}, max_query_ms={}, query_latency_target_ms={PERFORMANCE_QUERY_LATENCY_TARGET_MS}, full_scan_count={full_scan_count}",
            result.max_query_ms
        ))
    }
}

fn count_full_video_scans(query_plans: &[DbBenchmarkQueryPlan]) -> usize {
    query_plans
        .iter()
        .filter(|plan| {
            let detail = plan.detail.to_ascii_uppercase();
            detail.contains("SCAN VIDEOS")
        })
        .count()
}

fn performance_json(
    passed: bool,
    result: &case_db::DbBenchmarkResult,
    rows_per_minute: u128,
    full_scan_count: usize,
    markdown_path: &Path,
) -> String {
    format!(
        "{{\n  \"schema_version\": 1,\n  \"qa_type\": \"performance\",\n  \"passed\": {},\n  \"rows\": {},\n  \"elapsed_ms\": {},\n  \"rows_per_minute\": {},\n  \"rows_per_minute_target\": {},\n  \"query_count\": {},\n  \"max_query_ms\": {},\n  \"query_latency_target_ms\": {},\n  \"query_rows_returned\": {},\n  \"query_plan_full_scan_count\": {},\n  \"query_plans\": {},\n  \"markdown_path\": \"{}\",\n  \"database_path\": \"{}\"\n}}\n",
        passed,
        result.rows,
        result.elapsed_ms,
        rows_per_minute,
        PERFORMANCE_ROWS_PER_MINUTE_TARGET,
        result.query_count,
        result.max_query_ms,
        PERFORMANCE_QUERY_LATENCY_TARGET_MS,
        result.query_rows_returned,
        full_scan_count,
        query_plans_json(&result.query_plans),
        json_escape(&markdown_path.to_string_lossy()),
        json_escape(&result.path.to_string_lossy())
    )
}

fn query_plans_json(query_plans: &[DbBenchmarkQueryPlan]) -> String {
    let items = query_plans
        .iter()
        .map(|plan| {
            format!(
                "{{\"label\":\"{}\",\"sql\":\"{}\",\"detail\":\"{}\"}}",
                json_escape(&plan.label),
                json_escape(&plan.sql),
                json_escape(&plan.detail)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{items}]")
}

fn performance_markdown(
    passed: bool,
    result: &case_db::DbBenchmarkResult,
    rows_per_minute: u128,
    full_scan_count: usize,
) -> String {
    let mut text = String::from("# Performance Report\n\n");
    text.push_str(&format!(
        "- Status: {}\n",
        if passed { "PASS" } else { "FAIL" }
    ));
    text.push_str(&format!("- Rows: {}\n", result.rows));
    text.push_str(&format!("- Elapsed ms: {}\n", result.elapsed_ms));
    text.push_str(&format!(
        "- Rows per minute: {rows_per_minute} (target: {PERFORMANCE_ROWS_PER_MINUTE_TARGET})\n"
    ));
    text.push_str(&format!(
        "- Max indexed query ms: {} (target: {PERFORMANCE_QUERY_LATENCY_TARGET_MS})\n",
        result.max_query_ms
    ));
    text.push_str(&format!("- Query plan full scans: {full_scan_count}\n"));
    text.push_str(&format!("- Database: `{}`\n", result.path.display()));
    text.push_str("\n## Query Plans\n\n");
    for plan in &result.query_plans {
        text.push_str(&format!(
            "### {}\n\n```sql\nEXPLAIN QUERY PLAN {}\n```\n\n```text\n{}\n```\n\n",
            plan.label, plan.sql, plan.detail
        ));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::performance_report;
    use std::fs;

    #[test]
    fn performance_report_writes_query_plan_evidence() {
        let root = std::env::temp_dir().join(format!(
            "frametrace-performance-query-plan-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);

        let report = performance_report(&root, 1_000).unwrap();
        let json = fs::read_to_string(&report.report_path).unwrap();
        let markdown = fs::read_to_string(root.join("performance-report.md")).unwrap();

        assert!(report.passed);
        assert!(json.contains("\"query_plan_full_scan_count\": 0"));
        assert!(json.contains("\"query_plans\""));
        assert!(markdown.contains("## Query Plans"));
        assert!(markdown.contains("EXPLAIN"));

        let _ = fs::remove_dir_all(root);
    }
}
