use crate::audit;
use crate::util::{json_escape, now_unix, write_text};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PackageResult {
    pub output_dir: PathBuf,
    pub file_count: usize,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Clone)]
struct PackageFile {
    relative_path: PathBuf,
    sha256: String,
    size_bytes: u64,
}

pub fn package_case(case_dir: &Path, output_dir: Option<&Path>) -> Result<PackageResult, String> {
    let created_unix = now_unix()?;
    let running_jobs = match crate::case_db::latest_running_jobs(case_dir) {
        Ok(jobs) => jobs,
        Err(err) => {
            return Err(format!(
                "package-case cannot determine case quiescence; refusing to claim an unverified snapshot: {err}"
            ));
        }
    };
    let quiescent = running_jobs.is_empty();
    let output_dir = match output_dir {
        Some(path) => {
            reject_recursive_package_output(case_dir, path)?;
            if path.exists()
                && path
                    .read_dir()
                    .map_err(|err| format!("failed to inspect package output: {err}"))?
                    .next()
                    .is_some()
            {
                return Err(format!(
                    "package output already exists and is not empty: {}",
                    path.display()
                ));
            }
            path.to_path_buf()
        }
        None => crate::util::unique_dir(
            &case_dir
                .join("reports")
                .join(format!("package_{created_unix}")),
        ),
    };
    validate_required_package_files(case_dir)?;
    // Preflight: the package duplicates every report/artifact under the
    // case dir, so ensure the target volume can hold a second copy before
    // the copy loop starts.
    crate::diskspace::ensure_available(
        &output_dir,
        estimate_package_bytes(case_dir, &output_dir),
        "package-case",
    )?;
    fs::create_dir_all(&output_dir)
        .map_err(|err| format!("failed to create package output: {err}"))?;

    let mut files = Vec::new();
    let mut missing_optional_files = Vec::new();
    for rel in required_package_files() {
        copy_package_file(case_dir, &output_dir, Path::new(rel), &mut files)?;
    }
    for rel in optional_package_files() {
        copy_optional_package_file(
            case_dir,
            &output_dir,
            Path::new(rel),
            &mut files,
            &mut missing_optional_files,
        )?;
    }
    for rel_dir in recursive_package_dirs() {
        copy_package_dir(case_dir, &output_dir, Path::new(rel_dir), &mut files)?;
    }
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    let checksum_text = files
        .iter()
        .map(|file| {
            format!(
                "{}  {}\n",
                file.sha256,
                rel_manifest_path(&file.relative_path)
            )
        })
        .collect::<String>();
    let checksum_path = output_dir.join("manifest.sha256");
    write_text(&checksum_path, &checksum_text)
        .map_err(|err| format!("failed to write package checksum manifest: {err}"))?;

    let manifest_json = package_manifest_json(
        created_unix,
        &files,
        &missing_optional_files,
        &running_jobs,
        quiescent,
    );
    let manifest_path = output_dir.join("package-manifest.json");
    write_text(&manifest_path, &manifest_json)
        .map_err(|err| format!("failed to write package manifest: {err}"))?;

    let report_included = files
        .iter()
        .any(|file| file.relative_path == Path::new("reports/case-report.html"));
    let report_note = if report_included {
        "Open reports/case-report.html for the HTML report. Use the browser print dialog to create a PDF when required by the engagement."
    } else {
        "No HTML report is included — run `frametrace make-report <case-dir>` in the case before packaging to produce reports/case-report.html."
    };
    write_text(
        &output_dir.join("README.txt"),
        &format!(
            "FrameTrace case package\n\n{report_note} Verify package contents with manifest.sha256 before transfer.\n\nSnapshot scope: see snapshot_scope in package-manifest.json. For Amped FIVE or Magnet DVR Examiner handoff steps, see docs/COMMERCIAL_HANDOFF.md in the FrameTrace repository.\n"
        ),
    )
    .map_err(|err| format!("failed to write package README: {err}"))?;

    Ok(PackageResult {
        output_dir,
        file_count: files.len(),
        manifest_path,
    })
}

fn estimate_package_bytes(case_dir: &Path, output_dir: &Path) -> u64 {
    fn dir_bytes(path: &Path, rel: &Path, output_dir: &Path, total: &mut u64) {
        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            let entry_path = entry.path();
            let entry_rel = rel.join(entry.file_name());
            if excluded_package_path(&entry_path, output_dir) {
                continue;
            }
            let Ok(metadata) = fs::symlink_metadata(&entry_path) else {
                continue;
            };
            if metadata.is_file() && is_report_file(rel) {
                *total = total.saturating_add(metadata.len());
            } else if metadata.is_dir() {
                dir_bytes(&entry_path, &entry_rel, output_dir, total);
            }
        }
    }

    let mut total = 0u64;
    for rel in required_package_files()
        .iter()
        .chain(optional_package_files().iter())
    {
        total = total.saturating_add(
            fs::metadata(case_dir.join(rel))
                .map(|meta| meta.len())
                .unwrap_or(0),
        );
    }
    dir_bytes(
        &case_dir.join("reports"),
        Path::new("reports"),
        output_dir,
        &mut total,
    );
    dir_bytes(
        &case_dir.join("qa"),
        Path::new("qa"),
        output_dir,
        &mut total,
    );
    for rel_dir in recursive_package_dirs() {
        dir_bytes(
            &case_dir.join(rel_dir),
            Path::new(rel_dir),
            output_dir,
            &mut total,
        );
    }
    total
}

fn reject_recursive_package_output(case_dir: &Path, output_dir: &Path) -> Result<(), String> {
    for rel_dir in recursive_package_dirs() {
        let packaged_tree = case_dir.join(rel_dir);
        if output_dir.starts_with(&packaged_tree) {
            return Err(format!(
                "package output cannot be inside recursively packaged directory: {}",
                packaged_tree.display()
            ));
        }
    }
    Ok(())
}

fn required_package_files() -> &'static [&'static str] {
    &[
        "case.json",
        "db/case.db",
        "db/video_index.json",
        "db/videos.jsonl",
        "db/video_paths.tsv",
    ]
}

fn optional_package_files() -> &'static [&'static str] {
    &[
        "db/carve_results.json",
        "review/index.html",
        "review/evidence-viewer.html",
        "review/thumbs.json",
        "artifacts/logs/batch-log.jsonl",
        "db/timeline.jsonl",
    ]
}

fn validate_required_package_files(case_dir: &Path) -> Result<(), String> {
    let missing = required_package_files()
        .iter()
        .filter(|rel| !case_dir.join(rel).is_file())
        .copied()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "case package is missing required files: {}",
            missing.join(", ")
        ))
    }
}

fn recursive_package_dirs() -> &'static [&'static str] {
    &[
        "evidence/logs",
        "artifacts/clips",
        "artifacts/proxies",
        "artifacts/thumbnails",
        "artifacts/carved",
        "artifacts/recovered",
        "db/filesystem",
        "db/scan_runs",
        "review/thumbs",
        "qa",
        "reports",
    ]
}

fn excluded_package_path(path: &Path, output_dir: &Path) -> bool {
    if path == output_dir || path.join("package-manifest.json").is_file() {
        return true;
    }
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    name.starts_with('.')
        || name.starts_with("package_")
        || matches!(
            name.as_str(),
            "audit-key" | "audit-keys.json" | "secrets" | "credentials" | "credentials.json"
        )
        || matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("key" | "pem" | "p12" | "pfx")
        )
}

// Manifest paths must be portable: a package written on Windows and
// verified on Linux has to carry identical relative paths, so always
// serialize with '/' separators rather than the host Path separator.
fn rel_manifest_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn is_report_file(rel: &Path) -> bool {
    if rel.starts_with("reports") || rel.starts_with("qa") {
        matches!(
            rel.extension().and_then(|ext| ext.to_str()),
            Some(
                "md" | "html" | "json" | "jsonl" | "csv" | "tsv" | "xml" | "txt" | "pdf" | "dfxml"
            )
        )
    } else {
        true
    }
}

fn copy_package_dir(
    case_dir: &Path,
    output_dir: &Path,
    rel_dir: &Path,
    files: &mut Vec<PackageFile>,
) -> Result<(), String> {
    let source_dir = case_dir.join(rel_dir);
    if !source_dir.is_dir() {
        return Ok(());
    }
    let entries = fs::read_dir(&source_dir).map_err(|err| {
        format!(
            "failed to read package directory {}: {err}",
            source_dir.display()
        )
    })?;
    for entry in entries {
        let entry =
            entry.map_err(|err| format!("failed to read package directory entry: {err}"))?;
        let path = entry.path();
        let rel = rel_dir.join(entry.file_name());
        if excluded_package_path(&path, output_dir) {
            continue;
        }
        let file_type = entry
            .file_type()
            .map_err(|err| format!("failed to read package file type {}: {err}", path.display()))?;
        if file_type.is_symlink() {
            return Err(format!(
                "package input contains unsupported symlink: {}",
                path.display()
            ));
        }
        if file_type.is_dir() {
            copy_package_dir(case_dir, output_dir, &rel, files)?;
        } else if file_type.is_file() && is_report_file(&rel) {
            copy_package_file(case_dir, output_dir, &rel, files)?;
        }
    }
    Ok(())
}

fn copy_package_file(
    case_dir: &Path,
    output_dir: &Path,
    rel: &Path,
    files: &mut Vec<PackageFile>,
) -> Result<(), String> {
    let source = case_dir.join(rel);
    let metadata = match fs::symlink_metadata(&source) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(format!(
                "failed to inspect package source {}: {err}",
                source.display()
            ));
        }
    };
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "package source cannot be a symlink: {}",
            source.display()
        ));
    }
    if !metadata.is_file() {
        return Ok(());
    }
    let target = output_dir.join(rel);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create package directory: {err}"))?;
    }
    if rel == Path::new("db/case.db") {
        let conn = crate::case_db::open_readonly_case_db(&source)?;
        conn.backup(rusqlite::MAIN_DB, &target, None)
            .map_err(|err| format!("failed to snapshot package SQLite database: {err}"))?;
    } else {
        fs::copy(&source, &target).map_err(|err| {
            format!(
                "failed to copy package file {} to {}: {err}",
                source.display(),
                target.display()
            )
        })?;
    }
    files.push(PackageFile {
        relative_path: rel.to_path_buf(),
        sha256: audit::digest_file(&target)?,
        size_bytes: fs::metadata(&target)
            .map_err(|err| format!("failed to stat final package file: {err}"))?
            .len(),
    });
    Ok(())
}

fn copy_optional_package_file(
    case_dir: &Path,
    output_dir: &Path,
    rel: &Path,
    files: &mut Vec<PackageFile>,
    missing_optional_files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if !case_dir.join(rel).is_file() {
        missing_optional_files.push(rel.to_path_buf());
        return Ok(());
    }
    copy_package_file(case_dir, output_dir, rel, files)
}

fn package_manifest_json(
    created_unix: u64,
    files: &[PackageFile],
    missing_optional_files: &[PathBuf],
    running_jobs: &[crate::case_db::RunningJob],
    quiescent: bool,
) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"schema_version\": 1,\n");
    out.push_str("  \"package_type\": \"frametrace-case-package\",\n");
    out.push_str(&format!("  \"created_unix\": {},\n", created_unix));
    out.push_str(&format!("  \"file_count\": {},\n", files.len()));
    out.push_str("  \"files\": [\n");
    for (index, file) in files.iter().enumerate() {
        out.push_str(&format!(
            "    {{\"relative_path\":\"{}\",\"size_bytes\":{},\"sha256\":\"{}\"}}",
            json_escape(&rel_manifest_path(&file.relative_path)),
            file.size_bytes,
            json_escape(&file.sha256)
        ));
        if index + 1 != files.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ],\n");
    out.push_str("  \"missing_optional_files\": [\n");
    for (index, rel) in missing_optional_files.iter().enumerate() {
        out.push_str(&format!("    \"{}\"", json_escape(&rel.to_string_lossy())));
        if index + 1 != missing_optional_files.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ],\n");
    out.push_str("  \"quiescence\": {\n");
    out.push_str(&format!("    \"quiescent\": {},\n", quiescent));
    out.push_str("    \"running_jobs\": [\n");
    for (index, job) in running_jobs.iter().enumerate() {
        out.push_str(&format!(
            "    {{\"job_id\":\"{}\",\"job_type\":\"{}\",\"subject_path\":\"{}\",\"started_unix\":{}}}",
            json_escape(&job.job_id),
            json_escape(&job.job_type),
            json_escape(&job.subject_path),
            job.started_unix
        ));
        if index + 1 != running_jobs.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("    ]\n");
    out.push_str("  },\n");
    let snapshot_scope = if quiescent {
        "Packaged while the case showed no running jobs; the multi-file set is a best-effort near-point-in-time snapshot, not a single atomic point-in-time snapshot. Files captured at different moments during packaging; re-run tools that modify the case after packaging invalidates it."
    } else {
        "Jobs were running when this package was created, so the multi-file set is explicitly NOT a point-in-time snapshot and not a single atomic point-in-time snapshot; files captured mid-write may be internally inconsistent. Re-package after jobs complete."
    };
    out.push_str(&format!(
        "  \"snapshot_scope\": \"{}\",\n",
        json_escape(snapshot_scope)
    ));
    let pdf_note = if files
        .iter()
        .any(|file| file.relative_path == Path::new("reports/case-report.html"))
    {
        "Open reports/case-report.html and print to PDF after examiner review."
    } else {
        "No HTML report is included; run `frametrace make-report <case-dir>` before packaging to produce one."
    };
    out.push_str(&format!("  \"pdf_ready_note\": \"{pdf_note}\"\n"));
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::package_case;
    use std::fs;

    fn fixture(name: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("frametrace-package-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let case_dir = root.join("case");
        fs::create_dir_all(case_dir.join("db")).unwrap();
        fs::write(case_dir.join("case.json"), b"{}").unwrap();
        fs::write(case_dir.join("db/video_index.json"), b"{}").unwrap();
        fs::write(case_dir.join("db/videos.jsonl"), b"").unwrap();
        fs::write(case_dir.join("db/video_paths.tsv"), b"id\tsource_path\n").unwrap();
        let conn = crate::case_db::open_case_db(&case_dir).unwrap();
        crate::case_db::init_schema(&conn).unwrap();
        root
    }

    #[test]
    fn wal_snapshot_preserves_committed_rows_and_final_file_metadata() {
        let root = fixture("wal");
        let case_dir = root.join("case");
        let output = root.join("package");
        let conn = rusqlite::Connection::open(case_dir.join("db/case.db")).unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE snapshot_rows (value TEXT); INSERT INTO snapshot_rows VALUES ('committed');").unwrap();
        assert!(case_dir.join("db/case.db-wal").metadata().unwrap().len() > 0);
        package_case(&case_dir, Some(&output)).unwrap();
        let backup = rusqlite::Connection::open(output.join("db/case.db")).unwrap();
        assert_eq!(
            backup
                .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        let rows = backup
            .prepare("SELECT value FROM snapshot_rows")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(rows, vec!["committed"]);
        drop(backup);
        drop(conn);
        let manifest: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(output.join("package-manifest.json")).unwrap(),
        )
        .unwrap();
        for file in manifest["files"].as_array().unwrap() {
            let path = output.join(file["relative_path"].as_str().unwrap());
            assert_eq!(
                file["size_bytes"].as_u64().unwrap(),
                path.metadata().unwrap().len()
            );
            assert_eq!(
                file["sha256"].as_str().unwrap(),
                crate::audit::digest_file(&path).unwrap()
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn package_includes_work_products_without_previous_packages_or_private_inputs() {
        let root = fixture("products");
        let case_dir = root.join("case");
        let products = [
            "artifacts/logs/batch-log.jsonl",
            "db/scan_runs/run_1.json",
            "db/timeline.jsonl",
            "review/thumbs/vid_1.jpg",
            "review/thumbs.json",
            "reports/qa/consistency-report.json",
            "reports/qa/report-defense-checklist.md",
            "qa/accuracy-report.html",
            "reports/comparison.json",
            "reports/timeline.csv",
            "reports/dfxml.xml",
            "reports/nested/export.tsv",
        ];
        let excluded = [
            "evidence/raw/original.raw",
            "evidence/originals/source.mp4",
            "db/job-locks/job.lock",
            "reports/.env",
            "reports/private.key",
            "reports/audit-key",
            "reports/audit-keys.json",
            "reports/secrets/credentials.json",
            "reports/previous/package-manifest.json",
            "reports/previous/db/case.db",
        ];
        for rel in products.iter().chain(excluded.iter()) {
            let path = case_dir.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, rel).unwrap();
        }
        let first = package_case(&case_dir, None).unwrap();
        let second = package_case(&case_dir, None).unwrap();
        for output in [&first.output_dir, &second.output_dir] {
            let manifest: serde_json::Value = serde_json::from_str(
                &fs::read_to_string(output.join("package-manifest.json")).unwrap(),
            )
            .unwrap();
            let listed = manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .map(|file| file["relative_path"].as_str().unwrap())
                .collect::<Vec<_>>();
            for rel in products {
                assert!(output.join(rel).is_file(), "missing {rel}");
                assert!(listed.contains(&rel), "not manifested: {rel}");
            }
            for rel in excluded {
                assert!(!output.join(rel).exists(), "included {rel}");
            }
            assert!(!listed.iter().any(|rel| rel.contains("package_")));
            assert_eq!(
                listed.len(),
                listed
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn manifest_records_running_jobs_and_snapshot_scope_limits() {
        let root = fixture("quiescence");
        let case_dir = root.join("case");
        let output = root.join("package");
        let job =
            crate::case_db::start_job(&case_dir, "scan-folder", &case_dir, None, "{}").unwrap();
        package_case(&case_dir, Some(&output)).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(output.join("package-manifest.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            manifest["quiescence"]["quiescent"],
            serde_json::json!(false)
        );
        assert!(
            manifest["quiescence"]["running_jobs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["job_type"] == "scan-folder")
        );
        assert!(
            manifest["snapshot_scope"]
                .as_str()
                .unwrap()
                .contains("not a single atomic point-in-time snapshot")
        );
        crate::case_db::complete_job(&case_dir, &job.job_id, 0, "done").unwrap();
        let output2 = root.join("package2");
        package_case(&case_dir, Some(&output2)).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(output2.join("package-manifest.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["quiescence"]["quiescent"], serde_json::json!(true));
        assert_eq!(
            manifest["quiescence"]["running_jobs"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn creates_checksummed_package_directory() {
        let root =
            std::env::temp_dir().join(format!("frametrace-package-test-{}", std::process::id()));
        let case_dir = root.join("case");
        let output_dir = root.join("package");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(case_dir.join("db")).unwrap();
        fs::create_dir_all(case_dir.join("reports")).unwrap();
        fs::write(case_dir.join("case.json"), b"{}").unwrap();
        let conn = crate::case_db::open_case_db(&case_dir).unwrap();
        crate::case_db::init_schema(&conn).unwrap();
        drop(conn);
        fs::write(case_dir.join("db/video_index.json"), b"{}").unwrap();
        fs::write(case_dir.join("db/videos.jsonl"), b"").unwrap();
        fs::write(case_dir.join("db/video_paths.tsv"), b"id\tsource_path\n").unwrap();
        fs::write(case_dir.join("reports/case-report.html"), b"<html></html>").unwrap();
        fs::write(case_dir.join("reports/summary.md"), b"# Summary").unwrap();

        let result = package_case(&case_dir, Some(&output_dir)).unwrap();
        assert!(output_dir.join("case.json").is_file());
        assert!(output_dir.join("db/case.db").is_file());
        assert!(output_dir.join("reports/summary.md").is_file());
        assert!(output_dir.join("manifest.sha256").is_file());
        assert!(result.manifest_path.is_file());
        let manifest = fs::read_to_string(&result.manifest_path).unwrap();
        assert!(manifest.contains("\"missing_optional_files\""));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_missing_required_package_files() {
        let root = std::env::temp_dir().join(format!(
            "frametrace-package-required-test-{}",
            std::process::id()
        ));
        let case_dir = root.join("case");
        let output_dir = root.join("package");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&case_dir).unwrap();
        fs::write(case_dir.join("case.json"), b"{}").unwrap();

        let err = package_case(&case_dir, Some(&output_dir)).unwrap_err();
        assert!(err.contains("missing required files"));
        assert!(!output_dir.exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_output_inside_recursively_packaged_tree() {
        let root = std::env::temp_dir().join(format!(
            "frametrace-package-recursive-test-{}",
            std::process::id()
        ));
        let case_dir = root.join("case");
        let output_dir = case_dir.join("evidence/logs/package");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(case_dir.join("evidence/logs")).unwrap();

        let err = package_case(&case_dir, Some(&output_dir)).unwrap_err();
        assert!(err.contains("recursively packaged directory"));

        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_package_inputs() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "frametrace-package-symlink-test-{}",
            std::process::id()
        ));
        let case_dir = root.join("case");
        let output_dir = root.join("package");
        let outside = root.join("outside.txt");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(case_dir.join("db")).unwrap();
        fs::create_dir_all(case_dir.join("evidence/logs")).unwrap();
        fs::write(case_dir.join("case.json"), b"{}").unwrap();
        let conn = crate::case_db::open_case_db(&case_dir).unwrap();
        crate::case_db::init_schema(&conn).unwrap();
        drop(conn);
        fs::write(case_dir.join("db/video_index.json"), b"{}").unwrap();
        fs::write(case_dir.join("db/videos.jsonl"), b"").unwrap();
        fs::write(case_dir.join("db/video_paths.tsv"), b"id\tsource_path\n").unwrap();
        fs::write(&outside, b"outside").unwrap();
        symlink(&outside, case_dir.join("evidence/logs/leak.txt")).unwrap();

        let err = package_case(&case_dir, Some(&output_dir)).unwrap_err();
        assert!(err.contains("unsupported symlink"));

        let _ = fs::remove_dir_all(root);
    }
}
