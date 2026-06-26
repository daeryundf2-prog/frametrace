use super::compatibility::CompatibilityExportEvidence;
use super::survival::{
    FullJsonLoadDenial, InventoryTiming, LargeCaseSurvivalEvidence, query_latency_target_ms,
};
use super::{PERFORMANCE_ROWS_PER_MINUTE_TARGET, case_db};
use crate::resource_monitor::ResourceUsage;
use crate::util::json_escape;
use std::path::Path;

pub(super) fn performance_json(
    passed: bool,
    result: &case_db::DbBenchmarkResult,
    resources: &ResourceUsage,
    rows_per_minute: u128,
    full_scan_count: usize,
    markdown_path: &Path,
    survival: &LargeCaseSurvivalEvidence,
) -> String {
    format!(
        "{{\n  \"schema_version\": 1,\n  \"qa_type\": \"performance\",\n  \"passed\": {},\n  \"rows\": {},\n  \"elapsed_ms\": {},\n  \"rows_per_minute\": {},\n  \"rows_per_minute_target\": {},\n  \"query_count\": {},\n  \"max_query_ms\": {},\n  \"query_latency_target_ms\": {},\n  \"query_rows_returned\": {},\n  \"query_plan_full_scan_count\": {},\n  \"query_plans\": {},\n  \"large_case_survival\": {},\n  \"max_rss_bytes\": {},\n  \"max_rss_target_bytes\": {},\n  \"cpu_average_percent\": {},\n  \"cpu_target_percent\": {},\n  \"cpu_target_enforced\": {},\n  \"cpu_gate_status\": \"{}\",\n  \"resource_sample_count\": {},\n  \"markdown_path\": \"{}\",\n  \"database_path\": \"{}\"\n}}\n",
        passed,
        result.rows,
        result.elapsed_ms,
        rows_per_minute,
        PERFORMANCE_ROWS_PER_MINUTE_TARGET,
        result.query_count,
        result.max_query_ms,
        query_latency_target_ms(),
        result.query_rows_returned,
        full_scan_count,
        query_plans_json(&result.query_plans),
        large_case_survival_json(survival),
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

fn large_case_survival_json(survival: &LargeCaseSurvivalEvidence) -> String {
    format!(
        "{{\"report_generation_ms\":{},\"review_bundle_generation_ms\":{},\
\"max_query_ms\":{},\"query_latency_target_ms\":{},\"inventory_query_timings\":{},\
\"compatibility_exports\":{},\"full_json_load_denial\":{}}}",
        survival.report_generation_ms,
        survival.review_bundle_generation_ms,
        survival.max_query_ms,
        query_latency_target_ms(),
        inventory_timings_json(&survival.inventory_timings),
        compatibility_exports_json(&survival.compatibility_exports),
        full_json_load_denial_json(&survival.full_json_load_denial)
    )
}

fn inventory_timings_json(timings: &[InventoryTiming]) -> String {
    let items = timings
        .iter()
        .map(|timing| {
            format!(
                "{{\"operation\":\"{}\",\"duration_ms\":{},\"total_rows\":{},\
\"returned_rows\":{},\"truncated\":{}}}",
                json_escape(timing.operation),
                timing.duration_ms,
                timing.total_rows,
                timing.returned_rows,
                timing.truncated
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{items}]")
}

fn compatibility_exports_json(exports: &CompatibilityExportEvidence) -> String {
    format!(
        "{{\"jsonl_path\":\"{}\",\"jsonl_rows\":{},\"jsonl_elapsed_ms\":{},\
\"jsonl_rows_per_minute\":{},\"tsv_path\":\"{}\",\"tsv_rows\":{},\
\"tsv_elapsed_ms\":{},\"tsv_rows_per_minute\":{},\"manifest_path\":\"{}\",\
\"manifest_selected_count\":{},\"manifest_elapsed_ms\":{}}}",
        json_escape(&exports.jsonl_path),
        exports.jsonl_rows,
        exports.jsonl_elapsed_ms,
        exports.jsonl_rows_per_minute,
        json_escape(&exports.tsv_path),
        exports.tsv_rows,
        exports.tsv_elapsed_ms,
        exports.tsv_rows_per_minute,
        json_escape(&exports.manifest_path),
        exports.manifest_selected_count,
        exports.manifest_elapsed_ms
    )
}

fn full_json_load_denial_json(denial: &FullJsonLoadDenial) -> String {
    format!(
        "{{\"status\":\"{}\",\"full_json_load_allowed\": {},\"evidence\":\"{}\"}}",
        json_escape(denial.status),
        denial.full_json_load_allowed,
        json_escape(&denial.evidence)
    )
}

fn query_plans_json(query_plans: &[case_db::DbBenchmarkQueryPlan]) -> String {
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

pub(super) fn performance_markdown(
    passed: bool,
    result: &case_db::DbBenchmarkResult,
    resources: &ResourceUsage,
    rows_per_minute: u128,
    full_scan_count: usize,
    survival: &LargeCaseSurvivalEvidence,
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
        "- Max indexed query ms: {} (target: {})\n",
        result.max_query_ms,
        query_latency_target_ms()
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
    text.push_str("\n## Large-Case Survival\n\n");
    text.push_str(&format!(
        "- Report generation ms: {}\n",
        survival.report_generation_ms
    ));
    text.push_str(&format!(
        "- Review bundle generation ms: {}\n",
        survival.review_bundle_generation_ms
    ));
    text.push_str(&format!(
        "- Max inventory query ms: {} (target: {})\n",
        survival.max_query_ms,
        query_latency_target_ms()
    ));
    text.push_str(&format!(
        "- Compatibility JSONL rows: {} ({}/min)\n",
        survival.compatibility_exports.jsonl_rows,
        survival.compatibility_exports.jsonl_rows_per_minute
    ));
    text.push_str(&format!(
        "- Compatibility TSV rows: {} ({}/min)\n",
        survival.compatibility_exports.tsv_rows, survival.compatibility_exports.tsv_rows_per_minute
    ));
    text.push_str(&format!(
        "- Full JSON load denial: {}\n",
        survival.full_json_load_denial.status
    ));
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

pub(super) fn optional_u64_json(value: Option<u64>) -> String {
    value.map_or_else(|| "null".to_string(), |value| value.to_string())
}

pub(super) fn optional_f64_json(value: Option<f64>) -> String {
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
