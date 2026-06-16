use std::path::Path;

const VOLATILE_NUMERIC_FIELDS: &[&str] = &[
    "created_unix",
    "scanned_unix",
    "first_indexed_unix",
    "last_indexed_unix",
    "last_scanned_unix",
    "registered_unix",
    "last_seen_unix",
    "started_unix",
    "updated_unix",
    "completed_unix",
    "event_unix",
    "carved_unix",
    "inspected_unix",
    "recovered_unix",
    "validated_unix",
];

const VOLATILE_STRING_FIELDS: &[&str] = &["entry_sha256", "previous_entry_sha256"];

pub(crate) fn normalize_reproducibility_text(case_dir: &Path, text: &str) -> String {
    let mut normalized = normalize_case_paths(case_dir, text);
    for key in VOLATILE_NUMERIC_FIELDS {
        normalized = replace_json_number_field(&normalized, key, "0");
    }
    for key in VOLATILE_STRING_FIELDS {
        normalized = replace_json_string_field(&normalized, key, "<VOLATILE>");
    }
    normalized
}

fn normalize_case_paths(case_dir: &Path, text: &str) -> String {
    let mut out = text.replace('\\', "/");
    let raw = case_dir.to_string_lossy().replace('\\', "/");
    out = out.replace(&raw, "<CASE>");
    if let Ok(canonical) = case_dir.canonicalize() {
        let canonical = canonical.to_string_lossy().replace('\\', "/");
        out = out.replace(&canonical, "<CASE>");
    }
    out
}

fn replace_json_number_field(text: &str, key: &str, replacement: &str) -> String {
    let marker = format!("\"{key}\":");
    let mut out = String::with_capacity(text.len());
    let mut remaining = text;
    while let Some(index) = remaining.find(&marker) {
        out.push_str(&remaining[..index + marker.len()]);
        let after_marker = &remaining[index + marker.len()..];
        let whitespace_len = after_marker
            .char_indices()
            .find(|(_, ch)| !ch.is_whitespace())
            .map(|(idx, _)| idx)
            .unwrap_or(after_marker.len());
        out.push_str(&after_marker[..whitespace_len]);
        let after_whitespace = &after_marker[whitespace_len..];
        let number_len = after_whitespace
            .char_indices()
            .take_while(|(_, ch)| ch.is_ascii_digit())
            .last()
            .map(|(idx, ch)| idx + ch.len_utf8())
            .unwrap_or(0);
        if number_len == 0 {
            remaining = after_marker;
            continue;
        }
        out.push_str(replacement);
        remaining = &after_whitespace[number_len..];
    }
    out.push_str(remaining);
    out
}

fn replace_json_string_field(text: &str, key: &str, replacement: &str) -> String {
    let marker = format!("\"{key}\":");
    let mut out = String::with_capacity(text.len());
    let mut remaining = text;
    while let Some(index) = remaining.find(&marker) {
        out.push_str(&remaining[..index + marker.len()]);
        let after_marker = &remaining[index + marker.len()..];
        let whitespace_len = after_marker
            .char_indices()
            .find(|(_, ch)| !ch.is_whitespace())
            .map(|(idx, _)| idx)
            .unwrap_or(after_marker.len());
        out.push_str(&after_marker[..whitespace_len]);
        let after_whitespace = &after_marker[whitespace_len..];
        let Some(rest) = after_whitespace.strip_prefix('"') else {
            remaining = after_marker;
            continue;
        };
        let Some(string_len) = json_string_content_len(rest) else {
            remaining = after_marker;
            continue;
        };
        out.push('"');
        out.push_str(replacement);
        out.push('"');
        remaining = &rest[string_len + 1..];
    }
    out.push_str(remaining);
    out
}

fn json_string_content_len(value: &str) -> Option<usize> {
    let mut escaping = false;
    for (index, ch) in value.char_indices() {
        if escaping {
            escaping = false;
            continue;
        }
        if ch == '\\' {
            escaping = true;
            continue;
        }
        if ch == '"' {
            return Some(index);
        }
    }
    None
}
