use super::PERFORMANCE_QUERY_LATENCY_TARGET_MS;
use super::compatibility::{CompatibilityExportEvidence, compatibility_export_evidence};
use crate::case_db::{self, INVENTORY_MAX_PAGE_SIZE, InventoryListQuery, InventoryPage};
use crate::distributable_redaction::RedactionPolicy;
use crate::util::write_text;
use crate::{audit, html_report, report, review_bundle, workstation};
use std::path::Path;
use std::time::Instant;

#[derive(Debug)]
pub(super) struct LargeCaseSurvivalEvidence {
    pub(super) report_generation_ms: u128,
    pub(super) review_bundle_generation_ms: u128,
    pub(super) inventory_timings: Vec<InventoryTiming>,
    pub(super) compatibility_exports: CompatibilityExportEvidence,
    pub(super) full_json_load_denial: FullJsonLoadDenial,
    pub(super) max_query_ms: u128,
}

#[derive(Debug)]
pub(super) struct InventoryTiming {
    pub(super) operation: &'static str,
    pub(super) duration_ms: u128,
    pub(super) total_rows: usize,
    pub(super) returned_rows: usize,
    pub(super) truncated: bool,
}

#[derive(Debug)]
pub(super) struct FullJsonLoadDenial {
    pub(super) status: &'static str,
    pub(super) full_json_load_allowed: bool,
    pub(super) evidence: String,
}

pub(super) fn large_case_survival_evidence(
    case_dir: &Path,
    rows: usize,
) -> Result<LargeCaseSurvivalEvidence, String> {
    let report_generation_ms = timed_report_generation(case_dir)?;
    let review_bundle_generation_ms = timed_review_generation(case_dir)?;
    let inventory_timings = inventory_query_timings(case_dir)?;
    let compatibility_exports = compatibility_export_evidence(case_dir, rows)?;
    let full_json_load_denial = full_json_load_denial(case_dir)?;
    let max_query_ms = inventory_timings
        .iter()
        .map(|timing| timing.duration_ms)
        .max()
        .unwrap_or(0);
    Ok(LargeCaseSurvivalEvidence {
        report_generation_ms,
        review_bundle_generation_ms,
        inventory_timings,
        compatibility_exports,
        full_json_load_denial,
        max_query_ms,
    })
}

fn timed_report_generation(case_dir: &Path) -> Result<u128, String> {
    let started = Instant::now();
    let index_json = case_db::bounded_report_index_json(case_dir)?
        .ok_or_else(|| "benchmark case db missing for report generation".to_string())?;
    let manifest_json = std::fs::read_to_string(case_dir.join("case.json"))
        .map_err(|err| format!("failed to read benchmark case manifest: {err}"))?;
    let audit_chain_status =
        audit::audit_chain_statuses_json(&audit::media_audit_chain_statuses(case_dir));
    let html = report::render_case_report(&report::ReportInputs {
        manifest_json: &manifest_json,
        index_json: &index_json,
        export_log_jsonl: "",
        proxy_log_jsonl: "",
        thumbnail_log_jsonl: "",
        frame_log_jsonl: "",
        carve_log_jsonl: "",
        filesystem_log_jsonl: "",
        validation_log_jsonl: "",
        audit_chain_status_json: &audit_chain_status,
    });
    write_text(&case_dir.join("reports/case-report.html"), &html)
        .map_err(|err| format!("failed to write benchmark case report: {err}"))?;
    Ok(started.elapsed().as_millis())
}

fn timed_review_generation(case_dir: &Path) -> Result<u128, String> {
    let started = Instant::now();
    let index_json = review_bundle::bounded_review_index_json_with_policy(
        case_dir,
        RedactionPolicy::redacted(),
    )?;
    let manifest_json = std::fs::read_to_string(case_dir.join("case.json"))
        .map_err(|err| format!("failed to read benchmark case manifest: {err}"))?;
    let review_html = html_report::render_review_html(&manifest_json, &index_json);
    write_text(&case_dir.join("review/index.html"), &review_html)
        .map_err(|err| format!("failed to write benchmark review html: {err}"))?;
    let audit_chain_status =
        audit::audit_chain_statuses_json(&audit::media_audit_chain_statuses(case_dir));
    let evidence_viewer =
        html_report::render_evidence_viewer_html(html_report::EvidenceViewerInputs {
            manifest_json: &manifest_json,
            index_json: &index_json,
            carve_log_jsonl: "",
            filesystem_log_jsonl: "",
            validation_log_jsonl: "",
            export_log_jsonl: "",
            proxy_log_jsonl: "",
            thumbnail_log_jsonl: "",
            frame_log_jsonl: "",
            audit_chain_status_json: &audit_chain_status,
        });
    write_text(
        &case_dir.join("review/evidence-viewer.html"),
        &evidence_viewer,
    )
    .map_err(|err| format!("failed to write benchmark evidence viewer: {err}"))?;
    Ok(started.elapsed().as_millis())
}

fn inventory_query_timings(case_dir: &Path) -> Result<Vec<InventoryTiming>, String> {
    let mut timings = Vec::new();
    timings.push(timed_inventory_page(
        "inventory-list",
        case_db::list_inventory(
            case_dir,
            &InventoryListQuery {
                page_size: INVENTORY_MAX_PAGE_SIZE,
                ..InventoryListQuery::default()
            },
        )?,
    ));
    timings.push(timed_inventory_page(
        "inventory-sort-size-desc",
        case_db::list_inventory(
            case_dir,
            &InventoryListQuery {
                page_size: INVENTORY_MAX_PAGE_SIZE,
                sort: Some("size-desc".to_string()),
                ..InventoryListQuery::default()
            },
        )?,
    ));
    timings.push(timed_inventory_page(
        "inventory-search-prefix",
        case_db::search_inventory(case_dir, "clip_000000", INVENTORY_MAX_PAGE_SIZE)?,
    ));
    let started = Instant::now();
    let facets = case_db::inventory_facets(case_dir)?;
    timings.push(InventoryTiming {
        operation: "inventory-facets",
        duration_ms: started.elapsed().as_millis(),
        total_rows: facets.total_rows,
        returned_rows: facets.by_extension.len()
            + facets.by_source.len()
            + facets.by_type.len()
            + facets.by_parser_lane.len()
            + facets.by_validation_state.len()
            + facets.by_review_state.len()
            + facets.by_report_state.len()
            + facets.by_hash_state.len(),
        truncated: false,
    });
    Ok(timings)
}

fn timed_inventory_page(operation: &'static str, page: InventoryPage) -> InventoryTiming {
    InventoryTiming {
        operation,
        duration_ms: page.duration_ms,
        total_rows: page.total_rows,
        returned_rows: page.rows.len(),
        truncated: page.truncated,
    }
}

fn full_json_load_denial(case_dir: &Path) -> Result<FullJsonLoadDenial, String> {
    let status_json = workstation::workstation_status_json(case_dir)?;
    let denied = status_json.contains("\"full_json_load_allowed\":false")
        && status_json.contains("\"large_case_full_json_load_allowed\":false")
        && status_json.contains("\"inventory_transport\":\"paged-sqlite-query\"");
    if !denied {
        return Err("workstation status does not deny large-case full JSON load".to_string());
    }
    Ok(FullJsonLoadDenial {
        status: "DENIED",
        full_json_load_allowed: false,
        evidence: "workstation-status inventory_transport=paged-sqlite-query and full_json_load_allowed=false".to_string(),
    })
}

pub(super) fn query_latency_target_ms() -> u128 {
    PERFORMANCE_QUERY_LATENCY_TARGET_MS
}
