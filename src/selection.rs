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

/// Minimal JSON field/array accessors reused by other modules that avoid a
/// serde dependency (e.g. reading the video index for thumbnail generation).
pub(crate) fn json_array_field(text: &str, key: &str) -> Option<String> {
    extract_json_value(text, key)
}

pub(crate) fn json_objects_in_array(array_text: &str) -> Vec<String> {
    json_object_lines(array_text)
}

pub(crate) fn json_string_field(line: &str, key: &str) -> Option<String> {
    extract_json_string(line, key)
}

pub fn parse_selection_file(path: &Path) -> Result<SelectionFile, String> {
    let text = read_to_string(path)
        .map_err(|err| format!("failed to read selection file {}: {err}", path.display()))?;
    let case_id = extract_json_string(&text, "case_id");
    let items_text = extract_json_value(&text, "items")
        .ok_or_else(|| format!("selection file {} is missing items", path.display()))?;
    let mut items = Vec::new();
    for (index, line) in json_object_lines(&items_text).into_iter().enumerate() {
        let selector = extract_json_string(&line, "selector").ok_or_else(|| {
            format!(
                "selection item {} in {} is missing selector",
                index + 1,
                path.display()
            )
        })?;
        if selector.trim().is_empty() {
            return Err(format!(
                "selection item {} in {} has an empty selector",
                index + 1,
                path.display()
            ));
        }
        items.push(SelectionItem {
            selector,
            kind: extract_json_string(&line, "kind"),
            action: extract_json_string(&line, "action"),
            format: extract_json_string(&line, "format"),
            time_seconds: extract_json_f64(&line, "time_seconds"),
            notes: extract_json_string(&line, "notes"),
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
    let case_id = extract_json_string(&text, "case_id");
    let marks_text = extract_json_value(&text, "marks")
        .ok_or_else(|| format!("marks file {} is missing marks", path.display()))?;
    let mut marks = Vec::new();
    for (index, line) in json_object_lines(&marks_text).into_iter().enumerate() {
        let id = extract_json_string(&line, "id").ok_or_else(|| {
            format!(
                "marks entry {} in {} is missing id",
                index + 1,
                path.display()
            )
        })?;
        let status = extract_json_string(&line, "status").ok_or_else(|| {
            format!(
                "marks entry {} in {} is missing status",
                index + 1,
                path.display()
            )
        })?;
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
            marked_unix: extract_json_u64(&line, "marked_unix"),
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

const BS_CHAR: char = char::from_u32(0x5C).expect("backslash");

fn json_object_lines(array_text: &str) -> Vec<String> {
    let mut records = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for ch in array_text.chars() {
        if depth == 0 {
            // Only a `{` starts the next object; separators like `,` and the
            // closing `]` of the wrapping array are ignored.
            if ch == '{' {
                depth += 1;
                current.push(ch);
            }
            continue;
        }
        if in_string {
            current.push(ch);
            if escaped {
                escaped = false;
            } else if ch == BS_CHAR {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
                current.push(ch);
            }
            '{' => {
                depth += 1;
                current.push(ch);
            }
            '}' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
                if depth == 0 && !current.trim().is_empty() {
                    records.push(current.trim().to_string());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    records
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

fn extract_json_value(line: &str, key: &str) -> Option<String> {
    let key = format!("\"{}\":", key);
    let start = line.find(&key)? + key.len();
    let value = line[start..].trim_start();
    let first = value.chars().next()?;
    if first == '"' {
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
    } else if first == '[' {
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
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(value[..offset + 1].to_string());
                    }
                }
                _ => {}
            }
        }
        None
    } else {
        None
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
}
