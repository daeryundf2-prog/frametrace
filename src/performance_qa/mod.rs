mod compatibility;
mod render;
mod survival;
#[cfg(test)]
mod tests;

use crate::case_db::{self, DbBenchmarkQueryPlan};
use crate::model::CaseManifest;
use crate::qa::QaReport;
use crate::resource_monitor::{ResourceMonitor, ResourceUsage};
use crate::util::{create_case_layout, now_unix, write_text};
use std::path::Path;

const PERFORMANCE_ROWS_PER_MINUTE_TARGET: u128 = 50_000;
const PERFORMANCE_QUERY_LATENCY_TARGET_MS: u128 = 2_000;
const GIB: u64 = 1024 * 1024 * 1024;
const PERFORMANCE_100K_MAX_RSS_TARGET_BYTES: u64 = (GIB * 3) / 2;
const PERFORMANCE_1M_MAX_RSS_TARGET_BYTES: u64 = (GIB * 7) / 2;
const CPU_GATE_ENFORCED_FOR_SQLITE_BENCHMARK: bool = false;

pub fn performance_report(output_dir: &Path, rows: usize) -> Result<QaReport, String> {
    let monitor = ResourceMonitor::start();
    prepare_benchmark_case(output_dir)?;
    let result = case_db::benchmark_case_db(output_dir, rows)?;
    let survival = survival::large_case_survival_evidence(output_dir, rows)?;
    let resources = monitor.finish(
        max_rss_target_for_rows(rows),
        CPU_GATE_ENFORCED_FOR_SQLITE_BENCHMARK,
    );
    let json_path = output_dir.join("performance-report.json");
    let markdown_path = output_dir.join("performance-report.md");
    let rows_per_minute = rows_per_minute(rows, result.elapsed_ms);
    let full_scan_count = count_full_video_scans(&result.query_plans);
    let passed = performance_passed(
        &result,
        &resources,
        rows_per_minute,
        full_scan_count,
        &survival,
    );

    write_text(
        &json_path,
        &render::performance_json(
            passed,
            &result,
            &resources,
            rows_per_minute,
            full_scan_count,
            &markdown_path,
            &survival,
        ),
    )
    .map_err(|err| format!("failed to write performance report: {err}"))?;
    write_text(
        &markdown_path,
        &render::performance_markdown(
            passed,
            &result,
            &resources,
            rows_per_minute,
            full_scan_count,
            &survival,
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
            "performance QA failed: rows_per_minute={rows_per_minute}, target={PERFORMANCE_ROWS_PER_MINUTE_TARGET}, max_query_ms={}, survival_max_query_ms={}, query_latency_target_ms={PERFORMANCE_QUERY_LATENCY_TARGET_MS}, full_scan_count={full_scan_count}, max_rss_bytes={}, max_rss_target_bytes={}, cpu_average_percent={}",
            result.max_query_ms,
            survival.max_query_ms,
            render::optional_u64_json(resources.max_rss_bytes),
            resources.max_rss_target_bytes,
            render::optional_f64_json(resources.average_cpu_percent)
        ))
    }
}

fn prepare_benchmark_case(output_dir: &Path) -> Result<(), String> {
    create_case_layout(output_dir).map_err(|err| {
        format!(
            "failed to create benchmark case layout {}: {err}",
            output_dir.display()
        )
    })?;
    let created_unix = now_unix()?;
    let manifest = CaseManifest {
        schema_version: 1,
        case_id: format!("FT-PERF-{created_unix}"),
        title: "FrameTrace performance benchmark case".to_string(),
        created_unix,
        tool_name: env!("CARGO_PKG_NAME").to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        operator: Some("qa-performance".to_string()),
        host: None,
        device_id: None,
        device_serial: None,
        write_protect: None,
        acquisition_tool: Some("synthetic benchmark".to_string()),
        evidence_hash: None,
        notes: Some("Synthetic large-case survival profile.".to_string()),
    };
    write_text(&output_dir.join("case.json"), &manifest.to_json())
        .map_err(|err| format!("failed to write benchmark case manifest: {err}"))
}

fn rows_per_minute(rows: usize, elapsed_ms: u128) -> u128 {
    if elapsed_ms == 0 {
        rows as u128 * 60_000
    } else {
        rows as u128 * 60_000 / elapsed_ms
    }
}

fn max_rss_target_for_rows(rows: usize) -> u64 {
    if rows >= 1_000_000 {
        PERFORMANCE_1M_MAX_RSS_TARGET_BYTES
    } else if rows >= 100_000 {
        PERFORMANCE_100K_MAX_RSS_TARGET_BYTES
    } else {
        GIB
    }
}

fn performance_passed(
    result: &case_db::DbBenchmarkResult,
    resources: &ResourceUsage,
    rows_per_minute: u128,
    full_scan_count: usize,
    survival: &survival::LargeCaseSurvivalEvidence,
) -> bool {
    rows_per_minute >= PERFORMANCE_ROWS_PER_MINUTE_TARGET
        && result.max_query_ms <= PERFORMANCE_QUERY_LATENCY_TARGET_MS
        && survival.max_query_ms <= PERFORMANCE_QUERY_LATENCY_TARGET_MS
        && full_scan_count == 0
        && resources.metrics_available()
        && resources.rss_passed()
        && resources.cpu_passed()
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
