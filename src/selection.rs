use crate::util::read_to_string;
use std::path::Path;

pub const SELECTION_SCHEMA_VERSION: u32 = 1;
pub const MARKS_SCHEMA_VERSION: u32 = 1;

pub const MARK_STATUSES: &[&str] = &["reviewed", "important", "needs_verification"];

#[derive(Debug, Clone, PartialEq)]
pub struct SelectionItem {
    pub selector: String,
    pub kind: Option<String>,
    pub action: Option<String>,
    pub format: Option<String>,
    pub time_seconds: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectionFile {
    pub case_id: Option<String>,
    pub items: Vec<SelectionItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MarkEntry {
    pub id: String,
    pub status: String,
    pub marked_unix: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MarksFile {
    pub case_id: Option<String>,
    pub marks: Vec<MarkEntry>,
}

/// Minimal JSON field/array accessors reused by other modules (e.g. reading
/// the video index for thumbnail generation).
pub(crate) fn json_array_field(text: &str, key: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()?
        .get(key)?
        .as_array()
        .map(|array| serde_json::Value::Array(array.clone()).to_string())
}

pub(crate) fn json_objects_in_array(array_text: &str) -> Vec<String> {
    serde_json::from_str::<serde_json::Value>(array_text)
        .ok()
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter(|item| item.is_object())
        .map(|item| item.to_string())
        .collect()
}

pub(crate) fn json_string_field(line: &str, key: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(line)
        .ok()?
        .get(key)?
        .as_str()
        .map(str::to_string)
}

pub fn parse_selection_file(path: &Path) -> Result<SelectionFile, String> {
    let text = read_to_string(path)
        .map_err(|err| format!("failed to read selection file {}: {err}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|err| format!("failed to parse selection file {}: {err}", path.display()))?;
    let case_id = value
        .get("case_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let items_value = value
        .get("items")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("selection file {} is missing items", path.display()))?;
    let mut items = Vec::new();
    for (index, item) in items_value.iter().enumerate() {
        let selector = item
            .get("selector")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                format!(
                    "selection item {} in {} is missing selector",
                    index + 1,
                    path.display()
                )
            })?
            .to_string();
        if selector.trim().is_empty() {
            return Err(format!(
                "selection item {} in {} has an empty selector",
                index + 1,
                path.display()
            ));
        }
        let field = |key: &str| {
            item.get(key)
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        };
        items.push(SelectionItem {
            selector,
            kind: field("kind"),
            action: field("action"),
            format: field("format"),
            time_seconds: item
                .get("time_seconds")
                .and_then(serde_json::Value::as_f64)
                .filter(|value| value.is_finite()),
            notes: field("notes"),
        });
    }
    if items.is_empty() {
        return Err(format!("selection file {} has no items", path.display()));
    }
    Ok(SelectionFile { case_id, items })
}

pub fn parse_marks_file(path: &Path) -> Result<MarksFile, String> {
    let text = read_to_string(path)
        .map_err(|err| format!("failed to read marks file {}: {err}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|err| format!("failed to parse marks file {}: {err}", path.display()))?;
    let case_id = value
        .get("case_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let marks_value = value
        .get("marks")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("marks file {} is missing marks", path.display()))?;
    let mut marks = Vec::new();
    for (index, item) in marks_value.iter().enumerate() {
        let id = item
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                format!(
                    "marks entry {} in {} is missing id",
                    index + 1,
                    path.display()
                )
            })?
            .to_string();
        let status = item
            .get("status")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                format!(
                    "marks entry {} in {} is missing status",
                    index + 1,
                    path.display()
                )
            })?
            .to_string();
        if !MARK_STATUSES.contains(&status.as_str()) {
            return Err(format!(
                "marks entry {} in {} has unsupported status '{}' (expected one of {})",
                index + 1,
                path.display(),
                status,
                MARK_STATUSES.join(", ")
            ));
        }
        marks.push(MarkEntry {
            id,
            status,
            marked_unix: item.get("marked_unix").and_then(serde_json::Value::as_u64),
        });
    }
    if marks.is_empty() {
        return Err(format!("marks file {} has no marks", path.display()));
    }
    Ok(MarksFile { case_id, marks })
}

/// Effective action for a selection item when the file omits it.
pub fn effective_action(item: &SelectionItem) -> &'static str {
    match item.action.as_deref() {
        Some("export") => "export",
        Some("proxy") => "proxy",
        Some("thumbnail") => "thumbnail",
        Some("validate") => "validate",
        _ => match item.kind.as_deref() {
            Some("carved") | Some("filesystem") => "validate",
            _ => "export",
        },
    }
}

pub fn effective_format(item: &SelectionItem) -> Result<&'static str, String> {
    match item.format.as_deref() {
        None => Ok("mp4"),
        Some(raw) => match raw.to_ascii_lowercase().as_str() {
            "mp4" => Ok("mp4"),
            "avi" => Ok("avi"),
            other => Err(format!(
                "unsupported export format '{other}' (use mp4 or avi)"
            )),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{effective_action, effective_format, parse_marks_file, parse_selection_file};
    use std::fs;
    use std::path::PathBuf;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "frametrace-selection-{name}-{}",
            std::process::id()
        ))
    }

    #[test]
    fn parses_selection_items() {
        let path = temp_path("parse");
        fs::write(
            &path,
            r#"{"schema_version":1,"case_id":"FT-1","items":[{"selector":"vid_000001","kind":"video","action":"export","format":"mp4","notes":"a\"b"},{"selector":"carve_000001","kind":"carved","action":"validate"}]}"#,
        )
        .unwrap();
        let selection = parse_selection_file(&path).unwrap();
        assert_eq!(selection.case_id.as_deref(), Some("FT-1"));
        assert_eq!(selection.items.len(), 2);
        assert_eq!(selection.items[0].selector, "vid_000001");
        assert_eq!(selection.items[0].notes.as_deref(), Some("a\"b"));
        assert_eq!(effective_action(&selection.items[0]), "export");
        assert_eq!(effective_action(&selection.items[1]), "validate");
        assert_eq!(effective_format(&selection.items[0]).unwrap(), "mp4");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn defaults_action_from_kind_and_rejects_bad_format() {
        let mut item = super::SelectionItem {
            selector: "vid_000009".to_string(),
            kind: Some("video".to_string()),
            action: None,
            format: None,
            time_seconds: None,
            notes: None,
        };
        assert_eq!(effective_action(&item), "export");
        item.kind = Some("carved".to_string());
        assert_eq!(effective_action(&item), "validate");
        item.format = Some("mkv".to_string());
        assert!(effective_format(&item).is_err());
    }

    #[test]
    fn rejects_selection_without_selector() {
        let path = temp_path("noselector");
        fs::write(&path, r#"{"schema_version":1,"items":[{"kind":"video"}]}"#).unwrap();
        let error = parse_selection_file(&path).unwrap_err();
        assert!(error.contains("missing selector"), "unexpected: {error}");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_marks_and_rejects_unknown_status() {
        let path = temp_path("marks");
        fs::write(
            &path,
            r#"{"schema_version":1,"case_id":"FT-1","marks":[{"id":"vid_000001","status":"important","marked_unix":100},{"id":"carve_000002","status":"reviewed"}]}"#,
        )
        .unwrap();
        let marks = parse_marks_file(&path).unwrap();
        assert_eq!(marks.marks.len(), 2);
        assert_eq!(marks.marks[0].marked_unix, Some(100));

        fs::write(&path, r#"{"marks":[{"id":"x","status":"bogus"}]}"#).unwrap();
        let error = parse_marks_file(&path).unwrap_err();
        assert!(error.contains("unsupported status"), "unexpected: {error}");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn shared_json_accessors_handle_escaped_and_nested_values() {
        let index = r#"{"videos":[{"id":"vid_1","source_path":"C:\\ev\\a.mp4"},{"id":"vid_2","source_path":"C:\\ev\\b.mp4"}]}"#;
        let items = super::json_array_field(index, "videos").unwrap();
        let objects = super::json_objects_in_array(&items);
        assert_eq!(objects.len(), 2);
        assert_eq!(
            super::json_string_field(&objects[0], "source_path").unwrap(),
            "C:\\ev\\a.mp4"
        );
        // A missing array key must be None, not an error.
        assert!(super::json_array_field(index, "missing").is_none());
        assert!(super::json_string_field(&objects[0], "missing").is_none());
    }
}
