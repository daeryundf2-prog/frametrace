use crate::audit;
use crate::case_db::{self, CaseDbSummary, InventoryFacet, InventoryFacetCounts};
use crate::runtime_readiness;
use crate::util::{compact_json_value_if_well_formed, json_escape, read_to_string};
use crate::windows_prerequisites;
use crate::workstation_contract;
use std::collections::BTreeMap;
use std::path::Path;

const ENGINE_COMMANDS_JSON: &str = concat!(
    "[\"init-case\",\"register-source\",\"scan-folder\",\"inspect-e01\",\"import-e01\",",
    "\"inspect-image\",\"recover-inode\",\"carve-file\",\"validate-artifact\",",
    "\"confirm-playback\",\"make-proxy\",\"make-thumbnail\",\"capture-frame\",",
    "\"export-video\",\"inventory\",\"inventory-bulk-preview\",",
    "\"inventory-export-manifest\",\"make-review\",\"make-report\",\"package-case\",",
    "\"qa release\"]"
);

#[derive(Debug, Clone, Default)]
struct ValidationSummary {
    total_events: usize,
    ffprobe_confirmed_count: usize,
    playback_confirmed_count: usize,
    failed_count: usize,
    status_counts: BTreeMap<String, usize>,
}

pub fn workstation_status_json(case_dir: &Path) -> Result<String, String> {
    let manifest_json = case_manifest_json(case_dir)?;
    let sqlite_json = sqlite_summary_json(case_dir)?;
    let inventory_json = inventory_summary_json(case_dir)?;
    let validation_json = validation_summary_json(case_dir);
    let audit_chain_json =
        audit::audit_chain_statuses_json(&audit::media_audit_chain_statuses(case_dir));
    let artifacts_json = generated_artifacts_json(case_dir);
    let runtime_readiness_json = runtime_readiness::runtime_readiness_json(case_dir)?;
    let windows_prerequisites_json = windows_prerequisites::status_json();

    Ok(format!(
        "{{\"schema_version\":1,\"view\":\"workstation-status\",\
\"engine_source_of_truth\":true,\"gui_durable_state_allowed\":false,\
\"case_dir\":\"{}\",\"case_manifest\":{},\"sqlite\":{},\"inventory\":{},\
\"validation\":{},\"audit_chain\":{},\"generated_artifacts\":{},\
\"runtime_readiness\":{},\"gui_data_adapter\":{},\"engine_commands\":{},\
\"windows_prerequisites\":{},\"winui_contract\":{}}}",
        json_escape(&case_dir.to_string_lossy()),
        manifest_json,
        sqlite_json,
        inventory_json,
        validation_json,
        audit_chain_json,
        artifacts_json,
        runtime_readiness_json,
        workstation_contract::gui_data_adapter_json(),
        ENGINE_COMMANDS_JSON,
        windows_prerequisites_json,
        workstation_contract::winui_contract_json()
    ))
}

fn case_manifest_json(case_dir: &Path) -> Result<String, String> {
    let manifest_path = case_dir.join("case.json");
    let raw = read_to_string(&manifest_path).map_err(|err| {
        format!(
            "failed to read case manifest {}: {err}",
            manifest_path.display()
        )
    })?;
    compact_json_value_if_well_formed(&raw).ok_or_else(|| {
        format!(
            "case manifest is not valid JSON: {}",
            manifest_path.display()
        )
    })
}

fn sqlite_summary_json(case_dir: &Path) -> Result<String, String> {
    match case_db::summarize_case_db(case_dir)? {
        Some(summary) => Ok(case_db_summary_json(&summary)),
        None => Ok(format!(
            "{{\"path\":\"{}\",\"exists\":false,\"video_count\":0,\
\"scan_run_count\":0,\"evidence_source_count\":0,\"job_count\":0,\
\"active_job_count\":0}}",
            json_escape(&case_db::case_db_path(case_dir).to_string_lossy())
        )),
    }
}

fn case_db_summary_json(summary: &CaseDbSummary) -> String {
    format!(
        "{{\"path\":\"{}\",\"exists\":true,\"video_count\":{},\
\"scan_run_count\":{},\"evidence_source_count\":{},\"job_count\":{},\
\"active_job_count\":{}}}",
        json_escape(&summary.path.to_string_lossy()),
        summary.video_count,
        summary.scan_run_count,
        summary.evidence_source_count,
        summary.job_count,
        summary.active_job_count
    )
}

fn inventory_summary_json(case_dir: &Path) -> Result<String, String> {
    let facets = case_db::inventory_facets(case_dir)?;
    Ok(format!(
        "{{\"transport\":\"sqlite-bounded-query\",\"full_json_load_allowed\":false,\
\"max_page_size\":{},\"facets\":{}}}",
        case_db::INVENTORY_MAX_PAGE_SIZE,
        facets_json(&facets)
    ))
}

fn validation_summary_json(case_dir: &Path) -> String {
    let log_path = case_dir.join("evidence/logs/validation-log.jsonl");
    let summary = validation_summary_from_jsonl(&read_to_string(&log_path).unwrap_or_default());
    format!(
        "{{\"log_path\":\"{}\",\"exists\":{},\"total_events\":{},\
\"ffprobe_video_stream_confirmed_count\":{},\"playback_confirmed_count\":{},\
\"failed_count\":{},\"candidate_requires_playback_confirmation\":true,\
\"ffprobe_and_playback_are_separate_states\":true,\"status_counts\":{}}}",
        json_escape(&log_path.to_string_lossy()),
        log_path.is_file(),
        summary.total_events,
        summary.ffprobe_confirmed_count,
        summary.playback_confirmed_count,
        summary.failed_count,
        status_counts_json(&summary.status_counts)
    )
}

fn validation_summary_from_jsonl(text: &str) -> ValidationSummary {
    let mut summary = ValidationSummary::default();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        summary.total_events += 1;
        let Some(status) = extract_json_string(line, "validation_status") else {
            continue;
        };
        *summary.status_counts.entry(status.clone()).or_insert(0) += 1;
        match status.as_str() {
            "ffprobe-video-stream-confirmed" => summary.ffprobe_confirmed_count += 1,
            "playback-confirmed" => summary.playback_confirmed_count += 1,
            "validation-failed" => summary.failed_count += 1,
            _ => {}
        }
    }
    summary
}

fn facets_json(facets: &InventoryFacetCounts) -> String {
    format!(
        "{{\"total_rows\":{},\"confirmed_count\":{},\"candidate_count\":{},\
\"by_extension\":{},\"by_source\":{},\"by_type\":{},\"by_parser_lane\":{},\
\"by_validation_state\":{},\"by_review_state\":{},\"by_report_state\":{},\
\"by_hash_state\":{}}}",
        facets.total_rows,
        facets.confirmed_count,
        facets.candidate_count,
        facet_array_json(&facets.by_extension),
        facet_array_json(&facets.by_source),
        facet_array_json(&facets.by_type),
        facet_array_json(&facets.by_parser_lane),
        facet_array_json(&facets.by_validation_state),
        facet_array_json(&facets.by_review_state),
        facet_array_json(&facets.by_report_state),
        facet_array_json(&facets.by_hash_state)
    )
}

fn facet_array_json(facets: &[InventoryFacet]) -> String {
    format!(
        "[{}]",
        facets
            .iter()
            .map(|facet| format!(
                "{{\"value\":\"{}\",\"count\":{}}}",
                json_escape(&facet.value),
                facet.count
            ))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn status_counts_json(counts: &BTreeMap<String, usize>) -> String {
    let entries = counts
        .iter()
        .map(|(status, count)| format!("\"{}\":{}", json_escape(status), count))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{entries}}}")
}

fn generated_artifacts_json(case_dir: &Path) -> String {
    let review_path = case_dir.join("review/index.html");
    let viewer_path = case_dir.join("review/evidence-viewer.html");
    let report_path = case_dir.join("reports/case-report.html");
    let package_manifest_path = case_dir.join("packages/package-manifest.json");
    format!(
        "{{\"review_html\":{},\"evidence_viewer_html\":{},\"case_report_html\":{},\
\"package_manifest\":{}}}",
        artifact_path_json(&review_path),
        artifact_path_json(&viewer_path),
        artifact_path_json(&report_path),
        artifact_path_json(&package_manifest_path)
    )
}

fn artifact_path_json(path: &Path) -> String {
    format!(
        "{{\"path\":\"{}\",\"exists\":{}}}",
        json_escape(&path.to_string_lossy()),
        path.is_file()
    )
}

fn extract_json_string(line: &str, key: &str) -> Option<String> {
    let key = format!("\"{}\":", key);
    let start = line.find(&key)? + key.len();
    let value = line[start..].trim_start();
    if value.starts_with("null") {
        return None;
    }
    let value = value.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'b' => out.push('\u{08}'),
                'f' => out.push('\u{0C}'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                other => out.push(other),
            },
            other => out.push(other),
        }
    }
    None
}
