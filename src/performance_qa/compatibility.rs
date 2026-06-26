use super::rows_per_minute;
use crate::case_db::{self, ExportManifestRequest};
use crate::util::json_escape;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

#[derive(Debug)]
pub(super) struct CompatibilityExportEvidence {
    pub(super) jsonl_path: String,
    pub(super) jsonl_rows: usize,
    pub(super) jsonl_elapsed_ms: u128,
    pub(super) jsonl_rows_per_minute: u128,
    pub(super) tsv_path: String,
    pub(super) tsv_rows: usize,
    pub(super) tsv_elapsed_ms: u128,
    pub(super) tsv_rows_per_minute: u128,
    pub(super) manifest_path: String,
    pub(super) manifest_selected_count: usize,
    pub(super) manifest_elapsed_ms: u128,
}

struct CompatibilityRow {
    file_id: String,
    full_path: String,
    relative_path: String,
    extension: String,
    size_bytes: u64,
    sha256: Option<String>,
    hash_state: String,
    parser_lane: String,
}

pub(super) fn compatibility_export_evidence(
    case_dir: &Path,
    rows: usize,
) -> Result<CompatibilityExportEvidence, String> {
    let jsonl_path = case_dir.join("db/videos.jsonl");
    let tsv_path = case_dir.join("db/video_paths.tsv");
    let jsonl_started = Instant::now();
    let jsonl_rows = write_compatibility_jsonl(case_dir, &jsonl_path)?;
    let jsonl_elapsed_ms = jsonl_started.elapsed().as_millis();
    let tsv_started = Instant::now();
    let tsv_rows = write_compatibility_tsv(case_dir, &tsv_path)?;
    let tsv_elapsed_ms = tsv_started.elapsed().as_millis();
    let manifest_started = Instant::now();
    let manifest = case_db::export_manifest(
        case_dir,
        &ExportManifestRequest {
            file_ids: selected_export_ids(rows),
            operator: "qa-performance".to_string(),
            filters_json: Some("{\"source\":\"performance-profile\"}".to_string()),
            output_path: None,
        },
    )?;
    Ok(CompatibilityExportEvidence {
        jsonl_path: jsonl_path.to_string_lossy().to_string(),
        jsonl_rows,
        jsonl_elapsed_ms,
        jsonl_rows_per_minute: rows_per_minute(jsonl_rows, jsonl_elapsed_ms),
        tsv_path: tsv_path.to_string_lossy().to_string(),
        tsv_rows,
        tsv_elapsed_ms,
        tsv_rows_per_minute: rows_per_minute(tsv_rows, tsv_elapsed_ms),
        manifest_path: manifest.output_path.to_string_lossy().to_string(),
        manifest_selected_count: manifest.selected_count,
        manifest_elapsed_ms: manifest_started.elapsed().as_millis(),
    })
}

fn write_compatibility_jsonl(case_dir: &Path, path: &Path) -> Result<usize, String> {
    let file = File::create(path).map_err(|err| {
        format!(
            "failed to create compatibility JSONL {}: {err}",
            path.display()
        )
    })?;
    let mut writer = BufWriter::new(file);
    let mut written = 0usize;
    for_each_compatibility_row(case_dir, |row| {
        writer
            .write_all(compatibility_row_json(&row).as_bytes())
            .and_then(|_| writer.write_all(b"\n"))
            .map_err(|err| format!("failed to write compatibility JSONL: {err}"))?;
        written += 1;
        Ok(())
    })?;
    writer
        .flush()
        .map_err(|err| format!("failed to flush compatibility JSONL: {err}"))?;
    Ok(written)
}

fn write_compatibility_tsv(case_dir: &Path, path: &Path) -> Result<usize, String> {
    let file = File::create(path).map_err(|err| {
        format!(
            "failed to create compatibility TSV {}: {err}",
            path.display()
        )
    })?;
    let mut writer = BufWriter::new(file);
    writer
        .write_all(b"id\tsource_path\trelative_path\textension\tsize_bytes\tsha256\tparser\n")
        .map_err(|err| format!("failed to write compatibility TSV header: {err}"))?;
    let mut written = 0usize;
    for_each_compatibility_row(case_dir, |row| {
        writer
            .write_all(compatibility_tsv_row(&row).as_bytes())
            .map_err(|err| format!("failed to write compatibility TSV: {err}"))?;
        written += 1;
        Ok(())
    })?;
    writer
        .flush()
        .map_err(|err| format!("failed to flush compatibility TSV: {err}"))?;
    Ok(written)
}

fn for_each_compatibility_row<F>(case_dir: &Path, mut visit: F) -> Result<(), String>
where
    F: FnMut(CompatibilityRow) -> Result<(), String>,
{
    let conn = rusqlite::Connection::open(case_db::case_db_path(case_dir))
        .map_err(|err| format!("failed to open compatibility export db: {err}"))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, source_path, relative_path, extension, size_bytes, sha256, hash_status, \
             source_profile_json FROM videos ORDER BY relative_path ASC, id ASC",
        )
        .map_err(|err| format!("failed to prepare compatibility export query: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            let source_profile_json: String = row.get(7)?;
            Ok(CompatibilityRow {
                file_id: row.get(0)?,
                full_path: row.get(1)?,
                relative_path: row.get(2)?,
                extension: row.get(3)?,
                size_bytes: row.get::<_, i64>(4)?.max(0) as u64,
                sha256: row.get(5)?,
                hash_state: row.get(6)?,
                parser_lane: if source_profile_json.contains("\"parser\":\"benchmark\"") {
                    "benchmark".to_string()
                } else {
                    "video-index".to_string()
                },
            })
        })
        .map_err(|err| format!("failed to query compatibility export rows: {err}"))?;
    for row in rows {
        visit(row.map_err(|err| format!("failed to read compatibility export row: {err}"))?)?;
    }
    Ok(())
}

fn compatibility_row_json(row: &CompatibilityRow) -> String {
    format!(
        "{{\"id\":\"{}\",\"source_path\":\"{}\",\"relative_path\":\"{}\",\
\"extension\":\"{}\",\"size_bytes\":{},\"sha256\":{},\"hash_status\":\"{}\",\
\"source_profile\":{{\"parser\":\"{}\"}}}}",
        json_escape(&row.file_id),
        json_escape(&row.full_path),
        json_escape(&row.relative_path),
        json_escape(&row.extension),
        row.size_bytes,
        optional_json_string(row.sha256.as_deref()),
        json_escape(&row.hash_state),
        json_escape(&row.parser_lane)
    )
}

fn compatibility_tsv_row(row: &CompatibilityRow) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
        tsv_escape(&row.file_id),
        tsv_escape(&row.full_path),
        tsv_escape(&row.relative_path),
        tsv_escape(&row.extension),
        row.size_bytes,
        tsv_escape(row.sha256.as_deref().unwrap_or("")),
        tsv_escape(&row.parser_lane)
    )
}

fn selected_export_ids(rows: usize) -> Vec<String> {
    let last = rows.saturating_sub(1);
    [0usize, rows / 2, last]
        .into_iter()
        .filter(|index| *index < rows)
        .map(|index| format!("bench_{index:08}"))
        .collect()
}

fn optional_json_string(value: Option<&str>) -> String {
    value
        .map(|inner| format!("\"{}\"", json_escape(inner)))
        .unwrap_or_else(|| "null".to_string())
}

fn tsv_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\t', "\\t")
}
