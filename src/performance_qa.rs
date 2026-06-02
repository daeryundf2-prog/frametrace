use crate::case_db::{self, DbBenchmarkQueryPlan};
use crate::qa::QaReport;
use crate::resource_monitor::{ResourceMonitor, ResourceUsage};
use crate::util::{json_escape, write_text};
use std::path::Path;

const PERFORMANCE_ROWS_PER_MINUTE_TARGET: u128 = 50_000;
const PERFORMANCE_QUERY_LATENCY_TARGET_MS: u128 = 2_000;
const GIB: u64 = 1024 * 1024 * 1024;
const CPU_GATE_ENFORCED_FOR_SQLITE_BENCHMARK: bool = false;

pub fn performance_report(output_dir: &Path, rows: usize) -> Result<QaReport, String> {
    let monitor = ResourceMonitor::start();
    let result = case_db::benchmark_case_db(output_dir, rows)?;
    let resources = monitor.finish(
        max_rss_target_for_rows(rows),
        CPU_GATE_ENFORCED_FOR_SQLITE_BENCHMARK,
    );
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
        && full_scan_count == 0
        && resources.metrics_available()
        && resources.rss_passed()
        && resources.cpu_passed();

    write_text(
        &json_path,
        &performance_json(
            passed,
            &result,
            &resources,
            rows_per_minute,
            full_scan_count,
            &markdown_path,
        ),
    )
    .map_err(|err| format!("failed to write performance report: {err}"))?;
    write_text(
        &markdown_path,
        &performance_markdown(
            passed,
            &result,
            &resources,
            rows_per_minute,
            full_scan_count,
        ),
    )
    .map_err(|err| format!("failed to write performance markdown: {err}"))?;
    if passed {
        Ok(QaReport {
            report_path: json_path,
            passed,
        })
    } else {
        Err(format!(
            "performance QA failed: rows_per_minute={rows_per_minute}, target={PERFORMANCE_ROWS_PER_MINUTE_TARGET}, max_query_ms={}, query_latency_target_ms={PERFORMANCE_QUERY_LATENCY_TARGET_MS}, full_scan_count={full_scan_count}, max_rss_bytes={}, max_rss_target_bytes={}, cpu_average_percent={}",
            result.max_query_ms,
            optional_u64_json(resources.max_rss_bytes),
            resources.max_rss_target_bytes,
            optional_f64_json(resources.average_cpu_percent)
        ))
    }
}

fn max_rss_target_for_rows(rows: usize) -> u64 {
    if rows >= 1_000_000 {
        4 * GIB
    } else if rows >= 100_000 {
        (GIB * 5) / 2
    } else {
        GIB
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
    resources: &ResourceUsage,
    rows_per_minute: u128,
    full_scan_count: usize,
    markdown_path: &Path,
) -> String {
    format!(
        "{{\n  \"schema_version\": 1,\n  \"qa_type\": \"performance\",\n  \"passed\": {},\n  \"rows\": {},\n  \"elapsed_ms\": {},\n  \"rows_per_minute\": {},\n  \"rows_per_minute_target\": {},\n  \"query_count\": {},\n  \"max_query_ms\": {},\n  \"query_latency_target_ms\": {},\n  \"query_rows_returned\": {},\n  \"query_plan_full_scan_count\": {},\n  \"query_plans\": {},\n  \"max_rss_bytes\": {},\n  \"max_rss_target_bytes\": {},\n  \"cpu_average_percent\": {},\n  \"cpu_target_percent\": {},\n  \"cpu_target_enforced\": {},\n  \"cpu_gate_status\": \"{}\",\n  \"resource_sample_count\": {},\n  \"markdown_path\": \"{}\",\n  \"database_path\": \"{}\"\n}}\n",
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
        optional_u64_json(resources.max_rss_bytes),
        resources.max_rss_target_bytes,
        optional_f64_json(resources.average_cpu_percent),
        resources.cpu_target_percent,
        resources.cpu_target_enforced,
        cpu_gate_status(resources),
        resources.sample_count,
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
    resources: &ResourceUsage,
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
    text.push_str(&format!(
        "- Max RSS: {} (target: {})\n",
        optional_bytes_text(resources.max_rss_bytes),
        bytes_text(resources.max_rss_target_bytes)
    ));
    text.push_str(&format!(
        "- Average CPU: {} (target: {}%, enforced: {})\n",
        optional_percent_text(resources.average_cpu_percent),
        resources.cpu_target_percent,
        resources.cpu_target_enforced
    ));
    text.push_str(&format!("- Resource samples: {}\n", resources.sample_count));
    text.push_str(&format!("- Database: `{}`\n", result.path.display()));
    text.push_str("\n## Resource Metrics\n\n");
    text.push_str("| Metric | Value | Target | Status |\n");
    text.push_str("| --- | ---: | ---: | --- |\n");
    text.push_str(&format!(
        "| Max RSS | {} | {} | {} |\n",
        optional_bytes_text(resources.max_rss_bytes),
        bytes_text(resources.max_rss_target_bytes),
        if resources.rss_passed() {
            "PASS"
        } else {
            "FAIL"
        }
    ));
    text.push_str(&format!(
        "| Average CPU | {} | {}% | {} |\n",
        optional_percent_text(resources.average_cpu_percent),
        resources.cpu_target_percent,
        cpu_gate_status(resources)
    ));
    text.push_str("\n## Query Plans\n\n");
    for plan in &result.query_plans {
        text.push_str(&format!(
            "### {}\n\n```sql\nEXPLAIN QUERY PLAN {}\n```\n\n```text\n{}\n```\n\n",
            plan.label, plan.sql, plan.detail
        ));
    }
    text
}

fn optional_u64_json(value: Option<u64>) -> String {
    value.map_or_else(|| "null".to_string(), |value| value.to_string())
}

fn optional_f64_json(value: Option<f64>) -> String {
    value.map_or_else(|| "null".to_string(), |value| format!("{value:.2}"))
}

fn optional_bytes_text(value: Option<u64>) -> String {
    value.map_or_else(|| "unavailable".to_string(), bytes_text)
}

fn bytes_text(bytes: u64) -> String {
    format!("{:.2} MiB", bytes as f64 / 1024.0 / 1024.0)
}

fn optional_percent_text(value: Option<f64>) -> String {
    value.map_or_else(|| "unavailable".to_string(), |value| format!("{value:.2}%"))
}

fn cpu_gate_status(resources: &ResourceUsage) -> &'static str {
    match (resources.cpu_target_enforced, resources.average_cpu_percent) {
        (true, Some(_)) if resources.cpu_passed() => "PASS",
        (true, Some(_)) | (true, None) | (false, None) => "FAIL",
        (false, Some(_)) => "MEASURED_NOT_GATED",
    }
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
        assert!(json.contains("\"max_rss_bytes\""));
        assert!(json.contains("\"max_rss_target_bytes\""));
        assert!(json.contains("\"cpu_average_percent\""));
        assert!(json.contains("\"cpu_gate_status\""));
        assert!(markdown.contains("## Query Plans"));
        assert!(markdown.contains("## Resource Metrics"));
        assert!(markdown.contains("EXPLAIN"));

        let _ = fs::remove_dir_all(root);
    }
}
