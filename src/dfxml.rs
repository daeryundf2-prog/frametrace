//! DFXML (Digital Forensics XML) export of the case video index.
//!
//! Emits one `<fileobject>` per indexed record with `filename`, `filesize`,
//! `hashdigest type="sha256"` (when the index carries a hash), and `mtime`
//! in ISO 8601. Values come straight from `db/videos.jsonl` — recorded index
//! claims, not re-verified measurements — so the document is labelled
//! `candidate-export` and says so in its metadata. Output is confined to the
//! case directory and a chained entry lands in
//! `evidence/logs/dfxml-export-log.jsonl`.

use crate::anomaly::{IndexedRow, read_indexed_rows};
use crate::audit;
use crate::util::{html_escape, json_escape, now_unix, write_text};
use std::path::{Path, PathBuf};

/// Candidate-grade label: index values are recorded claims, not verified.
pub const LABEL: &str = "candidate-export";

#[derive(Debug, Clone)]
pub struct DfxmlExportResult {
    pub output_path: PathBuf,
    pub generated_unix: u64,
    pub object_count: usize,
    pub hashed_count: usize,
}

/// Write the case index as DFXML inside the case and append a chained audit
/// entry. `output_path` must resolve inside `case_dir`
/// (`require_case_output_path`).
pub fn export_dfxml(case_dir: &Path, output_path: &Path) -> Result<DfxmlExportResult, String> {
    crate::tool_policy::require_case_output_path(case_dir, output_path, "DFXML export")?;
    let rows = read_indexed_rows(case_dir)?;
    let generated_unix = now_unix()?;
    let document = render_dfxml(&rows, generated_unix);
    write_text(output_path, &document).map_err(|err| {
        format!(
            "failed to write DFXML export {}: {err}",
            output_path.display()
        )
    })?;
    let hashed_count = rows.iter().filter(|row| row.sha256.is_some()).count();
    let line = format!(
        "{{\"schema_version\":1,\"event\":\"export-dfxml\",\"generated_unix\":{},\"label\":\"{}\",\"output_path\":\"{}\",\"object_count\":{},\"hashed_count\":{},\"detail\":\"fileobject entries reflect recorded index rows, not re-verified content\"}}",
        generated_unix,
        LABEL,
        json_escape(&output_path.to_string_lossy()),
        rows.len(),
        hashed_count,
    );
    audit::append_chained_jsonl(
        &case_dir.join("evidence/logs/dfxml-export-log.jsonl"),
        &line,
    )?;
    Ok(DfxmlExportResult {
        output_path: output_path.to_path_buf(),
        generated_unix,
        object_count: rows.len(),
        hashed_count,
    })
}

fn render_dfxml(rows: &[IndexedRow], generated_unix: u64) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<dfxml xmlns=\"http://www.forensicswiki.org/wiki/Category:Digital_Forensics_XML\" xmloutputversion=\"1.0\">\n");
    out.push_str("  <metadata>\n");
    out.push_str(
        "    <dc:type xmlns:dc=\"http://purl.org/dc/elements/1.1/\">case video index</dc:type>\n",
    );
    out.push_str(&format!("    <label>{LABEL}</label>\n"));
    out.push_str(&format!(
        "    <generated_unix>{generated_unix}</generated_unix>\n"
    ));
    out.push_str("    <note>fileobject values are recorded index claims, not re-verified measurements</note>\n");
    out.push_str("  </metadata>\n");
    out.push_str("  <creator>\n");
    out.push_str(&format!(
        "    <program>{}</program>\n",
        env!("CARGO_PKG_NAME")
    ));
    out.push_str(&format!(
        "    <version>{}</version>\n",
        env!("CARGO_PKG_VERSION")
    ));
    out.push_str("  </creator>\n");
    for row in rows {
        out.push_str("  <fileobject>\n");
        out.push_str(&format!(
            "    <filename>{}</filename>\n",
            html_escape(&row.source_path)
        ));
        if let Some(size) = row.size_bytes {
            out.push_str(&format!("    <filesize>{size}</filesize>\n"));
        }
        if let Some(mtime) = row.modified_unix {
            out.push_str(&format!("    <mtime>{}</mtime>\n", unix_to_iso8601(mtime)));
        }
        if let Some(sha256) = row.sha256.as_deref() {
            out.push_str(&format!(
                "    <hashdigest type=\"sha256\">{}</hashdigest>\n",
                html_escape(&sha256.to_ascii_lowercase())
            ));
        }
        if !row.id.is_empty() {
            // Non-standard element: keeps the case-internal record id
            // traceable back to videos.jsonl without claiming verification.
            out.push_str(&format!(
                "    <frametrace:id>{}</frametrace:id>\n",
                html_escape(&row.id)
            ));
        }
        out.push_str("  </fileobject>\n");
    }
    out.push_str("</dfxml>\n");
    out
}

/// Unix seconds → `YYYY-MM-DDTHH:MM:SSZ` (UTC). Inverse of
/// `timeline::days_from_civil`; valid for all post-epoch timestamps.
fn unix_to_iso8601(unix: u64) -> String {
    let days = (unix / 86_400) as i64;
    let secs_of_day = unix % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        secs_of_day / 3600,
        (secs_of_day % 3600) / 60,
        secs_of_day % 60,
    )
}

/// Civil date for a count of days since the unix epoch (Howard Hinnant's
/// algorithm, inverse direction of `timeline::days_from_civil`).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = (z - era * 146_097) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_prime + 2) / 5 + 1) as u32;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::read_to_string;
    use std::fs;

    fn temp_case(name: &str, videos_jsonl: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("frametrace-dfxml-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("db")).unwrap();
        fs::write(dir.join("case.json"), "{}").unwrap();
        fs::write(dir.join("db/videos.jsonl"), videos_jsonl).unwrap();
        dir
    }

    #[test]
    fn emits_well_formed_dfxml_document() {
        let case_dir = temp_case(
            "wellformed",
            concat!(
                "{\"id\":\"vid_1\",\"source_path\":\"/ev/a.mp4\",\"size_bytes\":10,\"modified_unix\":1609459200,\"sha256\":\"AA11\"}\n",
                "{\"id\":\"vid_2\",\"source_path\":\"/ev/b.mp4\",\"size_bytes\":20,\"modified_unix\":null,\"sha256\":null}\n",
            ),
        );
        let output = case_dir.join("reports/case-index.dfxml");
        let result = export_dfxml(&case_dir, &output).unwrap();
        assert_eq!(result.object_count, 2);
        assert_eq!(result.hashed_count, 1);

        let text = read_to_string(&output).unwrap();
        // Structural well-formedness without an XML dependency: balanced
        // fileobject elements, header/footer, and expected children.
        assert!(text.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(text.contains("<dfxml "));
        assert!(text.trim_end().ends_with("</dfxml>"));
        assert_eq!(text.matches("<fileobject>").count(), 2);
        assert_eq!(text.matches("</fileobject>").count(), 2);
        assert!(text.contains("<filename>/ev/a.mp4</filename>"));
        assert!(text.contains("<filesize>10</filesize>"));
        // 1609459200 == 2021-01-01T00:00:00Z (checked against the timeline
        // parser's known constant).
        assert!(text.contains("<mtime>2021-01-01T00:00:00Z</mtime>"));
        assert!(text.contains("<hashdigest type=\"sha256\">aa11</hashdigest>"));
        // vid_2 has no hash or mtime: those elements must be absent.
        let second = text.rsplit("<fileobject>").next().unwrap();
        assert!(!second.contains("<hashdigest"), "{second}");
        assert!(!second.contains("<mtime>"), "{second}");
        assert!(text.contains(LABEL));

        // The run is audit-logged and the chain verifies.
        let verification =
            audit::verify_chained_jsonl(&case_dir.join("evidence/logs/dfxml-export-log.jsonl"))
                .unwrap();
        assert_eq!(verification.entries, 1);
        let _ = fs::remove_dir_all(&case_dir);
    }

    #[test]
    fn xml_escapes_special_characters_in_filenames() {
        let case_dir = temp_case(
            "escapes",
            "{\"id\":\"vid_1\",\"source_path\":\"/ev/a&b<q>.mp4\",\"size_bytes\":1,\"modified_unix\":null,\"sha256\":null}\n",
        );
        let output = case_dir.join("reports/case-index.dfxml");
        export_dfxml(&case_dir, &output).unwrap();
        let text = read_to_string(&output).unwrap();
        assert!(text.contains("<filename>/ev/a&amp;b&lt;q&gt;.mp4</filename>"));
        assert!(!text.contains("a&b<q>"));
        let _ = fs::remove_dir_all(&case_dir);
    }

    #[test]
    fn empty_index_emits_a_document_with_no_fileobjects() {
        let case_dir = temp_case("empty", "");
        let output = case_dir.join("reports/case-index.dfxml");
        let result = export_dfxml(&case_dir, &output).unwrap();
        assert_eq!(result.object_count, 0);
        let text = read_to_string(&output).unwrap();
        assert!(!text.contains("<fileobject>"));
        assert!(text.trim_end().ends_with("</dfxml>"));
        let _ = fs::remove_dir_all(&case_dir);
    }

    #[test]
    fn output_must_stay_inside_the_case() {
        let case_dir = temp_case("confined", "");
        let outside = case_dir.parent().unwrap().join("dfxml-outside.xml");
        let err = export_dfxml(&case_dir, &outside).unwrap_err();
        assert!(err.contains("inside the case directory"), "{err}");
        assert!(!outside.exists());
        let _ = fs::remove_dir_all(&case_dir);
    }

    #[test]
    fn unix_to_iso8601_round_trips_through_timeline_parser() {
        // The timeline module's creation_time parser is the inverse; feeding
        // its own output back must reproduce the original timestamp.
        for ts in [0u64, 1_609_459_200, 1_700_000_000, 4_000_000_000] {
            let rendered = unix_to_iso8601(ts);
            assert_eq!(
                crate::timeline::parse_creation_time_unix(&rendered),
                Some(ts),
                "round-trip failed for {ts} -> {rendered}"
            );
        }
        assert_eq!(unix_to_iso8601(0), "1970-01-01T00:00:00Z");
    }
}
