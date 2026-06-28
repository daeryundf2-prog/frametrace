#[cfg(unix)]
use super::resolve_external_tool;
use super::{reject_source_output_path, require_case_output_path, resolve_tool_binary};
use std::fs;
#[cfg(unix)]
use std::path::Path;

#[test]
fn rejects_unapproved_bare_tool_names() {
    let err = resolve_tool_binary("sh", &["ffprobe"]).unwrap_err();
    assert!(err.contains("unsupported tool binary"));
}

#[test]
fn accepts_allowed_bare_tool_names() {
    assert_eq!(
        resolve_tool_binary("ffprobe", &["ffprobe"]).unwrap(),
        "ffprobe"
    );
}

#[cfg(unix)]
#[test]
fn resolves_approved_ffmpeg_path_and_rejects_disallowed_path_name() {
    let base = std::env::temp_dir().join(format!(
        "frametrace-tool-policy-ffmpeg-test-{}",
        std::process::id()
    ));
    fs::create_dir_all(&base).unwrap();
    let approved = base.join("ffmpeg");
    let rejected = base.join("fake-ffmpeg");
    write_executable(&approved, "#!/bin/sh\necho 'ffmpeg fake 1.0'\n");
    write_executable(&rejected, "#!/bin/sh\necho 'fake ffmpeg'\n");

    let tool = resolve_external_tool(&approved.to_string_lossy(), &["ffmpeg"], "-version").unwrap();
    assert_eq!(tool.name(), "ffmpeg");
    assert_eq!(
        tool.path(),
        approved
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .to_string()
    );
    assert_eq!(tool.version(), "ffmpeg fake 1.0");

    let err =
        resolve_external_tool(&rejected.to_string_lossy(), &["ffmpeg"], "-version").unwrap_err();
    assert!(err.contains("unsupported tool binary"));
    let _ = fs::remove_dir_all(base);
}

#[test]
fn rejects_case_output_outside_case_root() {
    let base = std::env::temp_dir().join(format!(
        "frametrace-output-policy-test-{}",
        std::process::id()
    ));
    let case_dir = base.join("case");
    fs::create_dir_all(&case_dir).unwrap();
    let outside = base.join("outside.mp4");
    let err = require_case_output_path(&case_dir, &outside, "test").unwrap_err();
    assert!(err.contains("inside the case directory"));
    let _ = fs::remove_dir_all(base);
}

#[test]
fn accepts_case_output_inside_case_root() {
    let base = std::env::temp_dir().join(format!(
        "frametrace-output-policy-inside-test-{}",
        std::process::id()
    ));
    let case_dir = base.join("case");
    fs::create_dir_all(&case_dir).unwrap();
    let inside = case_dir.join("artifacts/out.mp4");
    require_case_output_path(&case_dir, &inside, "test").unwrap();
    let _ = fs::remove_dir_all(base);
}

#[cfg(unix)]
#[test]
fn rejects_case_output_dangling_symlink_leaf() {
    use std::os::unix::fs::symlink;

    let base = std::env::temp_dir().join(format!(
        "frametrace-output-policy-symlink-test-{}",
        std::process::id()
    ));
    let case_dir = base.join("case");
    fs::create_dir_all(case_dir.join("artifacts")).unwrap();
    let output = case_dir.join("artifacts/out.mp4");
    let outside = base.join("outside.mp4");
    symlink(&outside, &output).unwrap();

    let err = require_case_output_path(&case_dir, &output, "test").unwrap_err();

    assert!(err.contains("cannot be a symlink"));
    let _ = fs::remove_dir_all(base);
}

#[test]
fn rejects_output_that_targets_source_evidence_path() {
    let base = std::env::temp_dir().join(format!(
        "frametrace-source-output-policy-test-{}",
        std::process::id()
    ));
    let case_dir = base.join("case");
    fs::create_dir_all(&case_dir).unwrap();
    let source = case_dir.join("source.mp4");
    fs::write(&source, b"source").unwrap();

    let err = reject_source_output_path(&source, &source, "proxy").unwrap_err();

    assert!(err.contains("cannot target the source evidence path"));
    let _ = fs::remove_dir_all(base);
}

#[cfg(unix)]
fn write_executable(path: &Path, contents: &str) {
    use std::os::unix::fs::PermissionsExt;

    fs::write(path, contents).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}
