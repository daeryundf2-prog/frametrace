//! Paged record APIs (/api/records, /api/records-meta) and filesystem-inspection lookup.

use super::*;

/// `GET /api/records?offset=0&limit=500&q=foo` — paged slice of the case
/// video index so the reviewer UI does not have to parse one giant inline
/// JSON block for a large case. `q` is a case-insensitive substring match
/// over id/paths/name; the JSON index stays the shape authority so the
/// API can never drift from what the viewer renders offline.
pub(crate) fn api_records(request: &Request, state: &SharedState) -> String {
    let Some(case_dir) = state_lock(state).case_dir.clone() else {
        return "{\"ok\":false,\"error\":\"no case open\"}".to_string();
    };
    let index_path = case_dir.join("db/video_index.json");
    let mtime = index_path
        .metadata()
        .and_then(|m| m.modified())
        .unwrap_or(UNIX_EPOCH);
    let index = {
        let mut guard = state_lock(state);
        match &guard.index_cache {
            Some((path, cached_mtime, value)) if *path == index_path && *cached_mtime == mtime => {
                value.clone()
            }
            _ => {
                let Ok(text) = crate::util::read_to_string(&index_path) else {
                    return "{\"ok\":false,\"error\":\"no case index yet\"}".to_string();
                };
                let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
                    return "{\"ok\":false,\"error\":\"case index is not valid JSON\"}".to_string();
                };
                guard.index_cache = Some((index_path, mtime, value.clone()));
                value
            }
        }
    };
    let empty = Vec::new();
    let videos = index
        .get("videos")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let query = query_value(&request.query, "q").map(|q| q.to_lowercase());
    let matches = |video: &&serde_json::Value| -> bool {
        let Some(q) = query.as_deref() else {
            return true;
        };
        [
            "id",
            "relative_path",
            "source_path",
            "name",
            "original_name",
        ]
        .iter()
        .filter_map(|key| video.get(*key).and_then(|v| v.as_str()))
        .any(|text| text.to_lowercase().contains(q))
    };
    let offset = query_value(&request.query, "offset")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    let limit = query_value(&request.query, "limit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(500)
        .clamp(1, 5000);
    let filtered: Vec<&serde_json::Value> = videos.iter().filter(matches).collect();
    let total = filtered.len();
    let page: Vec<serde_json::Value> = filtered
        .into_iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();
    format!(
        "{{\"ok\":true,\"total\":{total},\"offset\":{offset},\"videos\":{}}}",
        serde_json::to_string(&page).unwrap_or_else(|_| "[]".to_string())
    )
}

/// `GET /api/records-meta` — the non-record half of the viewer data
/// bundle (manifest, logs, thumbs map, annotations, deepfake reports) so
/// the paged API can fully replace the inline `__FRAMETRACE_DATA__`
/// payload when the viewer is served over the workstation.
pub(crate) fn api_records_meta(state: &SharedState) -> String {
    let Some(case_dir) = state_lock(state).case_dir.clone() else {
        return "{\"ok\":false,\"error\":\"no case open\"}".to_string();
    };
    let read_json = |path: PathBuf| -> serde_json::Value {
        crate::util::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or(serde_json::Value::Null)
    };
    let read_jsonl = |path: PathBuf| -> serde_json::Value {
        let items: Vec<serde_json::Value> = crate::util::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .collect();
        serde_json::Value::Array(items)
    };
    // The scan object minus the (paged) videos array.
    let mut scan = read_json(case_dir.join("db/video_index.json"));
    if let serde_json::Value::Object(map) = &mut scan {
        map.insert("videos".to_string(), serde_json::Value::Array(Vec::new()));
    }
    let fls_entries = std::fs::read_dir(case_dir.join("db/filesystem"))
        .ok()
        .and_then(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| {
                            name.starts_with("tsk-files-") && name.ends_with(".jsonl")
                        })
                })
                .max()
        })
        .map(read_jsonl)
        .unwrap_or(serde_json::Value::Array(Vec::new()));
    // Same thumb mapping the bundle generator emits: review/thumbs/<id>.jpg
    // relative to the served viewer page.
    let mut thumbs = serde_json::Map::new();
    if let Ok(entries) = std::fs::read_dir(case_dir.join("review/thumbs")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("jpg")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                thumbs.insert(
                    stem.to_string(),
                    serde_json::Value::String(format!("thumbs/{stem}.jpg")),
                );
            }
        }
    }
    let annotations = serde_json::json!({
        "marks": crate::case_db::load_review_marks(&case_dir).unwrap_or_default(),
        "tags": crate::case_db::load_review_tags(&case_dir).unwrap_or_default(),
    });
    let body = serde_json::json!({
        "ok": true,
        "manifest": read_json(case_dir.join("case.json")),
        "scan": scan,
        "carveLog": read_jsonl(case_dir.join("artifacts/carved/carve-log.jsonl")),
        "filesystemLog": read_jsonl(case_dir.join("evidence/logs/tsk-audit.jsonl")),
        "validationLog": read_jsonl(case_dir.join("evidence/logs/validation-log.jsonl")),
        "anomalyLog": read_jsonl(case_dir.join("evidence/logs/anomaly-log.jsonl")),
        "flsEntries": fls_entries,
        "thumbs": thumbs,
        "annotations": annotations,
        "deepfake": crate::deepfake::collect_reports(&case_dir),
        "telemetry": crate::telemetry::collect_reports(&case_dir),
    });
    serde_json::to_string(&body).unwrap_or_else(|_| "{\"ok\":false}".to_string())
}

/// Newest `db/filesystem/tsk-inspection-*.json` summary — carries the
/// image path and auto-selected partition offset the recover step needs.
pub(crate) fn newest_tsk_inspection(case_dir: &Path) -> Option<(PathBuf, u64)> {
    let path = std::fs::read_dir(case_dir.join("db/filesystem"))
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("tsk-inspection-") && name.ends_with(".json"))
        })
        .max()?;
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let image = value.get("image_path")?.as_str()?;
    let offset = value.get("partition_offset")?.as_u64()?;
    Some((PathBuf::from(image), offset))
}
