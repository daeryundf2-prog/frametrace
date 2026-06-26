use crate::util::{json_escape, now_unix, write_text};
use rusqlite::{Connection, params};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub const FULL_PATH_DISCLOSURE_FILE: &str = "privacy-full-path-disclosure.json";

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PathDisclosureMode {
    Redacted,
    LocalOperatorFullPaths,
}

#[derive(Debug, Clone, Copy)]
pub struct RedactionPolicy {
    mode: PathDisclosureMode,
}

impl RedactionPolicy {
    pub const fn redacted() -> Self {
        Self {
            mode: PathDisclosureMode::Redacted,
        }
    }

    pub const fn local_operator_full_paths() -> Self {
        Self {
            mode: PathDisclosureMode::LocalOperatorFullPaths,
        }
    }

    pub const fn mode(self) -> PathDisclosureMode {
        self.mode
    }

    pub const fn is_redacted(self) -> bool {
        matches!(self.mode, PathDisclosureMode::Redacted)
    }

    pub const fn mode_label(self) -> &'static str {
        match self.mode {
            PathDisclosureMode::Redacted => "redacted",
            PathDisclosureMode::LocalOperatorFullPaths => "local_operator_full_paths",
        }
    }

    pub const fn notice(self) -> &'static str {
        match self.mode {
            PathDisclosureMode::Redacted => {
                "Distributable output redacts local workstation/source paths by default."
            }
            PathDisclosureMode::LocalOperatorFullPaths => {
                "LOCAL/OPERATOR MODE: full workstation/source paths are disclosed by explicit operator opt-in."
            }
        }
    }
}

impl Default for RedactionPolicy {
    fn default() -> Self {
        Self::redacted()
    }
}

pub fn redact_json_for_distributable(
    case_dir: &Path,
    raw_json: &str,
    policy: RedactionPolicy,
) -> Result<String, String> {
    let mut value = serde_json::from_str::<Value>(raw_json)
        .map_err(|err| format!("failed to parse JSON for path redaction: {err}"))?;
    apply_value_policy(case_dir, &mut value, policy);
    serde_json::to_string(&value)
        .map_err(|err| format!("failed to serialize redacted distributable JSON: {err}"))
}

pub fn redact_jsonl_for_distributable(
    case_dir: &Path,
    raw_jsonl: &str,
    policy: RedactionPolicy,
) -> Result<String, String> {
    if !policy.is_redacted() {
        return Ok(raw_jsonl.to_string());
    }

    let mut out = String::new();
    for line in raw_jsonl.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut value = serde_json::from_str::<Value>(trimmed)
            .map_err(|err| format!("failed to parse JSONL row for path redaction: {err}"))?;
        apply_value_policy(case_dir, &mut value, policy);
        out.push_str(
            &serde_json::to_string(&value)
                .map_err(|err| format!("failed to serialize redacted JSONL row: {err}"))?,
        );
        out.push('\n');
    }
    Ok(out)
}

pub fn redact_generated_html_for_distributable(
    case_dir: &Path,
    raw_html: &str,
    policy: RedactionPolicy,
) -> Result<String, String> {
    if !policy.is_redacted() {
        return Ok(raw_html.to_string());
    }

    let mut out = String::new();
    for line in raw_html.split_inclusive('\n') {
        out.push_str(&redact_generated_html_line(case_dir, line, policy)?);
    }
    if !raw_html.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    Ok(out)
}

pub fn redact_tsv_for_distributable(raw_tsv: &str, policy: RedactionPolicy) -> String {
    if !policy.is_redacted() {
        return raw_tsv.to_string();
    }

    let mut lines = raw_tsv.lines();
    let Some(header) = lines.next() else {
        return String::new();
    };
    let headers = header.split('\t').collect::<Vec<_>>();
    let path_columns = headers
        .iter()
        .enumerate()
        .filter_map(|(index, name)| is_path_key(name).then_some(index))
        .collect::<Vec<_>>();
    let id_column = headers
        .iter()
        .position(|name| *name == "id")
        .unwrap_or_default();

    let mut out = String::new();
    out.push_str(header);
    out.push('\n');
    for line in lines {
        let mut fields = line.split('\t').map(str::to_string).collect::<Vec<_>>();
        let label = fields
            .get(id_column)
            .filter(|value| !value.is_empty())
            .map(String::as_str)
            .unwrap_or("row")
            .to_string();
        for index in &path_columns {
            if let Some(value) = fields.get_mut(*index)
                && is_absolute_path_or_file_url(value)
            {
                *value = redacted_label(headers[*index], &label);
            }
        }
        out.push_str(&fields.join("\t"));
        out.push('\n');
    }
    out
}

pub fn privacy_metadata_fields(policy: RedactionPolicy) -> String {
    format!(
        "\"path_disclosure_mode\":\"{}\",\"local_operator_full_path_disclosure\":{},\"path_disclosure_notice\":\"{}\"",
        policy.mode_label(),
        if policy.mode() == PathDisclosureMode::LocalOperatorFullPaths {
            "true"
        } else {
            "false"
        },
        json_escape(policy.notice())
    )
}

pub fn write_full_path_disclosure_artifact(
    output_dir: &Path,
    surface: &str,
    policy: RedactionPolicy,
) -> Result<Option<PathBuf>, String> {
    if policy.mode() != PathDisclosureMode::LocalOperatorFullPaths {
        return Ok(None);
    }

    let created_unix = now_unix()?;
    let path = output_dir.join(FULL_PATH_DISCLOSURE_FILE);
    let body = format!(
        "{{\n  \"schema_version\": 1,\n  \"surface\": \"{}\",\n  \"created_unix\": {},\n  \"path_disclosure_mode\": \"{}\",\n  \"local_operator_full_path_disclosure\": true,\n  \"notice\": \"{}\"\n}}\n",
        json_escape(surface),
        created_unix,
        policy.mode_label(),
        json_escape(policy.notice())
    );
    write_text(&path, &body)
        .map_err(|err| format!("failed to write full path disclosure artifact: {err}"))?;
    Ok(Some(path))
}

pub fn redact_sqlite_copy_for_distributable(
    db_path: &Path,
    case_dir: &Path,
    policy: RedactionPolicy,
) -> Result<(), String> {
    if !policy.is_redacted() {
        return Ok(());
    }
    let Ok(conn) = Connection::open(db_path) else {
        return Ok(());
    };

    redact_sqlite_table_column(&conn, "videos", "id", "source_path", "source")?;
    redact_sqlite_table_column(&conn, "videos", "id", "file_url", "path")?;
    redact_sqlite_json_column(&conn, "videos", "id", "record_json", case_dir, policy)?;
    redact_sqlite_json_column(&conn, "videos", "id", "ffprobe_json", case_dir, policy)?;
    redact_sqlite_table_column(&conn, "scan_runs", "run_pk", "source_path", "source")?;
    if sqlite_table_exists(&conn, "evidence_sources")? {
        redact_sqlite_table_column(&conn, "evidence_sources", "source_id", "path", "source")?;
    }
    if sqlite_table_exists(&conn, "jobs")? {
        redact_sqlite_table_column(&conn, "jobs", "job_id", "subject_path", "path")?;
    }
    Ok(())
}

fn redact_generated_html_line(
    case_dir: &Path,
    line: &str,
    policy: RedactionPolicy,
) -> Result<String, String> {
    let Some(const_index) = line.find("const ") else {
        return Ok(line.to_string());
    };
    let Some(assign_relative) = line[const_index..].find(" = ") else {
        return Ok(line.to_string());
    };
    let payload_start = const_index + assign_relative + 3;
    let Some(payload_end_relative) = line[payload_start..].rfind(';') else {
        return Ok(line.to_string());
    };
    let payload_end = payload_start + payload_end_relative;
    let payload = line[payload_start..payload_end].trim();
    if !(payload.starts_with('{') || payload.starts_with('[')) {
        return Ok(line.to_string());
    }
    let leading_payload_ws = line[payload_start..payload_end]
        .chars()
        .take_while(|ch| ch.is_whitespace())
        .collect::<String>();
    let redacted_payload = match redact_json_for_distributable(case_dir, payload, policy) {
        Ok(redacted) => redacted,
        Err(_) => return Ok(line.to_string()),
    };
    Ok(format!(
        "{}{}{}{}",
        &line[..payload_start],
        leading_payload_ws,
        redacted_payload,
        &line[payload_end..]
    ))
}

fn apply_value_policy(case_dir: &Path, value: &mut Value, policy: RedactionPolicy) {
    match value {
        Value::Object(object) => apply_object_policy(case_dir, object, policy),
        Value::Array(items) => {
            for item in items {
                apply_value_policy(case_dir, item, policy);
            }
        }
        _ => {}
    }
}

fn apply_object_policy(case_dir: &Path, object: &mut Map<String, Value>, policy: RedactionPolicy) {
    let label = object_label(object);
    for (key, value) in object.iter_mut() {
        match value {
            Value::String(path) if policy.is_redacted() && is_path_key(key) => {
                *path = redact_path_value(case_dir, key, path, &label);
            }
            Value::Object(_) | Value::Array(_) => apply_value_policy(case_dir, value, policy),
            _ => {}
        }
    }
    object.insert(
        "path_disclosure_mode".to_string(),
        Value::String(policy.mode_label().to_string()),
    );
    object.insert(
        "local_operator_full_path_disclosure".to_string(),
        Value::Bool(policy.mode() == PathDisclosureMode::LocalOperatorFullPaths),
    );
    object.insert(
        "path_disclosure_notice".to_string(),
        Value::String(policy.notice().to_string()),
    );
}

fn object_label(object: &Map<String, Value>) -> String {
    for key in [
        "source_id",
        "source_artifact_id",
        "derived_artifact_id",
        "target_artifact_id",
        "id",
        "selector",
        "inode",
        "job_id",
    ] {
        if let Some(Value::String(value)) = object.get(key)
            && !value.is_empty()
        {
            return value.clone();
        }
    }
    "artifact".to_string()
}

fn redact_path_value(case_dir: &Path, key: &str, value: &str, label: &str) -> String {
    if key == "file_url" {
        return String::new();
    }
    if let Some(relative) = case_relative_path(case_dir, value) {
        return relative;
    }
    if is_absolute_path_or_file_url(value) {
        return redacted_label(key, label);
    }
    value.to_string()
}

fn case_relative_path(case_dir: &Path, value: &str) -> Option<String> {
    let path_text = value.strip_prefix("file://").unwrap_or(value);
    let path = Path::new(path_text);
    path.strip_prefix(case_dir)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
}

fn is_path_key(key: &str) -> bool {
    matches!(
        key,
        "source_path"
            | "file_url"
            | "output_path"
            | "output_artifact_path"
            | "target_path"
            | "source_artifact_path"
            | "image_path"
            | "summary_path"
            | "entries_jsonl_path"
            | "mmls_log_path"
            | "fls_log_path"
            | "filename"
            | "path"
            | "subject_path"
    )
}

fn is_absolute_path_or_file_url(value: &str) -> bool {
    let normalized = value.replace('\\', "/");
    normalized.starts_with("file://")
        || normalized.starts_with('/')
        || normalized.starts_with("//")
        || normalized
            .as_bytes()
            .get(1)
            .is_some_and(|byte| *byte == b':')
}

fn redacted_label(key: &str, label: &str) -> String {
    if key == "source_path" || key == "source_artifact_path" || key == "image_path" {
        format!("[redacted-source:{label}]")
    } else {
        format!("[redacted-path:{label}]")
    }
}

fn sqlite_table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
    match conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1 LIMIT 1",
        params![table],
        |_| Ok(()),
    ) {
        Ok(()) => Ok(true),
        Err(rusqlite::Error::QueryReturnedNoRows) | Err(rusqlite::Error::SqliteFailure(_, _)) => {
            Ok(false)
        }
        Err(err) => Err(format!("failed to inspect SQLite table {table}: {err}")),
    }
}

fn redact_sqlite_table_column(
    conn: &Connection,
    table: &str,
    id_column: &str,
    path_column: &str,
    label_kind: &str,
) -> Result<(), String> {
    if !sqlite_table_exists(conn, table)? || !sqlite_column_exists(conn, table, path_column)? {
        return Ok(());
    }
    let select_sql = format!("SELECT CAST({id_column} AS TEXT), {path_column} FROM {table}");
    let mut statement = match conn.prepare(&select_sql) {
        Ok(statement) => statement,
        Err(rusqlite::Error::SqliteFailure(_, _)) => return Ok(()),
        Err(err) => return Err(format!("failed to prepare SQLite redaction query: {err}")),
    };
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|err| format!("failed to query SQLite paths for redaction: {err}"))?;
    let mut updates = Vec::new();
    for row in rows {
        let (id, path) =
            row.map_err(|err| format!("failed to read SQLite redaction row: {err}"))?;
        if is_absolute_path_or_file_url(&path) {
            let redacted = if path_column == "file_url" {
                String::new()
            } else {
                format!("[redacted-{label_kind}:{id}]")
            };
            updates.push((id.clone(), redacted));
        }
    }
    let update_sql = format!("UPDATE {table} SET {path_column} = ?1 WHERE {id_column} = ?2");
    for (id, redacted) in updates {
        conn.execute(&update_sql, params![redacted, id])
            .map_err(|err| format!("failed to update SQLite redacted path: {err}"))?;
    }
    Ok(())
}

fn redact_sqlite_json_column(
    conn: &Connection,
    table: &str,
    id_column: &str,
    json_column: &str,
    case_dir: &Path,
    policy: RedactionPolicy,
) -> Result<(), String> {
    if !sqlite_table_exists(conn, table)? || !sqlite_column_exists(conn, table, json_column)? {
        return Ok(());
    }
    let select_sql = format!("SELECT CAST({id_column} AS TEXT), {json_column} FROM {table}");
    let mut statement = match conn.prepare(&select_sql) {
        Ok(statement) => statement,
        Err(rusqlite::Error::SqliteFailure(_, _)) => return Ok(()),
        Err(err) => {
            return Err(format!(
                "failed to prepare SQLite JSON redaction query: {err}"
            ));
        }
    };
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(|err| format!("failed to query SQLite JSON for redaction: {err}"))?;
    let mut updates = Vec::new();
    for row in rows {
        let (id, Some(raw_json)) =
            row.map_err(|err| format!("failed to read SQLite JSON redaction row: {err}"))?
        else {
            continue;
        };
        let trimmed = raw_json.trim();
        if trimmed.is_empty() || trimmed == "null" {
            continue;
        }
        let redacted = redact_json_for_distributable(case_dir, trimmed, policy)
            .map_err(|err| format!("failed to redact SQLite {table}.{json_column}: {err}"))?;
        updates.push((id, redacted));
    }
    let update_sql = format!("UPDATE {table} SET {json_column} = ?1 WHERE {id_column} = ?2");
    for (id, redacted) in updates {
        conn.execute(&update_sql, params![redacted, id])
            .map_err(|err| format!("failed to update SQLite redacted JSON: {err}"))?;
    }
    Ok(())
}

fn sqlite_column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    let pragma_sql = format!("PRAGMA table_info({table})");
    let mut statement = conn
        .prepare(&pragma_sql)
        .map_err(|err| format!("failed to inspect SQLite columns for {table}: {err}"))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|err| format!("failed to query SQLite columns for {table}: {err}"))?;
    for row in rows {
        if row.map_err(|err| format!("failed to read SQLite column info: {err}"))? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::{
        RedactionPolicy, redact_generated_html_for_distributable, redact_json_for_distributable,
        redact_tsv_for_distributable,
    };

    #[test]
    fn redacts_absolute_json_paths_but_keeps_case_relative_artifacts() {
        let case_dir = std::path::Path::new("/tmp/Case Root");
        let raw = r#"{"id":"vid_1","source_path":"/tmp/Client/source.mp4","file_url":"file:///tmp/Client/source.mp4","output_path":"/tmp/Case Root/artifacts/frames/frame.jpg"}"#;

        let redacted =
            redact_json_for_distributable(case_dir, raw, RedactionPolicy::redacted()).unwrap();

        assert!(!redacted.contains("/tmp/Client"));
        assert!(!redacted.contains("file:///"));
        assert!(redacted.contains("[redacted-source:vid_1]"));
        assert!(redacted.contains("artifacts/frames/frame.jpg"));
        assert!(redacted.contains("\"path_disclosure_mode\":\"redacted\""));
    }

    #[test]
    fn local_operator_mode_keeps_full_json_paths_and_marks_metadata() {
        let raw = r#"{"id":"vid_1","source_path":"/tmp/Client/source.mp4"}"#;

        let disclosed = redact_json_for_distributable(
            std::path::Path::new("/tmp/Case"),
            raw,
            RedactionPolicy::local_operator_full_paths(),
        )
        .unwrap();

        assert!(disclosed.contains("/tmp/Client/source.mp4"));
        assert!(disclosed.contains("\"path_disclosure_mode\":\"local_operator_full_paths\""));
        assert!(disclosed.contains("\"local_operator_full_path_disclosure\":true"));
    }

    #[test]
    fn redacts_tsv_path_columns() {
        let redacted = redact_tsv_for_distributable(
            "id\tsource_path\nvid_1\t/tmp/Client/source.mp4\n",
            RedactionPolicy::redacted(),
        );

        assert!(!redacted.contains("/tmp/Client"));
        assert!(redacted.contains("[redacted-source:vid_1]"));
    }

    #[test]
    fn redacts_generated_html_json_payloads() {
        let raw = r#"<script>
const scan = {"id":"vid_1","source_path":"/tmp/Client/source.mp4","file_url":"file:///tmp/Client/source.mp4"};
</script>"#;

        let redacted = redact_generated_html_for_distributable(
            std::path::Path::new("/tmp/Case"),
            raw,
            RedactionPolicy::redacted(),
        )
        .unwrap();

        assert!(!redacted.contains("/tmp/Client"));
        assert!(!redacted.contains("file:///"));
        assert!(redacted.contains("[redacted-source:vid_1]"));
    }

    #[test]
    fn keeps_generated_html_javascript_const_payloads_that_are_not_json() {
        let raw = r#"<script>
const bootstrap = {"scan": scan, "logs": frameLog};
</script>"#;

        let redacted = redact_generated_html_for_distributable(
            std::path::Path::new("/tmp/Case"),
            raw,
            RedactionPolicy::redacted(),
        )
        .unwrap();

        assert_eq!(redacted, raw);
    }
}
