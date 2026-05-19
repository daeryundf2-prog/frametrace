use crate::audit;
use crate::util::{json_escape, now_unix, unique_path, write_text};
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
        None => unique_path(
            &case_dir
                .join("reports")
                .join(format!("package_{created_unix}")),
        ),
    };
    fs::create_dir_all(&output_dir)
        .map_err(|err| format!("failed to create package output: {err}"))?;

    let mut files = Vec::new();
    for rel in fixed_package_files() {
        copy_package_file(case_dir, &output_dir, Path::new(rel), &mut files)?;
    }
    for rel_dir in recursive_package_dirs() {
        copy_package_dir(case_dir, &output_dir, Path::new(rel_dir), &mut files)?;
    }
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    let checksum_text = files
        .iter()
        .map(|file| format!("{}  {}\n", file.sha256, file.relative_path.display()))
        .collect::<String>();
    let checksum_path = output_dir.join("manifest.sha256");
    write_text(&checksum_path, &checksum_text)
        .map_err(|err| format!("failed to write package checksum manifest: {err}"))?;

    let manifest_json = package_manifest_json(created_unix, &files);
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

fn fixed_package_files() -> &'static [&'static str] {
    &[
        "case.json",
        "db/case.db",
        "db/video_index.json",
        "db/videos.jsonl",
        "db/video_paths.tsv",
        "db/carve_results.json",
        "review/index.html",
        "reports/case-report.html",
    ]
}

fn recursive_package_dirs() -> &'static [&'static str] {
    &[
        "evidence/logs",
        "artifacts/clips",
        "artifacts/proxies",
        "artifacts/thumbnails",
        "artifacts/carved",
    ]
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
        if path.is_dir() {
            copy_package_dir(case_dir, output_dir, &rel, files)?;
        } else if path.is_file() {
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
    if !source.is_file() {
        return Ok(());
    }
    let target = output_dir.join(rel);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create package directory: {err}"))?;
    }
    fs::copy(&source, &target).map_err(|err| {
        format!(
            "failed to copy package file {} to {}: {err}",
            source.display(),
            target.display()
        )
    })?;
    let metadata = fs::metadata(&target)
        .map_err(|err| format!("failed to read package file metadata: {err}"))?;
    files.push(PackageFile {
        relative_path: rel.to_path_buf(),
        sha256: audit::digest_file(&target)?,
        size_bytes: metadata.len(),
    });
    Ok(())
}

fn package_manifest_json(created_unix: u64, files: &[PackageFile]) -> String {
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
    out.push_str("  \"pdf_ready_note\": \"Open reports/case-report.html and print to PDF after examiner review.\"\n");
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::package_case;
    use std::fs;

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
        fs::write(case_dir.join("db/videos.jsonl"), b"").unwrap();
        fs::write(case_dir.join("reports/case-report.html"), b"<html></html>").unwrap();

        let result = package_case(&case_dir, Some(&output_dir)).unwrap();
        assert_eq!(result.file_count, 3);
        assert!(output_dir.join("case.json").is_file());
        assert!(output_dir.join("manifest.sha256").is_file());
        assert!(result.manifest_path.is_file());

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
}
