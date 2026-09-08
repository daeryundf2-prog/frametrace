use crate::case_db::{self, IndexedVideoRow};
use crate::detector;
use crate::ffprobe;
use crate::model::{ProbeSummary, ScanOptions, ScanResult, VideoRecord};
use crate::sha256;
use crate::util::{
    canonicalize_display, json_escape, now_unix, path_to_file_url, read_to_string, unique_path,
    write_text_atomic,
};
use std::collections::{HashMap, VecDeque};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "m4v", "avi", "mkv", "wmv", "asf", "mpg", "mpeg", "mts", "m2ts", "ts", "3gp",
    "webm", "flv", "dav", "dav_", "nov", "ave", "g64", "g64x", "glv", "blk", "264", "265", "h264",
    "h265", "hevc",
];

pub fn scan_folder(
    case_dir: &Path,
    source_dir: &Path,
    options: &ScanOptions,
) -> Result<ScanResult, String> {
    let source_dir = canonicalize_display(source_dir)
        .map_err(|err| format!("failed to canonicalize source: {err}"))?;
    let excluded_dirs = excluded_case_dirs(case_dir, &source_dir)?;
    let collection = collect_video_candidates(&source_dir, options.max_depth, &excluded_dirs)?;
    let mut id_registry = load_existing_video_ids(case_dir)?;
    let mut records = Vec::with_capacity(collection.files.len());
    let mut total_bytes = 0u64;

    for path in collection.files {
        let metadata = fs::metadata(&path)
            .map_err(|err| format!("failed to read metadata for {}: {err}", path.display()))?;
        total_bytes = total_bytes.saturating_add(metadata.len());
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let relative_path = path
            .strip_prefix(&source_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        let (sha256, hash_status) = if options.hash_files {
            let file = File::open(&path)
                .map_err(|err| format!("failed to open {} for hashing: {err}", path.display()))?;
            let digest = sha256::digest_reader(BufReader::new(file))
                .map_err(|err| format!("failed to hash {}: {err}", path.display()))?;
            (Some(digest), "complete".to_string())
        } else {
            (None, "skipped".to_string())
        };

        let probe = if options.use_ffprobe {
            ffprobe::probe(&path)
        } else {
            ProbeSummary::skipped()
        };

        let confidence = classify_confidence(&extension, &probe);
        let source_profile = detector::detect_source_profile(
            &relative_path,
            &extension,
            probe.format_name.as_deref(),
        );
        let id = id_registry.id_for(&path);
        records.push(VideoRecord {
            id,
            source_path: path,
            relative_path,
            extension,
            size_bytes: metadata.len(),
            modified_unix: modified_unix(&metadata),
            sha256,
            hash_status,
            probe,
            confidence,
            source_profile,
        });
    }

    let result = ScanResult {
        source_path: source_dir,
        scanned_unix: now_unix()?,
        video_count: records.len(),
        total_bytes,
        warnings: collection.warnings,
        options: options.clone(),
        records,
    };

    write_scan_outputs(case_dir, &result)?;
    Ok(result)
}

struct CandidateCollection {
    files: Vec<PathBuf>,
    warnings: Vec<String>,
}

fn collect_video_candidates(
    source_dir: &Path,
    max_depth: Option<usize>,
    excluded_dirs: &[PathBuf],
) -> Result<CandidateCollection, String> {
    let mut out = Vec::new();
    let mut warnings = Vec::new();
    let mut queue = VecDeque::from([(source_dir.to_path_buf(), 0usize)]);

    while let Some((dir, depth)) = queue.pop_front() {
        if max_depth.is_some_and(|max| depth > max) {
            continue;
        }
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) if depth == 0 => {
                return Err(format!("failed to read directory {}: {err}", dir.display()));
            }
            Err(err) => {
                warnings.push(format!(
                    "skipped unreadable directory {}: {err}",
                    dir.display()
                ));
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    warnings.push(format!("skipped unreadable directory entry: {err}"));
                    continue;
                }
            };
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(err) => {
                    warnings.push(format!(
                        "skipped unreadable file type {}: {err}",
                        path.display()
                    ));
                    continue;
                }
            };
            if file_type.is_dir() {
                if path_is_or_is_under_excluded_dir(&path, excluded_dirs) {
                    warnings.push(format!(
                        "skipped FrameTrace case output directory {}",
                        path.display()
                    ));
                    continue;
                }
                queue.push_back((path, depth + 1));
            } else if file_type.is_file() && looks_like_video(&path) {
                out.push(path);
            }
        }
    }

    out.sort();
    Ok(CandidateCollection {
        files: out,
        warnings,
    })
}

fn excluded_case_dirs(case_dir: &Path, source_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let case_dir = case_dir
        .canonicalize()
        .unwrap_or_else(|_| case_dir.to_path_buf());
    if case_dir == source_dir {
        return Err(format!(
            "source directory cannot be the FrameTrace case directory: {}",
            source_dir.display()
        ));
    }
    if case_dir.starts_with(source_dir) {
        Ok(vec![case_dir])
    } else {
        Ok(Vec::new())
    }
}

fn path_is_or_is_under_excluded_dir(path: &Path, excluded_dirs: &[PathBuf]) -> bool {
    excluded_dirs
        .iter()
        .any(|excluded| path == excluded || path.starts_with(excluded))
}

fn looks_like_video(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
        return has_video_magic(path);
    };
    VIDEO_EXTENSIONS
        .iter()
        .any(|known| extension.eq_ignore_ascii_case(known))
        || has_video_magic(path)
}

fn has_video_magic(path: &Path) -> bool {
    let file = File::open(path);
    let Ok(file) = file else {
        return false;
    };
    let mut reader = BufReader::new(file);
    let buffer = reader.fill_buf().unwrap_or(&[]);
    if buffer.len() < 12 {
        return false;
    }

    // `ftyp` must sit in an MP4 box header at the file start (4-byte size then
    // `ftyp`). Scanning the whole buffer would misclassify any file whose text
    // merely contains "ftyp", such as our own JSONL audit logs.
    buffer.len() >= 8 && &buffer[4..8] == b"ftyp"
        || buffer.starts_with(&[0x00, 0x00, 0x00, 0x01])
        || buffer.starts_with(&[0x00, 0x00, 0x01])
        || buffer.starts_with(b"RIFF")
        || buffer.starts_with(b"IMKH")
        || buffer.starts_with(b"DHAV")
}

fn modified_unix(metadata: &fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
}

fn classify_confidence(extension: &str, probe: &ProbeSummary) -> String {
    if probe.ok {
        "ffprobe-confirmed".to_string()
    } else if VIDEO_EXTENSIONS.contains(&extension) {
        "extension-candidate".to_string()
    } else {
        "magic-candidate".to_string()
    }
}

fn write_scan_outputs(case_dir: &Path, result: &ScanResult) -> Result<(), String> {
    let run_path = case_dir
        .join("db/scan_runs")
        .join(format!("scan_{}.json", result.scanned_unix));
    let run_path = unique_path(&run_path);
    write_text_atomic(&run_path, &result.to_json())
        .map_err(|err| format!("failed to write scan run snapshot: {err}"))?;

    let merged_records = merge_existing_with_scan(case_dir, result)?;
    let index_json = scan_index_json(result, &merged_records);
    write_text_atomic(&case_dir.join("db/video_index.json"), &index_json)
        .map_err(|err| format!("failed to write video index: {err}"))?;

    let mut jsonl = String::new();
    let mut paths_tsv = String::from(
        "id\tsource_path\trelative_path\textension\tsize_bytes\tsha256\tvendor\tparser\tparser_confidence\n",
    );
    for record in &merged_records {
        jsonl.push_str(&record.json_line);
        jsonl.push('\n');
        paths_tsv.push_str(&record.to_tsv_row());
    }
    write_text_atomic(&case_dir.join("db/videos.jsonl"), &jsonl)
        .map_err(|err| format!("failed to write video jsonl: {err}"))?;
    write_text_atomic(&case_dir.join("db/video_paths.tsv"), &paths_tsv)
        .map_err(|err| format!("failed to write video path index: {err}"))?;

    let db_records = merged_records
        .iter()
        .map(IndexedRecordLine::to_db_row)
        .collect::<Vec<_>>();
    case_db::write_scan_index(case_dir, result, &db_records)?;
    Ok(())
}

#[derive(Debug, Clone)]
struct IdRegistry {
    ids_by_source: HashMap<String, String>,
    next_number: usize,
}

impl IdRegistry {
    fn id_for(&mut self, path: &Path) -> String {
        let source_path = normalize_source_key(&path.to_string_lossy());
        if let Some(id) = self.ids_by_source.get(&source_path) {
            return id.clone();
        }

        let id = format!("vid_{:06}", self.next_number);
        self.next_number += 1;
        self.ids_by_source.insert(source_path, id.clone());
        id
    }
}

/// Older case indexes stored source paths with the Windows extended-length
/// prefix (`\\?\`), newer ones store clean paths. Matching on the normalized
/// form keeps video ids stable across binary upgrades.
fn normalize_source_key(source_path: &str) -> String {
    crate::util::strip_windows_extended_prefix(Path::new(source_path))
        .to_string_lossy()
        .to_string()
}

#[derive(Debug, Clone)]
struct IndexedRecordLine {
    id: String,
    source_path: String,
    /// Field order and spelling must stay identical to the published
    /// JSONL contract, so the serialized form is kept verbatim and the
    /// structured fields are derived through serde.
    json_line: String,
    record: VideoRecord,
}

impl IndexedRecordLine {
    fn from_record(record: &VideoRecord) -> Self {
        Self {
            id: record.id.clone(),
            source_path: record.source_path.to_string_lossy().to_string(),
            json_line: record.to_json(),
            record: record.clone(),
        }
    }

    fn to_tsv_row(&self) -> String {
        let profile = &self.record.source_profile;
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            tsv_escape(&self.id),
            tsv_escape(&self.source_path),
            tsv_escape(&self.record.relative_path),
            tsv_escape(&self.record.extension),
            self.record.size_bytes,
            tsv_escape(self.record.sha256.as_deref().unwrap_or("")),
            tsv_escape(&profile.vendor),
            tsv_escape(&profile.parser),
            tsv_escape(&profile.confidence)
        )
    }

    fn to_db_row(&self) -> IndexedVideoRow {
        let probe = &self.record.probe;
        IndexedVideoRow {
            id: self.id.clone(),
            source_path: self.source_path.clone(),
            file_url: path_to_file_url(&self.record.source_path),
            relative_path: self.record.relative_path.clone(),
            extension: self.record.extension.clone(),
            size_bytes: self.record.size_bytes,
            modified_unix: self.record.modified_unix,
            sha256: self.record.sha256.clone(),
            hash_status: clone_or(&self.record.hash_status, "unknown"),
            confidence: clone_or(&self.record.confidence, "unknown"),
            source_profile_json: self.record.source_profile.to_json(),
            duration_seconds: probe.duration_seconds,
            format_name: probe.format_name.clone(),
            video_codec: probe.video_codec.clone(),
            audio_codec: probe.audio_codec.clone(),
            width: probe.width.map(u64::from),
            height: probe.height.map(u64::from),
            ffprobe_ok: probe.ok,
            ffprobe_error: probe.error.clone(),
            ffprobe_json: probe.raw_json.clone(),
            record_json: self.json_line.clone(),
        }
    }
}

fn clone_or(value: &str, fallback: &str) -> String {
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn load_existing_video_ids(case_dir: &Path) -> Result<IdRegistry, String> {
    let existing = load_existing_record_lines(case_dir)?;
    let mut ids_by_source = HashMap::new();
    let mut max_number = 0usize;

    for record in case_db::load_video_ids(case_dir)? {
        if let Some(number) = record.id.strip_prefix("vid_").and_then(parse_usize) {
            max_number = max_number.max(number);
        }
        ids_by_source.insert(normalize_source_key(&record.source_path), record.id);
    }

    for record in existing {
        if let Some(number) = record.id.strip_prefix("vid_").and_then(parse_usize) {
            max_number = max_number.max(number);
        }
        ids_by_source.insert(normalize_source_key(&record.source_path), record.id);
    }

    Ok(IdRegistry {
        ids_by_source,
        next_number: max_number + 1,
    })
}

fn merge_existing_with_scan(
    case_dir: &Path,
    result: &ScanResult,
) -> Result<Vec<IndexedRecordLine>, String> {
    let current_by_source = result
        .records
        .iter()
        .map(IndexedRecordLine::from_record)
        .map(|record| (normalize_source_key(&record.source_path), record))
        .collect::<HashMap<_, _>>();

    let mut merged = Vec::new();
    let mut updated_keys = std::collections::HashSet::new();
    for existing in load_existing_record_lines(case_dir)? {
        let key = normalize_source_key(&existing.source_path);
        if let Some(updated) = current_by_source.get(&key) {
            // Legacy and clean spellings of one file may both exist in old
            // indexes; converge to a single refreshed record.
            if updated_keys.insert(key.clone()) {
                merged.push(updated.clone());
            }
        } else {
            merged.push(existing.mark_stale(result.scanned_unix));
        }
    }

    let existing_sources = merged
        .iter()
        .map(|record| normalize_source_key(&record.source_path))
        .collect::<std::collections::HashSet<_>>();
    for record in result.records.iter().map(IndexedRecordLine::from_record) {
        if !existing_sources.contains(&normalize_source_key(&record.source_path)) {
            merged.push(record);
        }
    }

    merged.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(merged)
}

fn load_existing_record_lines(case_dir: &Path) -> Result<Vec<IndexedRecordLine>, String> {
    let path = case_dir.join("db/videos.jsonl");
    let text = match read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };

    let mut records = Vec::new();
    for (line_index, line) in json_record_lines(&text).into_iter().enumerate() {
        let line = line.trim();
        let record: VideoRecord = serde_json::from_str(line).map_err(|err| {
            format!(
                "failed to parse {} record {}: {err}",
                path.display(),
                line_index + 1
            )
        })?;
        records.push(IndexedRecordLine {
            id: record.id.clone(),
            source_path: record.source_path.to_string_lossy().to_string(),
            json_line: line.to_string(),
            record,
        });
    }
    Ok(records)
}

impl IndexedRecordLine {
    fn mark_stale(mut self, stale_since_unix: u64) -> Self {
        // Stale markers are appended to the stored JSONL verbatim line so the
        // published record contract (field order, spelling) stays byte-stable
        // for already-indexed evidence. serde_json re-serialization would
        // reorder fields, so the edit stays textual.
        self.json_line = set_json_field(
            &set_json_field(&self.json_line, "index_status", "\"stale\""),
            "stale_since_unix",
            &stale_since_unix.to_string(),
        );
        self
    }
}

/// Appends or replaces one top-level scalar field on a serialized JSON object
/// through raw text edits only, so existing byte-stable records keep their
/// field order and spelling.
fn set_json_field(line: &str, key: &str, value: &str) -> String {
    if find_top_level_key(line, key).is_some() {
        replace_json_field(line, key, value)
    } else {
        insert_json_field(line, key, value)
    }
}

/// Byte range of `"key":<value>` at JSON depth 1 (top-level object members
/// only). Nested occurrences — e.g. a metadata tag named `index_status`
/// inside the inlined ffprobe object — are deliberately not matched, so
/// stale markers can never rewrite recorded probe evidence.
fn find_top_level_key(line: &str, key: &str) -> Option<std::ops::Range<usize>> {
    let bytes = line.as_bytes();
    let needle = format!("\"{key}\":");
    let needle = needle.as_bytes();
    let mut depth: usize = 0;
    let mut in_string = false;
    let mut escaped = false;
    let mut index = 0usize;

    while index < bytes.len() {
        let ch = bytes[index];
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == b'\\' {
                escaped = true;
            } else if ch == b'"' {
                in_string = false;
            }
            index += 1;
            continue;
        }
        match ch {
            b'"' => in_string = true,
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        // A member key starts at depth 1 right after a `{` or `,`.
        if depth == 1
            && (bytes[..=index].ends_with(b"{") || bytes[..=index].ends_with(b","))
            && bytes[index + 1..].starts_with(needle)
        {
            let value_start = index + 1 + needle.len();
            let value = extract_scalar_value(&line[value_start..])?;
            let range = value_start..value_start + value.len();
            return Some(range);
        }
        index += 1;
    }
    None
}

/// Consumes one scalar JSON value (string, number, bool, null) from the
/// start of `text` and returns its raw serialized form.
fn extract_scalar_value(text: &str) -> Option<String> {
    let trimmed = text.trim_start();
    let offset = text.len() - trimmed.len();
    let mut chars = trimmed.chars();
    match chars.next()? {
        '"' => {
            let mut out = String::from("\"");
            let mut escaped = false;
            for ch in trimmed[1..].chars() {
                out.push(ch);
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    return Some(format!("{}{}", &text[..offset], &out));
                }
            }
            None
        }
        first if first == '-' || first.is_ascii_digit() => {
            let raw: String = std::iter::once(first)
                .chain(chars.take_while(|ch| {
                    ch.is_ascii_digit() || matches!(ch, '.' | 'e' | 'E' | '+' | '-')
                }))
                .collect();
            (!raw.is_empty()).then(|| format!("{}{}", &text[..offset], raw))
        }
        't' if trimmed.starts_with("true") => Some(format!("{}true", &text[..offset])),
        'f' if trimmed.starts_with("false") => Some(format!("{}false", &text[..offset])),
        'n' if trimmed.starts_with("null") => Some(format!("{}null", &text[..offset])),
        _ => None,
    }
}

fn replace_json_field(line: &str, key: &str, value: &str) -> String {
    let Some(range) = find_top_level_key(line, key) else {
        return insert_json_field(line, key, value);
    };
    format!("{}{}{}", &line[..range.start], value, &line[range.end..])
}

fn insert_json_field(line: &str, key: &str, value: &str) -> String {
    let trimmed = line.trim_end();
    let Some(close_index) = trimmed.rfind('}') else {
        return line.to_string();
    };
    let prefix = &trimmed[..close_index];
    let suffix = &trimmed[close_index..];
    let separator = if prefix.trim_end().ends_with('{') {
        ""
    } else {
        ","
    };
    format!("{prefix}{separator}\"{key}\":{value}{suffix}")
}

fn json_record_lines(text: &str) -> Vec<String> {
    let mut records = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for ch in text.chars() {
        if depth > 0 {
            current.push(ch);
        } else if ch.is_whitespace() {
            continue;
        } else {
            current.push(ch);
        }

        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 && !current.trim().is_empty() {
                    records.push(current.trim().to_string());
                    current.clear();
                }
            }
            _ => {}
        }
    }

    if !current.trim().is_empty() {
        records.push(current.trim().to_string());
    }
    records
}

fn scan_index_json(result: &ScanResult, records: &[IndexedRecordLine]) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"schema_version\": 1,\n");
    out.push_str(&format!(
        "  \"source_path\": \"{}\",\n",
        json_escape(&result.source_path.to_string_lossy())
    ));
    out.push_str(&format!("  \"scanned_unix\": {},\n", result.scanned_unix));
    out.push_str(&format!("  \"video_count\": {},\n", records.len()));
    out.push_str(&format!(
        "  \"total_bytes\": {},\n",
        records
            .iter()
            .map(|record| record.record.size_bytes)
            .sum::<u64>()
    ));
    out.push_str("  \"warnings\": [\n");
    for (index, warning) in result.warnings.iter().enumerate() {
        out.push_str(&format!("    \"{}\"", json_escape(warning)));
        if index + 1 != result.warnings.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ],\n");
    out.push_str("  \"options\": {\n");
    out.push_str(&format!(
        "    \"hash_files\": {},\n",
        result.options.hash_files
    ));
    out.push_str(&format!(
        "    \"use_ffprobe\": {},\n",
        result.options.use_ffprobe
    ));
    match result.options.max_depth {
        Some(max_depth) => out.push_str(&format!("    \"max_depth\": {}\n", max_depth)),
        None => out.push_str("    \"max_depth\": null\n"),
    }
    out.push_str("  },\n");
    out.push_str("  \"videos\": [\n");
    for (index, record) in records.iter().enumerate() {
        out.push_str("    ");
        out.push_str(&record.json_line);
        if index + 1 != records.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n");
    out.push_str("}\n");
    out
}

fn parse_usize(value: &str) -> Option<usize> {
    value.parse::<usize>().ok()
}

fn tsv_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::{
        collect_video_candidates, excluded_case_dirs, looks_like_video, merge_existing_with_scan,
        set_json_field,
    };
    use crate::model::{ProbeSummary, ScanOptions, ScanResult, SourceProfile, VideoRecord};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn set_json_field_appends_without_reordering_existing_keys() {
        // The JSONL contract is byte-stable for already-indexed evidence:
        // stale markers must not rewrite the whole object.
        let line = r#"{"z_last":1,"a_first":"x"}"#;
        let updated = set_json_field(line, "index_status", "\"stale\"");
        let expected = r#"{"z_last":1,"a_first":"x","index_status":"stale"}"#;
        assert_eq!(updated, expected);
    }

    #[test]
    fn set_json_field_replaces_existing_scalar_in_place() {
        let line = r#"{"id":"vid_1","stale_since_unix":5,"ext":"mp4"}"#;
        let updated = set_json_field(line, "stale_since_unix", "9");
        assert_eq!(
            updated,
            r#"{"id":"vid_1","stale_since_unix":9,"ext":"mp4"}"#
        );
    }

    /// A nested key (e.g. an ffprobe metadata tag) sharing the stale
    /// marker's name must never be touched: only depth-1 members match.
    /// Evidence files can carry tags named `index_status` planted via
    /// `ffmpeg -metadata`, so this is an adversarial-input guard.
    #[test]
    fn set_json_field_ignores_nested_keys_with_the_same_name() {
        let line = r#"{"id":"vid_1","ffprobe":{"index_status":"attacker tag","streams":[]},"index_status":null,"ext":"mp4"}"#;
        let updated = set_json_field(line, "index_status", "\"stale\"");
        assert!(
            updated.contains(r#""ffprobe":{"index_status":"attacker tag""#),
            "nested tag must stay byte-identical: {updated}"
        );
        assert!(updated.contains(r#""index_status":"stale""#), "{updated}");
    }

    #[test]
    fn set_json_field_appends_when_top_level_key_missing() {
        let line = r#"{"id":"vid_1","ffprobe":{"index_status":"nested decoy"}}"#;
        let updated = set_json_field(line, "stale_since_unix", "7");
        assert!(updated.ends_with(r#","stale_since_unix":7}"#), "{updated}");
        assert!(updated.contains(r#""ffprobe":{"index_status":"nested decoy"}"#));
    }

    #[test]
    fn recognizes_common_video_extensions() {
        assert!(looks_like_video(&PathBuf::from("a.MP4")));
        assert!(looks_like_video(&PathBuf::from("camera.dav")));
        assert!(looks_like_video(&PathBuf::from("export.g64x")));
        assert!(looks_like_video(&PathBuf::from("phone.glv")));
        assert!(!looks_like_video(&PathBuf::from("notes.txt")));
    }

    #[test]
    fn recognizes_mp4_magic_without_extension() {
        let path = std::env::temp_dir().join(format!(
            "forensic-video-workstation-test-{}",
            std::process::id()
        ));
        fs::write(&path, b"\0\0\0\x18ftypmp42").unwrap();
        assert!(looks_like_video(&path));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_files_that_mention_ftyp_in_text() {
        let path = std::env::temp_dir().join(format!(
            "frametrace-false-magic-test-{}.jsonl",
            std::process::id()
        ));
        fs::write(
            &path,
            r#"{"signature":"mp4-ftyp","note":"audit text mentioning ftyp boxes"}"#,
        )
        .unwrap();
        assert!(!looks_like_video(&path));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_scanning_the_case_directory_as_source() {
        let root = std::env::temp_dir().join(format!(
            "frametrace-case-source-reject-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let canonical = root.canonicalize().unwrap();
        let err = excluded_case_dirs(&canonical, &canonical).unwrap_err();
        assert!(err.contains("source directory cannot be the FrameTrace case directory"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn collect_candidates_skips_excluded_case_output_tree() {
        let root = std::env::temp_dir().join(format!(
            "frametrace-scan-exclusion-test-{}",
            std::process::id()
        ));
        let source_dir = root.join("source");
        let case_dir = source_dir.join("case");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(source_dir.join("camera")).unwrap();
        fs::create_dir_all(case_dir.join("artifacts/clips")).unwrap();
        fs::write(source_dir.join("camera/real.mp4"), b"not actually media").unwrap();
        fs::write(
            case_dir.join("artifacts/clips/derived.mp4"),
            b"not actually media",
        )
        .unwrap();

        let collection =
            collect_video_candidates(&source_dir, None, std::slice::from_ref(&case_dir)).unwrap();
        let names = collection
            .files
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["real.mp4"]);
        assert!(
            collection
                .warnings
                .iter()
                .any(|warning| warning.contains("skipped FrameTrace case output directory"))
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn deserializes_published_jsonl_records_through_serde() {
        let line = r#"{"id":"vid_000001","source_path":"C:\\Evidence\\a\tb.mp4","relative_path":"a\tb.mp4","extension":"mp4","size_bytes":1,"modified_unix":null,"sha256":null,"hash_status":"skipped","confidence":"extension-candidate","source_profile":{"lane":"generic-video","vendor":"Generic media","parser":"generic_media","confidence":"medium","recommended_action":"Use ffprobe/FFmpeg first; preserve original and export derived clips only when requested.","evidence":["extension"]},"duration_seconds":12.5,"format_name":"mov,mp4,m4a","video_codec":"h264","audio_codec":null,"width":1920,"height":1080,"ffprobe_ok":true,"ffprobe_error":null,"ffprobe":null}"#;
        let record: VideoRecord = serde_json::from_str(line).unwrap();
        assert_eq!(record.id, "vid_000001");
        assert_eq!(record.source_path, PathBuf::from("C:\\Evidence\\a\tb.mp4"));
        assert!(record.probe.ok);
        assert_eq!(record.probe.width, Some(1920));
        assert_eq!(record.probe.duration_seconds, Some(12.5));

        // Round-trip keeps the published contract parseable.
        let reparsed: VideoRecord = serde_json::from_str(&record.to_json()).unwrap();
        assert_eq!(reparsed.id, record.id);
        assert_eq!(reparsed.probe.width, record.probe.width);
    }

    #[test]
    fn merges_scan_records_without_dropping_existing_case_index() {
        let case_dir =
            std::env::temp_dir().join(format!("frametrace-merge-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&case_dir);
        fs::create_dir_all(case_dir.join("db")).unwrap();

        let first = VideoRecord {
            id: "vid_000001".to_string(),
            source_path: PathBuf::from("/evidence/one.mp4"),
            relative_path: "one.mp4".to_string(),
            extension: "mp4".to_string(),
            size_bytes: 1,
            modified_unix: None,
            sha256: None,
            hash_status: "skipped".to_string(),
            probe: ProbeSummary::skipped(),
            confidence: "extension-candidate".to_string(),
            source_profile: SourceProfile::generic_media("test"),
        };
        let second = VideoRecord {
            id: "vid_000002".to_string(),
            source_path: PathBuf::from("/evidence/two.mp4"),
            relative_path: "two.mp4".to_string(),
            extension: "mp4".to_string(),
            size_bytes: 2,
            modified_unix: None,
            sha256: None,
            hash_status: "skipped".to_string(),
            probe: ProbeSummary::skipped(),
            confidence: "extension-candidate".to_string(),
            source_profile: SourceProfile::generic_media("test"),
        };
        fs::write(
            case_dir.join("db/videos.jsonl"),
            format!("{}\n{}\n", first.to_json(), second.to_json()),
        )
        .unwrap();

        let rescanned_second = VideoRecord {
            sha256: Some("abc".to_string()),
            hash_status: "complete".to_string(),
            ..second
        };
        let result = ScanResult {
            source_path: PathBuf::from("/evidence"),
            scanned_unix: 1,
            video_count: 1,
            total_bytes: 2,
            warnings: Vec::new(),
            options: ScanOptions::default(),
            records: vec![rescanned_second],
        };

        let merged = merge_existing_with_scan(&case_dir, &result).unwrap();
        assert_eq!(merged.len(), 2);
        assert!(merged[0].json_line.contains("\"id\":\"vid_000001\""));
        assert!(merged[0].json_line.contains("\"index_status\":\"stale\""));
        assert!(merged[0].json_line.contains("\"stale_since_unix\":1"));
        assert!(merged[1].json_line.contains("\"id\":\"vid_000002\""));
        assert!(merged[1].json_line.contains("\"sha256\":\"abc\""));

        let _ = fs::remove_dir_all(case_dir);
    }

    #[test]
    fn upgrades_legacy_extended_prefix_paths_without_duplicating_ids() {
        let case_dir = std::env::temp_dir().join(format!(
            "frametrace-legacy-path-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&case_dir);
        fs::create_dir_all(case_dir.join("db")).unwrap();

        let legacy = VideoRecord {
            id: "vid_000001".to_string(),
            source_path: PathBuf::from(r"\\?\C:\evidence\one.mp4"),
            relative_path: "one.mp4".to_string(),
            extension: "mp4".to_string(),
            size_bytes: 1,
            modified_unix: None,
            sha256: None,
            hash_status: "skipped".to_string(),
            probe: ProbeSummary::skipped(),
            confidence: "extension-candidate".to_string(),
            source_profile: SourceProfile::generic_media("test"),
        };
        fs::write(
            case_dir.join("db/videos.jsonl"),
            format!("{}\n", legacy.to_json()),
        )
        .unwrap();

        let rescanned = VideoRecord {
            source_path: PathBuf::from(r"C:\evidence\one.mp4"),
            sha256: Some("abc".to_string()),
            hash_status: "complete".to_string(),
            ..legacy
        };
        let result = ScanResult {
            source_path: PathBuf::from(r"C:\evidence"),
            scanned_unix: 2,
            video_count: 1,
            total_bytes: 1,
            warnings: Vec::new(),
            options: ScanOptions::default(),
            records: vec![rescanned],
        };

        let merged = merge_existing_with_scan(&case_dir, &result).unwrap();
        assert_eq!(merged.len(), 1, "same file must not duplicate records");
        assert!(merged[0].json_line.contains("\"id\":\"vid_000001\""));
        assert!(merged[0].json_line.contains(r"C:\\evidence\\one.mp4"));
        assert!(!merged[0].json_line.contains("index_status"));

        let _ = fs::remove_dir_all(case_dir);
    }
}
