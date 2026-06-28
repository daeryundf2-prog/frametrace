use crate::audit;
use crate::distributable_redaction::{
    RedactionPolicy, redact_generated_html_for_distributable, redact_json_for_distributable,
    redact_jsonl_for_distributable, redact_sqlite_copy_for_distributable,
    redact_tsv_for_distributable, write_full_path_disclosure_artifact,
};
use crate::tool_policy::require_case_output_path;
use crate::util::{json_escape, now_unix, read_to_string, unique_path, write_text};
use std::fs;
use std::io::ErrorKind;
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

pub fn package_case(
    case_dir: &Path,
    output_dir: Option<&Path>,
    redaction_policy: RedactionPolicy,
) -> Result<PackageResult, String> {
    let created_unix = now_unix()?;
    let output_dir = match output_dir {
        Some(path) => {
            reject_recursive_package_output(case_dir, path)?;
            reject_package_output_symlink(path)?;
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
        None => {
            let path = unique_path(
                &case_dir
                    .join("reports")
                    .join(format!("package_{created_unix}")),
            );
            require_case_output_path(case_dir, &path, "case package")?;
            path
        }
    };
    validate_required_package_files(case_dir)?;
    fs::create_dir_all(&output_dir)
        .map_err(|err| format!("failed to create package output: {err}"))?;

    let mut files = Vec::new();
    let mut missing_optional_files = Vec::new();
    for rel in required_package_files() {
        copy_package_file(
            case_dir,
            &output_dir,
            Path::new(rel),
            &mut files,
            redaction_policy,
        )?;
    }
    for rel in optional_package_files() {
        copy_optional_package_file(
            case_dir,
            &output_dir,
            Path::new(rel),
            &mut files,
            &mut missing_optional_files,
            redaction_policy,
        )?;
    }
    copy_markdown_reports(case_dir, &output_dir, &mut files, redaction_policy)?;
    for rel_dir in recursive_package_dirs() {
        copy_package_dir(
            case_dir,
            &output_dir,
            Path::new(rel_dir),
            &mut files,
            redaction_policy,
        )?;
    }
    let disclosure_path =
        write_full_path_disclosure_artifact(&output_dir, "package", redaction_policy)?;
    if let Some(path) = disclosure_path {
        files.push(PackageFile {
            relative_path: PathBuf::from(crate::distributable_redaction::FULL_PATH_DISCLOSURE_FILE),
            sha256: audit::digest_file(&path)?,
            size_bytes: fs::metadata(&path)
                .map_err(|err| format!("failed to inspect disclosure artifact: {err}"))?
                .len(),
        });
    }
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    let checksum_text = files
        .iter()
        .map(|file| format!("{}  {}\n", file.sha256, file.relative_path.display()))
        .collect::<String>();
    let checksum_path = output_dir.join("manifest.sha256");
    write_text(&checksum_path, &checksum_text)
        .map_err(|err| format!("failed to write package checksum manifest: {err}"))?;

    let manifest_json = package_manifest_json(
        created_unix,
        &files,
        &missing_optional_files,
        redaction_policy,
    );
    let manifest_path = output_dir.join("package-manifest.json");
    write_text(&manifest_path, &manifest_json)
        .map_err(|err| format!("failed to write package manifest: {err}"))?;

    write_text(
        &output_dir.join("README.txt"),
        "FrameTrace case package\n\nOpen reports/case-report.html for the HTML report. Use the browser print dialog to create a PDF when required by the engagement. Verify package contents with manifest.sha256 before transfer.\n",
    )
    .map_err(|err| format!("failed to write package README: {err}"))?;

    Ok(PackageResult {
        output_dir,
        file_count: files.len(),
        manifest_path,
    })
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

fn reject_package_output_symlink(output_dir: &Path) -> Result<(), String> {
    match fs::symlink_metadata(output_dir) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(format!(
            "package output cannot be a symlink: {}",
            output_dir.display()
        )),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "failed to inspect package output {}: {err}",
            output_dir.display()
        )),
    }
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
        "reports/case-report.html",
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
        "artifacts/frames",
        "artifacts/carved",
        "artifacts/recovered",
        "db/filesystem",
    ]
}

fn copy_markdown_reports(
    case_dir: &Path,
    output_dir: &Path,
    files: &mut Vec<PackageFile>,
    policy: RedactionPolicy,
) -> Result<(), String> {
    let reports_dir = case_dir.join("reports");
    if !reports_dir.is_dir() {
        return Ok(());
    }

    let entries = fs::read_dir(&reports_dir)
        .map_err(|err| format!("failed to read reports directory: {err}"))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read reports entry: {err}"))?;
        let path = entry.path();
        if !path.is_file()
            || !path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        {
            continue;
        }
        copy_package_file(
            case_dir,
            output_dir,
            &Path::new("reports").join(entry.file_name()),
            files,
            policy,
        )?;
    }
    Ok(())
}

fn copy_package_dir(
    case_dir: &Path,
    output_dir: &Path,
    rel_dir: &Path,
    files: &mut Vec<PackageFile>,
    policy: RedactionPolicy,
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
            copy_package_dir(case_dir, output_dir, &rel, files, policy)?;
        } else if file_type.is_file() {
            copy_package_file(case_dir, output_dir, &rel, files, policy)?;
        }
    }
    Ok(())
}

fn copy_package_file(
    case_dir: &Path,
    output_dir: &Path,
    rel: &Path,
    files: &mut Vec<PackageFile>,
    policy: RedactionPolicy,
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
    copy_package_file_with_policy(case_dir, rel, &source, &target, policy)?;
    if rel == Path::new("db/case.db") {
        redact_sqlite_copy_for_distributable(&target, case_dir, policy)?;
    }
    files.push(PackageFile {
        relative_path: rel.to_path_buf(),
        sha256: audit::digest_file(&target)?,
        size_bytes: fs::metadata(&target)
            .map_err(|err| format!("failed to inspect package target: {err}"))?
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
    policy: RedactionPolicy,
) -> Result<(), String> {
    if !case_dir.join(rel).is_file() {
        missing_optional_files.push(rel.to_path_buf());
        return Ok(());
    }
    copy_package_file(case_dir, output_dir, rel, files, policy)
}

fn copy_package_file_with_policy(
    case_dir: &Path,
    rel: &Path,
    source: &Path,
    target: &Path,
    policy: RedactionPolicy,
) -> Result<(), String> {
    let extension = rel
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    let redacted = if extension.eq_ignore_ascii_case("json") {
        Some(redact_json_for_distributable(
            case_dir,
            &read_to_string(source).map_err(|err| {
                format!("failed to read package source {}: {err}", source.display())
            })?,
            policy,
        )?)
    } else if extension.eq_ignore_ascii_case("jsonl") {
        Some(redact_jsonl_for_distributable(
            case_dir,
            &read_to_string(source).map_err(|err| {
                format!("failed to read package source {}: {err}", source.display())
            })?,
            policy,
        )?)
    } else if extension.eq_ignore_ascii_case("tsv") {
        Some(redact_tsv_for_distributable(
            &read_to_string(source).map_err(|err| {
                format!("failed to read package source {}: {err}", source.display())
            })?,
            policy,
        ))
    } else if extension.eq_ignore_ascii_case("html") {
        Some(redact_generated_html_for_distributable(
            case_dir,
            &read_to_string(source).map_err(|err| {
                format!("failed to read package source {}: {err}", source.display())
            })?,
            policy,
        )?)
    } else {
        None
    };

    if let Some(text) = redacted {
        write_text(target, &text)
            .map_err(|err| format!("failed to write redacted package file: {err}"))?;
    } else {
        fs::copy(source, target).map_err(|err| {
            format!(
                "failed to copy package file {} to {}: {err}",
                source.display(),
                target.display()
            )
        })?;
    }
    Ok(())
}

fn package_manifest_json(
    created_unix: u64,
    files: &[PackageFile],
    missing_optional_files: &[PathBuf],
    redaction_policy: RedactionPolicy,
) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"schema_version\": 1,\n");
    out.push_str("  \"package_type\": \"frametrace-case-package\",\n");
    out.push_str(&format!(
        "  \"path_disclosure_mode\": \"{}\",\n",
        redaction_policy.mode_label()
    ));
    out.push_str(&format!(
        "  \"local_operator_full_path_disclosure\": {},\n",
        redaction_policy.mode()
            == crate::distributable_redaction::PathDisclosureMode::LocalOperatorFullPaths
    ));
    out.push_str(&format!("  \"created_unix\": {},\n", created_unix));
    out.push_str(&format!("  \"file_count\": {},\n", files.len()));
    out.push_str("  \"files\": [\n");
    for (index, file) in files.iter().enumerate() {
        out.push_str(&format!(
            "    {{\"relative_path\":\"{}\",\"size_bytes\":{},\"sha256\":\"{}\"}}",
            json_escape(&file.relative_path.to_string_lossy()),
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
    out.push_str("  \"pdf_ready_note\": \"Open reports/case-report.html and print to PDF after examiner review.\"\n");
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use crate::distributable_redaction::RedactionPolicy;

    use super::package_case;
    use serde_json::{Value, json};
    use std::fs;

    fn jsonl(value: Value) -> String {
        format!("{value}\n")
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
        fs::write(case_dir.join("db/case.db"), b"sqlite placeholder").unwrap();
        fs::write(case_dir.join("db/video_index.json"), b"{}").unwrap();
        fs::write(case_dir.join("db/videos.jsonl"), b"").unwrap();
        fs::write(case_dir.join("db/video_paths.tsv"), b"id\tsource_path\n").unwrap();
        fs::write(case_dir.join("reports/case-report.html"), b"<html></html>").unwrap();
        fs::write(case_dir.join("reports/summary.md"), b"# Summary").unwrap();

        let result =
            package_case(&case_dir, Some(&output_dir), RedactionPolicy::redacted()).unwrap();
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
    fn package_redacts_distributable_copies_by_default() {
        let root = std::env::temp_dir().join(format!(
            "frametrace-package-redaction-test-{}",
            std::process::id()
        ));
        let case_dir = root.join("case");
        let output_dir = root.join("package");
        let source_path = root.join("Client ACME/source clip.mp4");
        let frame_path = case_dir.join("artifacts/frames/frame.jpg");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(case_dir.join("db")).unwrap();
        fs::create_dir_all(case_dir.join("artifacts/frames")).unwrap();
        fs::create_dir_all(case_dir.join("review")).unwrap();
        fs::create_dir_all(source_path.parent().unwrap()).unwrap();
        fs::write(case_dir.join("case.json"), b"{}").unwrap();
        fs::write(case_dir.join("db/case.db"), b"sqlite placeholder").unwrap();
        let source_path_text = source_path.to_string_lossy().to_string();
        let source_file_url = format!("file://{source_path_text}");
        let frame_path_text = frame_path.to_string_lossy().to_string();
        fs::write(
            case_dir.join("db/video_index.json"),
            json!({
                "videos": [{
                    "id": "vid_000001",
                    "source_path": source_path_text,
                    "file_url": source_file_url,
                    "relative_path": "Camera/source clip.mp4",
                }]
            })
            .to_string(),
        )
        .unwrap();
        fs::write(
            case_dir.join("db/videos.jsonl"),
            jsonl(json!({
                "id": "vid_000001",
                "source_path": source_path_text,
                "file_url": source_file_url,
            })),
        )
        .unwrap();
        fs::write(
            case_dir.join("db/video_paths.tsv"),
            format!("id\tsource_path\nvid_000001\t{}\n", source_path.display()),
        )
        .unwrap();
        fs::write(
            case_dir.join("artifacts/frames/frame-log.jsonl"),
            jsonl(json!({
                "event": "make-frame-capture",
                "derived_artifact_id": "derived-frame",
                "source_path": source_path_text,
                "output_path": frame_path_text,
            })),
        )
        .unwrap();
        fs::write(&frame_path, b"frame").unwrap();
        fs::write(
            case_dir.join("review/index.html"),
            format!(
                r#"<script>
const scan = {};
</script>"#,
                json!({
                    "videos": [{
                        "id": "vid_000001",
                        "source_path": source_path_text,
                        "file_url": source_file_url,
                    }]
                })
            ),
        )
        .unwrap();

        package_case(&case_dir, Some(&output_dir), RedactionPolicy::redacted()).unwrap();

        let package_text = fs::read_to_string(output_dir.join("db/video_index.json")).unwrap()
            + &fs::read_to_string(output_dir.join("db/videos.jsonl")).unwrap()
            + &fs::read_to_string(output_dir.join("db/video_paths.tsv")).unwrap()
            + &fs::read_to_string(output_dir.join("artifacts/frames/frame-log.jsonl")).unwrap()
            + &fs::read_to_string(output_dir.join("review/index.html")).unwrap();
        assert!(!package_text.contains(&root.to_string_lossy().to_string()));
        assert!(package_text.contains("[redacted-source:vid_000001]"));
        assert!(package_text.contains("artifacts/frames/frame.jpg"));
        assert!(
            !output_dir
                .join("privacy-full-path-disclosure.json")
                .exists()
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn package_opt_in_keeps_paths_and_writes_disclosure() {
        let root = std::env::temp_dir().join(format!(
            "frametrace-package-disclosure-test-{}",
            std::process::id()
        ));
        let case_dir = root.join("case");
        let output_dir = root.join("package");
        let source_path = root.join("Client ACME/source clip.mp4");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(case_dir.join("db")).unwrap();
        fs::create_dir_all(source_path.parent().unwrap()).unwrap();
        fs::write(case_dir.join("case.json"), b"{}").unwrap();
        fs::write(case_dir.join("db/case.db"), b"sqlite placeholder").unwrap();
        let source_path_text = source_path.to_string_lossy().to_string();
        fs::write(
            case_dir.join("db/video_index.json"),
            json!({
                "videos": [{
                    "id": "vid_000001",
                    "source_path": source_path_text,
                }]
            })
            .to_string(),
        )
        .unwrap();
        fs::write(
            case_dir.join("db/videos.jsonl"),
            jsonl(json!({
                "id": "vid_000001",
                "source_path": source_path_text,
            })),
        )
        .unwrap();
        fs::write(
            case_dir.join("db/video_paths.tsv"),
            format!("id\tsource_path\nvid_000001\t{}\n", source_path.display()),
        )
        .unwrap();

        package_case(
            &case_dir,
            Some(&output_dir),
            RedactionPolicy::local_operator_full_paths(),
        )
        .unwrap();

        let index = fs::read_to_string(output_dir.join("db/video_index.json")).unwrap();
        let index_json: Value = serde_json::from_str(&index).unwrap();
        let disclosure =
            fs::read_to_string(output_dir.join("privacy-full-path-disclosure.json")).unwrap();
        let manifest = fs::read_to_string(output_dir.join("package-manifest.json")).unwrap();
        assert_eq!(index_json["videos"][0]["source_path"], source_path_text);
        assert!(disclosure.contains("\"local_operator_full_path_disclosure\": true"));
        assert!(manifest.contains("\"path_disclosure_mode\": \"local_operator_full_paths\""));

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

        let err =
            package_case(&case_dir, Some(&output_dir), RedactionPolicy::redacted()).unwrap_err();
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

        let err =
            package_case(&case_dir, Some(&output_dir), RedactionPolicy::redacted()).unwrap_err();
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
        fs::write(case_dir.join("db/case.db"), b"sqlite placeholder").unwrap();
        fs::write(case_dir.join("db/video_index.json"), b"{}").unwrap();
        fs::write(case_dir.join("db/videos.jsonl"), b"").unwrap();
        fs::write(case_dir.join("db/video_paths.tsv"), b"id\tsource_path\n").unwrap();
        fs::write(&outside, b"outside").unwrap();
        symlink(&outside, case_dir.join("evidence/logs/leak.txt")).unwrap();

        let err =
            package_case(&case_dir, Some(&output_dir), RedactionPolicy::redacted()).unwrap_err();
        assert!(err.contains("unsupported symlink"));

        let _ = fs::remove_dir_all(root);
    }
}
