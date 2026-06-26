use super::qa_repro_json::normalize_reproducibility_text;
use crate::qa::QaReport;
use crate::util::{json_escape, read_to_string, write_text};
use std::fs;
use std::path::{Path, PathBuf};

pub fn reproducibility_report(
    left_case_dir: &Path,
    right_case_dir: &Path,
    output_dir: &Path,
) -> Result<QaReport, String> {
    let left = normalized_case_core(left_case_dir)?;
    let right = normalized_case_core(right_case_dir)?;
    let normalized_core_differences = normalized_core_differences(&left, &right);
    let allowed_normalized_core_differences = 0usize;
    let passed = normalized_core_differences <= allowed_normalized_core_differences;
    fs::create_dir_all(output_dir)
        .map_err(|err| format!("failed to create QA output directory: {err}"))?;
    let report_path = output_dir.join("reproducibility-report.json");
    write_text(
        &report_path,
        &format!(
            "{{\n  \"schema_version\": 1,\n  \"qa_type\": \"reproducibility\",\n  \"passed\": {},\n  \"left_case\": \"{}\",\n  \"right_case\": \"{}\",\n  \"normalized_left_bytes\": {},\n  \"normalized_right_bytes\": {},\n  \"allowed_diff_thresholds\": {{\n    \"normalized_core_differences\": {}\n  }},\n  \"diff_metrics\": {{\n    \"normalized_core_differences\": {},\n    \"normalized_left_bytes\": {},\n    \"normalized_right_bytes\": {}\n  }}\n}}\n",
            passed,
            json_escape(&left_case_dir.to_string_lossy()),
            json_escape(&right_case_dir.to_string_lossy()),
            left.len(),
            right.len(),
            allowed_normalized_core_differences,
            normalized_core_differences,
            left.len(),
            right.len()
        ),
    )
    .map_err(|err| format!("failed to write reproducibility report: {err}"))?;
    if passed {
        Ok(QaReport {
            report_path,
            passed,
        })
    } else {
        Err("reproducibility QA failed: normalized core outputs differ".to_string())
    }
}

fn normalized_core_differences(left: &str, right: &str) -> usize {
    if left == right {
        return 0;
    }
    let left_lines = left.lines().collect::<Vec<_>>();
    let right_lines = right.lines().collect::<Vec<_>>();
    let shared = left_lines.len().min(right_lines.len());
    let changed = left_lines
        .iter()
        .zip(right_lines.iter())
        .take(shared)
        .filter(|(left, right)| left != right)
        .count();
    changed + left_lines.len().abs_diff(right_lines.len())
}

fn normalized_case_core(case_dir: &Path) -> Result<String, String> {
    let mut parts = Vec::new();
    for rel in ["db/videos.jsonl", "db/video_paths.tsv"] {
        parts.push(normalized_required_file_part(case_dir, rel)?);
    }
    for rel in [
        "db/carve_results.json",
        "artifacts/carved/carve-log.jsonl",
        "evidence/logs/tsk-audit.jsonl",
        "evidence/logs/validation-log.jsonl",
    ] {
        if let Some(part) = normalized_optional_file_part(case_dir, rel)? {
            parts.push(part);
        }
    }
    parts.extend(normalized_optional_dir_parts(case_dir, "db/filesystem")?);
    parts.extend(normalized_package_manifest_parts(case_dir)?);
    parts.sort();
    Ok(parts.join("\n"))
}

fn normalized_required_file_part(case_dir: &Path, rel: &str) -> Result<String, String> {
    let path = case_dir.join(rel);
    let text = read_to_string(&path).map_err(|err| {
        format!(
            "failed to read reproducibility input {}: {err}",
            path.display()
        )
    })?;
    Ok(normalized_text_part(case_dir, rel, &text))
}

fn normalized_optional_file_part(case_dir: &Path, rel: &str) -> Result<Option<String>, String> {
    let path = case_dir.join(rel);
    if !path.is_file() {
        return Ok(None);
    }
    let text = read_to_string(&path).map_err(|err| {
        format!(
            "failed to read reproducibility input {}: {err}",
            path.display()
        )
    })?;
    Ok(Some(normalized_text_part(case_dir, rel, &text)))
}

fn normalized_optional_dir_parts(case_dir: &Path, rel_dir: &str) -> Result<Vec<String>, String> {
    let dir = case_dir.join(rel_dir);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect_files_recursively(&dir, &mut files)?;
    files.sort();

    let mut normalized_contents = Vec::new();
    for path in files {
        let text = read_to_string(&path).map_err(|err| {
            format!(
                "failed to read reproducibility input {}: {err}",
                path.display()
            )
        })?;
        normalized_contents.push(normalized_text_part(case_dir, rel_dir, &text));
    }
    normalized_contents.sort();
    Ok(normalized_contents
        .into_iter()
        .enumerate()
        .map(|(index, part)| format!("{rel_dir}_artifact_{index}\n{part}"))
        .collect())
}

fn normalized_package_manifest_parts(case_dir: &Path) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    collect_files_recursively(case_dir, &mut files)?;
    files.retain(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "package-manifest.json" || name == "manifest.sha256")
    });

    let mut normalized_contents = Vec::new();
    for path in files {
        let text = read_to_string(&path).map_err(|err| {
            format!(
                "failed to read package reproducibility input {}: {err}",
                path.display()
            )
        })?;
        let label = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("package-manifest");
        normalized_contents.push(normalized_text_part(case_dir, label, &text));
    }
    normalized_contents.sort();
    Ok(normalized_contents
        .into_iter()
        .enumerate()
        .map(|(index, part)| format!("package_artifact_{index}\n{part}"))
        .collect())
}

fn collect_files_recursively(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir)
        .map_err(|err| format!("failed to read directory {}: {err}", dir.display()))?
    {
        let entry = entry.map_err(|err| format!("failed to read directory entry: {err}"))?;
        let file_type = entry
            .file_type()
            .map_err(|err| format!("failed to read file type for {:?}: {err}", entry.path()))?;
        if file_type.is_dir() {
            collect_files_recursively(&entry.path(), out)?;
        } else if file_type.is_file() {
            out.push(entry.path());
        }
    }
    Ok(())
}

fn normalized_text_part(case_dir: &Path, label: &str, text: &str) -> String {
    let normalized = normalize_reproducibility_text(case_dir, text);
    let mut lines = normalized
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    lines.sort();
    format!("{label}\n{}\n", lines.join("\n"))
}
