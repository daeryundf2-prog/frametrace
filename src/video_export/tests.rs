use super::log::{ExportLogBody, export_log_body_json};
use super::{ExportFormat, ExportOptions, ffmpeg_export_args, sanitize_filename, tsv_unescape};
use crate::tool_policy::{ResolvedExternalTool, resolve_external_tool};
use std::fs;
use std::path::Path;

#[test]
fn parses_export_formats() {
    assert_eq!(ExportFormat::parse("mp4").unwrap(), ExportFormat::Mp4);
    assert_eq!(ExportFormat::parse("AVI").unwrap(), ExportFormat::Avi);
    assert!(ExportFormat::parse("mkv").is_err());
}

#[test]
fn sanitizes_clip_names() {
    assert_eq!(sanitize_filename("vid_000001"), "vid_000001");
    assert_eq!(sanitize_filename("a/b:c.mp4"), "a_b_c_mp4");
}

#[test]
fn builds_export_command_args() {
    let options = ExportOptions {
        format: ExportFormat::Mp4,
        start_seconds: Some(1.0),
        duration_seconds: Some(2.0),
        output_path: None,
        operator: Some("qa-operator".to_string()),
        ffmpeg_bin: "ffmpeg".to_string(),
    };
    let args = ffmpeg_export_args(Path::new("in.mp4"), Path::new("out.mp4"), &options);
    assert!(args.contains(&"-n".to_string()));
    assert!(args.contains(&"libx264".to_string()));
    assert_eq!(args.last().map(String::as_str), Some("out.mp4"));
}

#[test]
fn export_log_body_records_derived_artifact_contract() {
    let options = ExportOptions {
        format: ExportFormat::Mp4,
        start_seconds: Some(1.0),
        duration_seconds: Some(2.0),
        output_path: None,
        operator: Some("qa-operator".to_string()),
        ffmpeg_bin: "ffmpeg".to_string(),
    };
    let source_sha256 = Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let output_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    let body = export_log_body_json(ExportLogBody {
        selector: "vid_000001",
        source_path: Path::new("/case/source/clip.mp4"),
        output_path: Path::new("/case/artifacts/clips/clip.mp4"),
        options: &options,
        source_sha256,
        output_sha256,
        exported_unix: 1_789_000_000,
        operator: "qa-operator",
        tool: &fake_tool(),
    });

    assert!(body.contains(r#""event":"export-video""#));
    assert!(body.contains(r#""artifact_state":"derived""#));
    assert!(body.contains(r#""operator":"qa-operator""#));
    assert!(body.contains(r#""method":"ffmpeg-clip-export""#));
    assert!(body.contains(r#""tool":"ffmpeg""#));
    assert!(body.contains(r#""resolved_tool_path":"#));
    assert!(body.contains(r#""derived_artifact_id":"derived-clip-bbbbbbbbbbbb""#));
    assert!(body.contains(r#""source_artifact_id":"source-vid_000001-aaaaaaaaaaaa""#));
    assert!(body.contains(
        r#""output_artifact_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb""#
    ));
}

#[test]
fn unescapes_tsv_paths() {
    assert_eq!(tsv_unescape("a\\tb\\\\c"), "a\tb\\c");
}

fn fake_tool() -> ResolvedExternalTool {
    let path = test_executable_named("ffmpeg");
    let tool = resolve_external_tool(path.to_str().unwrap(), &["ffmpeg"], "-version").unwrap();
    if let Some(parent) = path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
    tool
}

fn test_executable_named(name: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!(
        "frametrace-export-tool-test-{}-{}-{name}",
        std::process::id(),
        unique_test_suffix()
    ));
    fs::create_dir_all(&base).unwrap();
    let path = base.join(executable_name(name));
    fs::copy(std::env::current_exe().unwrap(), &path).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
    }
    path
}

fn executable_name(name: &str) -> String {
    #[cfg(windows)]
    {
        format!("{name}.exe")
    }
    #[cfg(not(windows))]
    {
        name.to_string()
    }
}

fn unique_test_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}
