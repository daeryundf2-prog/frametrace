use crate::case_db::{self, IndexedVideoRow};
use crate::detector;
use crate::ffprobe;
use crate::model::{ProbeSummary, ScanOptions, ScanResult, VideoRecord};
use crate::sha256;
use crate::util::{json_escape, now_unix, read_to_string, unique_path, write_text};
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
    let source_dir = source_dir
        .canonicalize()
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

    buffer.windows(4).any(|window| window == b"ftyp")
        || buffer.starts_with(&[0x00, 0x00, 0x00, 0x01])
        || buffer.starts_with(&[0x00, 0x00, 0x01])
        || buffer.starts_with(b"RIFF")
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
    write_text(&run_path, &result.to_json())
        .map_err(|err| format!("failed to write scan run snapshot: {err}"))?;

    let merged_records = merge_existing_with_scan(case_dir, result)?;
    let index_json = scan_index_json(result, &merged_records);
    write_text(&case_dir.join("db/video_index.json"), &index_json)
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
    write_text(&case_dir.join("db/videos.jsonl"), &jsonl)
        .map_err(|err| format!("failed to write video jsonl: {err}"))?;
    write_text(&case_dir.join("db/video_paths.tsv"), &paths_tsv)
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
        let source_path = path.to_string_lossy().to_string();
        if let Some(id) = self.ids_by_source.get(&source_path) {
            return id.clone();
        }

        let id = format!("vid_{:06}", self.next_number);
        self.next_number += 1;
        self.ids_by_source.insert(source_path, id.clone());
        id
    }
}

#[derive(Debug, Clone)]
struct IndexedRecordLine {
    id: String,
    source_path: String,
    json_line: String,
}

impl IndexedRecordLine {
    fn from_record(record: &VideoRecord) -> Self {
        Self {
            id: record.id.clone(),
            source_path: record.source_path.to_string_lossy().to_string(),
            json_line: record.to_json(),
        }
    }

    fn to_tsv_row(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            tsv_escape(&self.id),
            tsv_escape(&self.source_path),
            tsv_escape(&extract_json_string(&self.json_line, "relative_path").unwrap_or_default()),
            tsv_escape(&extract_json_string(&self.json_line, "extension").unwrap_or_default()),
            extract_json_u64(&self.json_line, "size_bytes").unwrap_or(0),
            tsv_escape(&extract_json_string(&self.json_line, "sha256").unwrap_or_default()),
            tsv_escape(
                &extract_nested_json_string(&self.json_line, "source_profile", "vendor")
                    .unwrap_or_default()
            ),
            tsv_escape(
                &extract_nested_json_string(&self.json_line, "source_profile", "parser")
                    .unwrap_or_default()
            ),
            tsv_escape(
                &extract_nested_json_string(&self.json_line, "source_profile", "confidence")
                    .unwrap_or_default()
            ),
        )
    }

    fn to_db_row(&self) -> IndexedVideoRow {
        IndexedVideoRow {
            id: self.id.clone(),
            source_path: self.source_path.clone(),
            file_url: extract_json_string(&self.json_line, "file_url").unwrap_or_default(),
            relative_path: extract_json_string(&self.json_line, "relative_path")
                .unwrap_or_default(),
            extension: extract_json_string(&self.json_line, "extension").unwrap_or_default(),
            size_bytes: extract_json_u64(&self.json_line, "size_bytes").unwrap_or(0),
            modified_unix: extract_json_u64(&self.json_line, "modified_unix"),
            sha256: extract_json_string(&self.json_line, "sha256"),
            hash_status: extract_json_string(&self.json_line, "hash_status")
                .unwrap_or_else(|| "unknown".to_string()),
            confidence: extract_json_string(&self.json_line, "confidence")
                .unwrap_or_else(|| "unknown".to_string()),
            source_profile_json: extract_json_object(&self.json_line, "source_profile")
                .unwrap_or_else(|| "{}".to_string()),
            duration_seconds: extract_json_f64(&self.json_line, "duration_seconds"),
            format_name: extract_json_string(&self.json_line, "format_name"),
            video_codec: extract_json_string(&self.json_line, "video_codec"),
            audio_codec: extract_json_string(&self.json_line, "audio_codec"),
            width: extract_json_u64(&self.json_line, "width"),
            height: extract_json_u64(&self.json_line, "height"),
            ffprobe_ok: extract_json_bool(&self.json_line, "ffprobe_ok").unwrap_or(false),
            ffprobe_error: extract_json_string(&self.json_line, "ffprobe_error"),
            ffprobe_json: extract_json_value(&self.json_line, "ffprobe")
                .filter(|value| value.trim() != "null"),
            record_json: self.json_line.clone(),
        }
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
        ids_by_source.insert(record.source_path, record.id);
    }

    for record in existing {
        if let Some(number) = record.id.strip_prefix("vid_").and_then(parse_usize) {
            max_number = max_number.max(number);
        }
        ids_by_source.insert(record.source_path, record.id);
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
        .map(|record| (record.source_path.clone(), record))
        .collect::<HashMap<_, _>>();

    let mut merged = Vec::new();
    for existing in load_existing_record_lines(case_dir)? {
        if let Some(updated) = current_by_source.get(&existing.source_path) {
            merged.push(updated.clone());
        } else {
            merged.push(existing.mark_stale(result.scanned_unix));
        }
    }

    let existing_sources = merged
        .iter()
        .map(|record| record.source_path.clone())
        .collect::<std::collections::HashSet<_>>();
    for record in result.records.iter().map(IndexedRecordLine::from_record) {
        if !existing_sources.contains(&record.source_path) {
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
        let id = extract_json_string(line, "id").ok_or_else(|| {
            format!(
                "failed to parse id in {} record {}",
                path.display(),
                line_index + 1
            )
        })?;
        let source_path = extract_json_string(line, "source_path").ok_or_else(|| {
            format!(
                "failed to parse source_path in {} record {}",
                path.display(),
                line_index + 1
            )
        })?;
        records.push(IndexedRecordLine {
            id,
            source_path,
            json_line: line.to_string(),
        });
    }
    Ok(records)
}

impl IndexedRecordLine {
    fn mark_stale(mut self, stale_since_unix: u64) -> Self {
        self.json_line = set_json_field(
            &set_json_field(&self.json_line, "index_status", "\"stale\""),
            "stale_since_unix",
            &stale_since_unix.to_string(),
        );
        self
    }
}

fn set_json_field(line: &str, key: &str, value: &str) -> String {
    if extract_json_value(line, key).is_some() {
        replace_json_field(line, key, value)
    } else {
        insert_json_field(line, key, value)
    }
}

fn replace_json_field(line: &str, key: &str, value: &str) -> String {
    let needle = format!("\"{key}\":");
    let Some(key_start) = line.find(&needle) else {
        return insert_json_field(line, key, value);
    };
    let value_start = key_start + needle.len();
    let whitespace_len = line[value_start..]
        .chars()
        .take_while(|ch| ch.is_whitespace())
        .map(char::len_utf8)
        .sum::<usize>();
    let actual_value_start = value_start + whitespace_len;
    let Some(existing_value) = extract_json_value(line, key) else {
        return insert_json_field(line, key, value);
    };
    let actual_value_end = actual_value_start + existing_value.len();
    format!(
        "{}{}{}",
        &line[..actual_value_start],
        value,
        &line[actual_value_end..]
    )
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
            .map(|record| extract_json_u64(&record.json_line, "size_bytes").unwrap_or(0))
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

fn extract_nested_json_string(line: &str, object_key: &str, field_key: &str) -> Option<String> {
    let key = format!("\"{}\":{{", object_key);
    let start = line.find(&key)? + key.len() - 1;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, ch) in line[start..].char_indices() {
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
                if depth == 0 {
                    return extract_json_string(&line[start..start + offset + 1], field_key);
                }
            }
            _ => {}
        }
    }
    None
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
                'u' => {
                    let mut code = String::new();
                    for _ in 0..4 {
                        code.push(chars.next()?);
                    }
                    let code = u32::from_str_radix(&code, 16).ok()?;
                    out.push(char::from_u32(code)?);
                }
                other => out.push(other),
            },
            other => out.push(other),
        }
    }
    None
}

fn extract_json_object(line: &str, key: &str) -> Option<String> {
    extract_json_value(line, key).filter(|value| value.trim_start().starts_with('{'))
}

fn extract_json_value(line: &str, key: &str) -> Option<String> {
    let key = format!("\"{}\":", key);
    let start = line.find(&key)? + key.len();
    let value = line[start..].trim_start();
    let first = value.chars().next()?;

    match first {
        '"' => extract_quoted_json_value(value),
        '{' | '[' => extract_balanced_json_value(value, first),
        _ => {
            let end = value
                .find(|ch| [',', '}', ']'].contains(&ch))
                .unwrap_or(value.len());
            let raw = value[..end].trim();
            (!raw.is_empty()).then(|| raw.to_string())
        }
    }
}

fn extract_quoted_json_value(value: &str) -> Option<String> {
    let mut escaped = false;
    for (offset, ch) in value.char_indices().skip(1) {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(value[..offset + ch.len_utf8()].to_string());
        }
    }
    None
}

fn extract_balanced_json_value(value: &str, opener: char) -> Option<String> {
    let closer = if opener == '{' { '}' } else { ']' };
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, ch) in value.char_indices() {
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
            ch if ch == opener => depth += 1,
            ch if ch == closer => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(value[..offset + ch.len_utf8()].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn extract_json_u64(line: &str, key: &str) -> Option<u64> {
    let key = format!("\"{}\":", key);
    let start = line.find(&key)? + key.len();
    let value = line[start..].trim_start();
    let digits = value
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<u64>().ok()
    }
}

fn extract_json_f64(line: &str, key: &str) -> Option<f64> {
    let key = format!("\"{}\":", key);
    let start = line.find(&key)? + key.len();
    let value = line[start..].trim_start();
    let raw = value
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || matches!(ch, '.' | '-' | '+' | 'e' | 'E'))
        .collect::<String>();
    if raw.is_empty() {
        None
    } else {
        raw.parse::<f64>().ok().filter(|value| value.is_finite())
    }
}

fn extract_json_bool(line: &str, key: &str) -> Option<bool> {
    let key = format!("\"{}\":", key);
    let start = line.find(&key)? + key.len();
    let value = line[start..].trim_start();
    if value.starts_with("true") {
        Some(true)
    } else if value.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{
        collect_video_candidates, excluded_case_dirs, extract_json_string, looks_like_video,
        merge_existing_with_scan,
    };
    use crate::model::{ProbeSummary, ScanOptions, ScanResult, SourceProfile, VideoRecord};
    use std::fs;
    use std::path::PathBuf;

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
    fn extracts_escaped_json_string_values() {
        let line = r#"{"id":"vid_000001","source_path":"C:\\Evidence\\a\tb.mp4"}"#;
        assert_eq!(
            extract_json_string(line, "source_path").unwrap(),
            "C:\\Evidence\\a\tb.mp4"
        );
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
}
