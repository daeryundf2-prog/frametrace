//! Known-hash filtering (NSRL-style noise reduction) for the case index.
//!
//! Splits every indexed file into `known` (its recorded sha256 appears in a
//! user-supplied hash list) and `unknown`; records that carry no hash are
//! reported separately as `unhashed` rather than silently counted as unknown.
//! The hash list is an examiner-supplied text file — one sha256 per line, or
//! a minimal NSRL-ish `sha256,<anything>` CSV row; `#`/`;` comments and blank
//! lines are skipped and malformed rows are counted, not fatal.
//!
//! The report is a JSON document inside the case (default
//! `reports/known-hash-filter.json`) plus a chained entry in
//! `evidence/logs/known-hash-log.jsonl`. Findings are candidate-grade: a hit
//! means the *recorded* hash matched the list, not that the bytes on disk
//! were re-verified.

use crate::anomaly::{IndexedRow, read_indexed_rows};
use crate::audit;
use crate::util::{json_escape, now_unix, read_to_string, write_text};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Candidate-grade label: matches reflect recorded index hashes only.
pub const LABEL: &str = "candidate-hash-match";

#[derive(Debug, Clone)]
pub struct KnownHashResult {
    pub report_path: PathBuf,
    pub generated_unix: u64,
    pub list_size: usize,
    pub ignored_list_lines: usize,
    pub known_count: usize,
    pub unknown_count: usize,
    pub unhashed_count: usize,
}

/// Classify the case index against `hash_list_path`, write the JSON report
/// inside the case, and append a chained audit entry. `output_path` must
/// resolve inside `case_dir` (`require_case_output_path`); the hash list
/// itself is an input and may live anywhere.
pub fn filter_known_hashes(
    case_dir: &Path,
    hash_list_path: &Path,
    output_path: &Path,
) -> Result<KnownHashResult, String> {
    crate::tool_policy::require_case_output_path(case_dir, output_path, "known-hash filter")?;
    let (known_hashes, ignored_list_lines) = read_hash_list(hash_list_path)?;
    let rows = read_indexed_rows(case_dir)?;

    let mut known = Vec::new();
    let mut unknown = Vec::new();
    let mut unhashed = Vec::new();
    for row in &rows {
        match row.sha256.as_deref() {
            Some(sha256) if known_hashes.contains(&sha256.to_ascii_lowercase()) => known.push(row),
            Some(_) => unknown.push(row),
            None => unhashed.push(row),
        }
    }

    let generated_unix = now_unix()?;
    let report = report_json(
        case_dir,
        hash_list_path,
        generated_unix,
        known_hashes.len(),
        ignored_list_lines,
        &known,
        &unknown,
        &unhashed,
    );
    write_text(output_path, &report).map_err(|err| {
        format!(
            "failed to write known-hash report {}: {err}",
            output_path.display()
        )
    })?;
    let line = format!(
        "{{\"schema_version\":1,\"event\":\"known-hash-filter\",\"generated_unix\":{},\"label\":\"{}\",\"hash_list\":\"{}\",\"report_path\":\"{}\",\"list_size\":{},\"ignored_list_lines\":{},\"known\":{},\"unknown\":{},\"unhashed\":{}}}",
        generated_unix,
        LABEL,
        json_escape(&hash_list_path.to_string_lossy()),
        json_escape(&output_path.to_string_lossy()),
        known_hashes.len(),
        ignored_list_lines,
        known.len(),
        unknown.len(),
        unhashed.len(),
    );
    audit::append_chained_jsonl(&case_dir.join("evidence/logs/known-hash-log.jsonl"), &line)?;
    Ok(KnownHashResult {
        report_path: output_path.to_path_buf(),
        generated_unix,
        list_size: known_hashes.len(),
        ignored_list_lines,
        known_count: known.len(),
        unknown_count: unknown.len(),
        unhashed_count: unhashed.len(),
    })
}

/// One sha256 per line, or `sha256,<rest>` NSRL-ish CSV rows. `#`/`;`
/// comments and blanks are skipped; anything else malformed is counted as
/// ignored rather than failing the run.
fn read_hash_list(path: &Path) -> Result<(HashSet<String>, usize), String> {
    let text = read_to_string(path)
        .map_err(|err| format!("failed to read hash list {}: {err}", path.display()))?;
    let mut hashes = HashSet::new();
    let mut ignored = 0usize;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        // NSRL-ish rows put the digest first ("SHA-1","name",... style
        // lists quote fields — strip a leading/trailing quote pair).
        let token = line
            .split([',', '\t', ' '])
            .next()
            .unwrap_or("")
            .trim_matches('"')
            .to_ascii_lowercase();
        if token.len() == 64 && token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            hashes.insert(token);
        } else {
            ignored += 1;
        }
    }
    Ok((hashes, ignored))
}

fn rows_json(rows: &[&IndexedRow]) -> String {
    rows.iter()
        .map(|row| {
            format!(
                "{{\"id\":\"{}\",\"path\":\"{}\",\"sha256\":{}}}",
                json_escape(&row.id),
                json_escape(&row.source_path),
                row.sha256
                    .as_deref()
                    .map(|sha| format!("\"{}\"", json_escape(sha)))
                    .unwrap_or_else(|| "null".to_string()),
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[allow(clippy::too_many_arguments)]
fn report_json(
    case_dir: &Path,
    hash_list_path: &Path,
    generated_unix: u64,
    list_size: usize,
    ignored_list_lines: usize,
    known: &[&IndexedRow],
    unknown: &[&IndexedRow],
    unhashed: &[&IndexedRow],
) -> String {
    format!(
        "{{\n  \"schema_version\": 1,\n  \"label\": \"{}\",\n  \"generated_unix\": {},\n  \"case_dir\": \"{}\",\n  \"hash_list\": \"{}\",\n  \"summary\": {{\"list_size\": {}, \"ignored_list_lines\": {}, \"known\": {}, \"unknown\": {}, \"unhashed\": {}}},\n  \"known\": [{}],\n  \"unknown\": [{}],\n  \"unhashed\": [{}]\n}}\n",
        LABEL,
        generated_unix,
        json_escape(&case_dir.to_string_lossy()),
        json_escape(&hash_list_path.to_string_lossy()),
        list_size,
        ignored_list_lines,
        known.len(),
        unknown.len(),
        unhashed.len(),
        rows_json(known),
        rows_json(unknown),
        rows_json(unhashed),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_case(name: &str, videos_jsonl: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "frametrace-knownhash-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("db")).unwrap();
        fs::write(dir.join("case.json"), "{}").unwrap();
        fs::write(dir.join("db/videos.jsonl"), videos_jsonl).unwrap();
        dir
    }

    fn row(id: &str, path: &str, sha: Option<&str>) -> String {
        format!(
            "{{\"id\":\"{id}\",\"source_path\":\"{path}\",\"size_bytes\":1,\"sha256\":{}}}\n",
            sha.map(|s| format!("\"{s}\""))
                .unwrap_or_else(|| "null".into())
        )
    }

    const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const SHA_C: &str = "CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC";

    fn write_list(case_dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = case_dir.join(name);
        fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn splits_index_into_known_unknown_and_unhashed() {
        let case_dir = temp_case(
            "split",
            &format!(
                "{}{}{}",
                row("vid_1", "/ev/known.mp4", Some(SHA_A)),
                row("vid_2", "/ev/unknown.mp4", Some(SHA_B)),
                row("vid_3", "/ev/unhashed.mp4", None),
            ),
        );
        // Mixed formats: plain digest, NSRL-ish CSV row, comment, uppercase.
        let list = write_list(
            &case_dir,
            "known.txt",
            &format!("# comment\n{SHA_A}\n{SHA_C},\"file name\",0\nnot-a-hash\n"),
        );
        let output = case_dir.join("reports/known-hash-filter.json");
        let result = filter_known_hashes(&case_dir, &list, &output).unwrap();
        assert_eq!(result.list_size, 2);
        assert_eq!(result.ignored_list_lines, 1);
        assert_eq!(result.known_count, 1);
        assert_eq!(result.unknown_count, 1);
        assert_eq!(result.unhashed_count, 1);

        let report = read_to_string(&output).unwrap();
        assert!(report.contains("\"/ev/known.mp4\""));
        assert!(report.contains(LABEL));
        let parsed: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert_eq!(parsed["summary"]["known"].as_u64(), Some(1));

        let verification =
            audit::verify_chained_jsonl(&case_dir.join("evidence/logs/known-hash-log.jsonl"))
                .unwrap();
        assert_eq!(verification.entries, 1);
        let _ = fs::remove_dir_all(&case_dir);
    }

    #[test]
    fn matching_is_case_insensitive_on_both_sides() {
        // Index stores SHA_C uppercase; the list is lowercase — still a hit.
        let case_dir = temp_case("casefold", &row("vid_1", "/ev/a.mp4", Some(SHA_C)));
        let list = write_list(
            &case_dir,
            "known.txt",
            &format!("{}\n", SHA_C.to_lowercase()),
        );
        let output = case_dir.join("reports/known-hash-filter.json");
        let result = filter_known_hashes(&case_dir, &list, &output).unwrap();
        assert_eq!(result.known_count, 1);
        assert_eq!(result.unknown_count, 0);
        let _ = fs::remove_dir_all(&case_dir);
    }

    #[test]
    fn empty_list_marks_everything_unknown_or_unhashed() {
        let case_dir = temp_case(
            "emptylist",
            &format!(
                "{}{}",
                row("vid_1", "/ev/a.mp4", Some(SHA_A)),
                row("vid_2", "/ev/b.mp4", None),
            ),
        );
        let list = write_list(&case_dir, "known.txt", "# nothing here\n\n");
        let output = case_dir.join("reports/known-hash-filter.json");
        let result = filter_known_hashes(&case_dir, &list, &output).unwrap();
        assert_eq!(result.list_size, 0);
        assert_eq!(result.known_count, 0);
        assert_eq!(result.unknown_count, 1);
        assert_eq!(result.unhashed_count, 1);
        let _ = fs::remove_dir_all(&case_dir);
    }

    #[test]
    fn empty_index_yields_zero_counts() {
        let case_dir = temp_case("emptyindex", "");
        let list = write_list(&case_dir, "known.txt", &format!("{SHA_A}\n"));
        let output = case_dir.join("reports/known-hash-filter.json");
        let result = filter_known_hashes(&case_dir, &list, &output).unwrap();
        assert_eq!(
            result.known_count + result.unknown_count + result.unhashed_count,
            0
        );
        let _ = fs::remove_dir_all(&case_dir);
    }

    #[test]
    fn report_must_stay_inside_the_case() {
        let case_dir = temp_case("confined", &row("vid_1", "/ev/a.mp4", Some(SHA_A)));
        let list = write_list(&case_dir, "known.txt", &format!("{SHA_A}\n"));
        let outside = case_dir.parent().unwrap().join("knownhash-outside.json");
        let err = filter_known_hashes(&case_dir, &list, &outside).unwrap_err();
        assert!(err.contains("inside the case directory"), "{err}");
        assert!(!outside.exists());
        let _ = fs::remove_dir_all(&case_dir);
    }
}
