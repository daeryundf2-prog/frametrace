use crate::case_db;
use serde_json::Value;
use std::fs;
use std::path::Path;

pub(super) fn seed_report_defense_case(case_dir: &Path, report_body: &str) {
    fs::create_dir_all(case_dir.join("db")).unwrap();
    fs::create_dir_all(case_dir.join("reports")).unwrap();
    fs::write(case_dir.join("case.json"), "{}").unwrap();
    fs::write(case_dir.join("db/video_index.json"), "{}").unwrap();
    fs::write(case_dir.join("db/videos.jsonl"), "").unwrap();
    fs::write(case_dir.join("db/video_paths.tsv"), "id\tsource_path\n").unwrap();
    fs::write(case_dir.join("reports/case-report.html"), report_body).unwrap();
    let conn = case_db::open_case_db(case_dir).unwrap();
    case_db::init_schema(&conn).unwrap();
}

pub(super) fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

pub(super) fn assert_finding_status(report: &Value, key: &str, status: &str) {
    let findings = report["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|finding| finding["key"] == key && finding["status"] == status),
        "missing finding {key}={status} in {report:#}"
    );
}
