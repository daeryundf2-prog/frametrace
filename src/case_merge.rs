//! Merge other cases' video indexes into this case's index.
//!
//! Each source case contributes its `db/videos.jsonl` rows as new records:
//! record ids are renumbered onto this case's `vid_NNNNNN` sequence (the
//! original id is kept as `merged_from_id`), and every merged record carries
//! `merged_from` with the source case path for provenance. Records whose
//! recorded sha256 already exists in the target index are still merged —
//! both records stay — but the merged copy is marked `duplicate_of` the
//! earlier record's id. Merged rows are claims copied from another case's
//! index, not re-verified evidence, hence the `candidate-merge` label.
//!
//! The merge rewrites `db/videos.jsonl`, `db/video_index.json`, and
//! `db/video_paths.tsv`, upserts the new rows into `db/case.db`, and appends
//! one chained audit entry per source case to
//! `evidence/logs/case-merge-log.jsonl`.

use crate::audit;
use crate::case_db::{IndexedVideoRow, upsert_indexed_rows};
use crate::scan::{json_record_lines, set_json_field, tsv_escape};
use crate::util::{json_escape, now_unix, read_to_string, write_text_atomic};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Candidate-grade label: merged rows are recorded claims from another
/// case's index, not re-verified content.
pub const LABEL: &str = "candidate-merge";

#[derive(Debug, Clone)]
pub struct MergeSourceSummary {
    pub case_dir: PathBuf,
    pub merged_records: usize,
    pub duplicate_records: usize,
}

#[derive(Debug, Clone)]
pub struct MergeResult {
    pub generated_unix: u64,
    pub total_records: usize,
    pub merged_records: usize,
    pub duplicate_records: usize,
    pub sources: Vec<MergeSourceSummary>,
}

/// Merge every `source_case_dirs` index into `case_dir`'s index. Sources are
/// processed in argument order; the operation fails loudly when a source has
/// no index (a likely operator error) but tolerates unparseable rows by
/// skipping them, matching the read policy of the other index consumers.
pub fn merge_cases(case_dir: &Path, source_case_dirs: &[PathBuf]) -> Result<MergeResult, String> {
    if source_case_dirs.is_empty() {
        return Err("merge-cases requires at least one source case directory".to_string());
    }
    let case_root = case_dir
        .canonicalize()
        .map_err(|err| format!("failed to canonicalize case directory: {err}"))?;
    let mut existing_lines = read_index_lines(case_dir, false)?;

    // sha256 -> first record id, for duplicate marking. Unhashed records
    // never deduplicate (nothing trustworthy to compare on).
    let mut id_by_sha256: HashMap<String, String> = HashMap::new();
    let mut next_number = 1usize;
    for line in &existing_lines {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let id = string_field(&value, "id");
        if let Some(number) = id
            .strip_prefix("vid_")
            .and_then(|rest| rest.parse::<usize>().ok())
        {
            next_number = next_number.max(number + 1);
        }
        if let Some(sha) = value.get("sha256").and_then(serde_json::Value::as_str) {
            id_by_sha256
                .entry(sha.to_ascii_lowercase())
                .or_insert_with(|| id.clone());
        }
    }

    // Validate every source before mutating anything: a failure here must
    // leave the target index and its audit log untouched.
    let mut sources: Vec<(PathBuf, Vec<String>)> = Vec::new();
    for source_dir in source_case_dirs {
        let source_root = source_dir.canonicalize().map_err(|err| {
            format!(
                "failed to canonicalize source case {}: {err}",
                source_dir.display()
            )
        })?;
        if source_root == case_root {
            return Err(format!(
                "cannot merge a case into itself: {}",
                source_dir.display()
            ));
        }
        let source_lines = read_index_lines(&source_root, true)?;
        sources.push((source_root, source_lines));
    }

    let mut new_lines: Vec<String> = Vec::new();
    let mut new_rows: Vec<IndexedVideoRow> = Vec::new();
    let mut summaries = Vec::new();
    let generated_unix = now_unix()?;

    for (source_root, source_lines) in &sources {
        let mut merged = 0usize;
        let mut duplicates = 0usize;
        for line in source_lines {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            let original_id = string_field(&value, "id");
            let new_id = format!("vid_{next_number:06}");
            next_number += 1;
            let duplicate_of = value
                .get("sha256")
                .and_then(serde_json::Value::as_str)
                .and_then(|sha| id_by_sha256.get(&sha.to_ascii_lowercase()).cloned());
            if duplicate_of.is_some() {
                duplicates += 1;
            }
            if let Some(sha) = value.get("sha256").and_then(serde_json::Value::as_str) {
                id_by_sha256
                    .entry(sha.to_ascii_lowercase())
                    .or_insert_with(|| new_id.clone());
            }

            // Textual field edits keep the copied record's published field
            // order/spelling; only provenance is added or replaced.
            let mut merged_line =
                set_json_field(line, "id", &format!("\"{}\"", json_escape(&new_id)));
            merged_line = set_json_field(
                &merged_line,
                "merged_from",
                &format!("\"{}\"", json_escape(&source_root.to_string_lossy())),
            );
            merged_line = set_json_field(
                &merged_line,
                "merged_from_id",
                &format!("\"{}\"", json_escape(&original_id)),
            );
            if let Some(duplicate_of) = &duplicate_of {
                merged_line = set_json_field(
                    &merged_line,
                    "duplicate_of",
                    &format!("\"{}\"", json_escape(duplicate_of)),
                );
            }
            let parsed: serde_json::Value = serde_json::from_str(&merged_line)
                .map_err(|err| format!("merged record failed to reparse: {err}"))?;
            new_rows.push(row_from_value(&parsed, &merged_line));
            new_lines.push(merged_line);
            merged += 1;
        }
        summaries.push(MergeSourceSummary {
            case_dir: source_root.clone(),
            merged_records: merged,
            duplicate_records: duplicates,
        });
        let line = format!(
            "{{\"schema_version\":1,\"event\":\"merge-cases\",\"generated_unix\":{},\"label\":\"{}\",\"source_case\":\"{}\",\"merged_records\":{},\"duplicate_records\":{}}}",
            generated_unix,
            LABEL,
            json_escape(&source_root.to_string_lossy()),
            merged,
            duplicates,
        );
        audit::append_chained_jsonl(&case_dir.join("evidence/logs/case-merge-log.jsonl"), &line)?;
    }

    // Deterministic ordering: existing records keep their (lower) id order
    // and merged records follow in argument/line order under fresh ids.
    existing_lines.extend(new_lines.iter().cloned());
    let mut all = existing_lines;
    all.sort_by_key(|line| line_id(line));

    let mut jsonl = String::new();
    let mut tsv = String::from(
        "id\tsource_path\trelative_path\textension\tsize_bytes\tsha256\tvendor\tparser\tparser_confidence\n",
    );
    for line in &all {
        jsonl.push_str(line);
        jsonl.push('\n');
        tsv.push_str(&tsv_row_for_line(line));
    }
    write_text_atomic(&case_dir.join("db/videos.jsonl"), &jsonl)
        .map_err(|err| format!("failed to write merged video jsonl: {err}"))?;
    write_text_atomic(&case_dir.join("db/video_paths.tsv"), &tsv)
        .map_err(|err| format!("failed to write merged video path index: {err}"))?;
    write_merged_index_json(case_dir, &all, generated_unix)?;
    upsert_indexed_rows(case_dir, &new_rows, generated_unix)?;

    let duplicate_total = summaries.iter().map(|s| s.duplicate_records).sum();
    let merged_total = summaries.iter().map(|s| s.merged_records).sum();
    Ok(MergeResult {
        generated_unix,
        total_records: all.len(),
        merged_records: merged_total,
        duplicate_records: duplicate_total,
        sources: summaries,
    })
}

/// Lines of a case's `db/videos.jsonl`. `required` distinguishes the target
/// (absent index = empty index) from a source (absent index = operator
/// error, fail loudly rather than merge nothing).
fn read_index_lines(case_dir: &Path, required: bool) -> Result<Vec<String>, String> {
    let path = case_dir.join("db/videos.jsonl");
    let text = match read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound && !required => {
            return Ok(Vec::new());
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!(
                "source case has no video index at {} — run scan-folder there first",
                path.display()
            ));
        }
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    Ok(json_record_lines(&text))
}

fn line_id(line: &str) -> String {
    serde_json::from_str::<serde_json::Value>(line)
        .ok()
        .map(|value| string_field(&value, "id"))
        .unwrap_or_default()
}

fn string_field(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn opt_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

/// One `video_paths.tsv` row from a serialized index record.
fn tsv_row_for_line(line: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
        return String::new();
    };
    let profile = value.get("source_profile").cloned().unwrap_or_default();
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
        tsv_escape(&string_field(&value, "id")),
        tsv_escape(&string_field(&value, "source_path")),
        tsv_escape(&string_field(&value, "relative_path")),
        tsv_escape(&string_field(&value, "extension")),
        value
            .get("size_bytes")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0),
        tsv_escape(&opt_string(&value, "sha256").unwrap_or_default()),
        tsv_escape(&string_field(&profile, "vendor")),
        tsv_escape(&string_field(&profile, "parser")),
        tsv_escape(&string_field(&profile, "confidence")),
    )
}

/// SQLite row for a merged record. `record_json` is the verbatim merged line
/// so `record_json` and the JSONL stay byte-identical.
fn row_from_value(value: &serde_json::Value, record_json: &str) -> IndexedVideoRow {
    let ffprobe_json = match value.get("ffprobe") {
        Some(serde_json::Value::String(text)) => Some(text.clone()),
        Some(other @ (serde_json::Value::Object(_) | serde_json::Value::Array(_))) => {
            Some(other.to_string())
        }
        _ => None,
    };
    IndexedVideoRow {
        id: string_field(value, "id"),
        source_path: string_field(value, "source_path"),
        file_url: string_field(value, "file_url"),
        relative_path: string_field(value, "relative_path"),
        extension: string_field(value, "extension"),
        size_bytes: value
            .get("size_bytes")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0),
        modified_unix: value
            .get("modified_unix")
            .and_then(serde_json::Value::as_u64),
        sha256: opt_string(value, "sha256"),
        hash_status: {
            let status = string_field(value, "hash_status");
            if status.is_empty() {
                "unknown".to_string()
            } else {
                status
            }
        },
        confidence: {
            let confidence = string_field(value, "confidence");
            if confidence.is_empty() {
                "unknown".to_string()
            } else {
                confidence
            }
        },
        source_profile_json: value
            .get("source_profile")
            .map(serde_json::Value::to_string)
            .unwrap_or_else(|| "{}".to_string()),
        duration_seconds: value
            .get("duration_seconds")
            .and_then(serde_json::Value::as_f64),
        format_name: opt_string(value, "format_name"),
        video_codec: opt_string(value, "video_codec"),
        audio_codec: opt_string(value, "audio_codec"),
        width: value.get("width").and_then(serde_json::Value::as_u64),
        height: value.get("height").and_then(serde_json::Value::as_u64),
        ffprobe_ok: value
            .get("ffprobe_ok")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        ffprobe_error: opt_string(value, "ffprobe_error"),
        ffprobe_json,
        record_json: record_json.to_string(),
    }
}

/// Rewrite `db/video_index.json` with the merged record set. Existing
/// top-level fields (source_path, scan options, warnings) are preserved;
/// only `videos`, `video_count`, `total_bytes`, and a `merged_unix` stamp
/// change. When no index exists yet a minimal document is created.
fn write_merged_index_json(
    case_dir: &Path,
    lines: &[String],
    merged_unix: u64,
) -> Result<(), String> {
    let path = case_dir.join("db/video_index.json");
    let mut index = match read_to_string(&path) {
        Ok(text) => serde_json::from_str::<serde_json::Value>(&text)
            .unwrap_or_else(|_| serde_json::json!({})),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => serde_json::json!({}),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    if !index.is_object() {
        index = serde_json::json!({});
    }
    let videos: Vec<serde_json::Value> = lines
        .iter()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .collect();
    let total_bytes: u64 = videos
        .iter()
        .filter_map(|video| video.get("size_bytes").and_then(serde_json::Value::as_u64))
        .sum();
    let object = index.as_object_mut().expect("object ensured above");
    object.insert("schema_version".to_string(), serde_json::json!(1));
    object
        .entry("source_path".to_string())
        .or_insert(serde_json::Value::Null);
    object.insert("videos".to_string(), serde_json::Value::Array(videos));
    object.insert("video_count".to_string(), serde_json::json!(lines.len()));
    object.insert("total_bytes".to_string(), serde_json::json!(total_bytes));
    object.insert("merged_unix".to_string(), serde_json::json!(merged_unix));
    let text = serde_json::to_string_pretty(&index)
        .map_err(|err| format!("failed to serialize merged index: {err}"))?;
    write_text_atomic(&path, &format!("{text}\n"))
        .map_err(|err| format!("failed to write merged video index: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_case(name: &str, videos_jsonl: Option<&str>) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "frametrace-merge-cases-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("db")).unwrap();
        fs::write(dir.join("case.json"), "{}").unwrap();
        if let Some(text) = videos_jsonl {
            fs::write(dir.join("db/videos.jsonl"), text).unwrap();
        }
        dir
    }

    fn row(id: &str, path: &str, size: u64, sha: Option<&str>) -> String {
        format!(
            "{{\"id\":\"{id}\",\"source_path\":\"{path}\",\"file_url\":\"file://{path}\",\"relative_path\":\"{id}.mp4\",\"extension\":\"mp4\",\"size_bytes\":{size},\"modified_unix\":100,\"sha256\":{},\"hash_status\":\"complete\",\"confidence\":\"ffprobe-confirmed\",\"source_profile\":{{\"lane\":\"generic-video\",\"vendor\":\"Generic media\",\"parser\":\"generic_media\",\"confidence\":\"medium\",\"recommended_action\":\"x\",\"evidence\":[\"extension\"]}},\"ffprobe_ok\":true,\"ffprobe\":null}}\n",
            sha.map(|s| format!("\"{s}\""))
                .unwrap_or_else(|| "null".into())
        )
    }

    fn jsonl_ids(case_dir: &Path) -> Vec<serde_json::Value> {
        read_to_string(&case_dir.join("db/videos.jsonl"))
            .unwrap()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    #[test]
    fn merges_two_cases_with_renumbered_ids_and_provenance() {
        let target = temp_case(
            "renum-t",
            Some(&row("vid_000001", "/ev/a.mp4", 10, Some("aa"))),
        );
        let source = temp_case(
            "renum-s",
            Some(&format!(
                "{}{}",
                row("vid_000001", "/ev/b.mp4", 20, Some("bb")),
                row("vid_000002", "/ev/c.mp4", 30, Some("cc"))
            )),
        );
        let result = merge_cases(&target, std::slice::from_ref(&source)).unwrap();
        assert_eq!(result.merged_records, 2);
        assert_eq!(result.duplicate_records, 0);
        assert_eq!(result.total_records, 3);

        let rows = jsonl_ids(&target);
        assert_eq!(rows.len(), 3);
        // Renumbered onto the target's id sequence; original ids preserved.
        let merged: Vec<_> = rows
            .iter()
            .filter(|row| row.get("merged_from").is_some())
            .collect();
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0]["id"].as_str(), Some("vid_000002"));
        assert_eq!(merged[0]["merged_from_id"].as_str(), Some("vid_000001"));
        assert_eq!(
            merged[0]["merged_from"].as_str(),
            Some(source.canonicalize().unwrap().to_string_lossy().as_ref())
        );

        // Index + SQLite were updated too. SQLite receives only the merged
        // rows — the target's own records land there when they are scanned;
        // this fixture wrote videos.jsonl directly, so only the two merged
        // rows are present.
        let index = read_to_string(&target.join("db/video_index.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&index).unwrap();
        assert_eq!(parsed["video_count"].as_u64(), Some(3));
        assert_eq!(parsed["videos"].as_array().unwrap().len(), 3);
        let ids = crate::case_db::load_video_ids(&target).unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.iter().all(|row| row.id != "vid_000001"));

        // One audit entry per source, chain intact.
        let verification =
            audit::verify_chained_jsonl(&target.join("evidence/logs/case-merge-log.jsonl"))
                .unwrap();
        assert_eq!(verification.entries, 1);

        let _ = fs::remove_dir_all(&target);
        let _ = fs::remove_dir_all(&source);
    }

    #[test]
    fn shared_sha256_marks_the_merged_copy_as_duplicate() {
        let target = temp_case(
            "dup-t",
            Some(&row("vid_000001", "/ev/a.mp4", 10, Some("aa"))),
        );
        let source = temp_case(
            "dup-s",
            Some(&format!(
                "{}{}",
                row("vid_000009", "/other/a-copy.mp4", 10, Some("aa")),
                row("vid_000010", "/other/b.mp4", 20, Some("bb"))
            )),
        );
        let result = merge_cases(&target, std::slice::from_ref(&source)).unwrap();
        assert_eq!(result.duplicate_records, 1);

        let rows = jsonl_ids(&target);
        assert_eq!(rows.len(), 3, "both records are kept");
        let dup: Vec<_> = rows
            .iter()
            .filter(|row| row.get("duplicate_of").is_some())
            .collect();
        assert_eq!(dup.len(), 1);
        assert_eq!(dup[0]["duplicate_of"].as_str(), Some("vid_000001"));
        assert_eq!(dup[0]["id"].as_str(), Some("vid_000002"));

        let _ = fs::remove_dir_all(&target);
        let _ = fs::remove_dir_all(&source);
    }

    #[test]
    fn merge_into_an_empty_index_creates_one() {
        let target = temp_case("empty-t", None);
        let source = temp_case(
            "empty-s",
            Some(&row("vid_000004", "/ev/x.mp4", 5, Some("xx"))),
        );
        let result = merge_cases(&target, std::slice::from_ref(&source)).unwrap();
        assert_eq!(result.total_records, 1);
        let rows = jsonl_ids(&target);
        assert_eq!(rows[0]["id"].as_str(), Some("vid_000001"));
        assert_eq!(rows[0]["merged_from_id"].as_str(), Some("vid_000004"));
        let _ = fs::remove_dir_all(&target);
        let _ = fs::remove_dir_all(&source);
    }

    #[test]
    fn rejects_merging_a_case_into_itself_and_indexless_sources() {
        let target = temp_case(
            "rej-t",
            Some(&row("vid_000001", "/ev/a.mp4", 10, Some("aa"))),
        );
        let err = merge_cases(&target, std::slice::from_ref(&target)).unwrap_err();
        assert!(err.contains("into itself"), "{err}");

        let no_index = temp_case("rej-empty", None);
        let err = merge_cases(&target, std::slice::from_ref(&no_index)).unwrap_err();
        assert!(err.contains("no video index"), "{err}");

        let _ = fs::remove_dir_all(&target);
        let _ = fs::remove_dir_all(&no_index);
    }
}
