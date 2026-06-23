use crate::audit;
use crate::tool_policy::{command_version, resolve_tool_binary};
use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) fn default_raw_filename(e01_path: &Path) -> String {
    let stem = e01_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("e01-export");
    format!("{stem}.raw")
}

pub(super) fn ewfexport_args(
    e01_path: &Path,
    export_target: &Path,
    max_bytes: Option<u64>,
    export_log_path: &Path,
) -> Vec<String> {
    let mut args = vec![
        "-u".to_string(),
        "-q".to_string(),
        "-f".to_string(),
        "raw".to_string(),
        "-l".to_string(),
        audit::path_string(export_log_path),
    ];
    if let Some(max_bytes) = max_bytes {
        args.push("-B".to_string());
        args.push(max_bytes.to_string());
    }
    args.extend([
        "-t".to_string(),
        audit::path_string(export_target),
        audit::path_string(e01_path),
    ]);
    args
}

pub(super) fn ewfexport_target_for_output(raw_path: &Path) -> PathBuf {
    if raw_path.extension().is_some() {
        raw_path.with_extension("")
    } else {
        raw_path.to_path_buf()
    }
}

pub(super) fn expected_ewfexport_output(export_target: &Path) -> PathBuf {
    export_target.with_extension("raw")
}

pub(super) fn resolve_ewfexport_output(expected: &Path) -> Result<PathBuf, String> {
    if expected.is_file() {
        return expected
            .canonicalize()
            .map_err(|err| format!("failed to canonicalize raw E01 output: {err}"));
    }

    Err(format!(
        "ewfexport completed but output file was not found at {}",
        expected.display()
    ))
}

pub(super) fn run_capture(
    binary: &str,
    allowed: &[&str],
    args: &[&str],
) -> Result<CommandOutput, String> {
    let resolved_binary = resolve_tool_binary(binary, allowed)
        .map_err(|err| format!("{err} (install libewf tools and ensure {binary} is in PATH)"))?;
    let output = Command::new(&resolved_binary)
        .args(args)
        .output()
        .map_err(|err| {
            format!(
                "failed to run {binary}: {err} (install libewf tools and ensure {binary} is in PATH)"
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "{binary} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(CommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
    })
}

pub(super) fn run_status(binary: &str, allowed: &[&str], args: &[String]) -> Result<(), String> {
    let resolved_binary = resolve_tool_binary(binary, allowed)
        .map_err(|err| format!("{err} (install libewf tools and ensure {binary} is in PATH)"))?;
    let output = Command::new(&resolved_binary)
        .args(args)
        .output()
        .map_err(|err| {
            format!(
                "failed to run {binary}: {err} (install libewf tools and ensure {binary} is in PATH)"
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "{binary} failed: {}{}",
            String::from_utf8_lossy(&output.stderr).trim(),
            String::from_utf8_lossy(&output.stdout).trim()
        ));
    }
    Ok(())
}

pub(super) fn ewf_command_version(binary: &str, allowed: &[&str]) -> String {
    command_version(binary, allowed, "-V")
}

pub(super) struct CommandOutput {
    pub(super) stdout: String,
}

#[cfg(test)]
mod tests {
    use super::{
        default_raw_filename, ewf_command_version, ewfexport_args, ewfexport_target_for_output,
        expected_ewfexport_output, resolve_ewfexport_output,
    };
    use std::fs;
    use std::path::Path;

    #[test]
    fn builds_default_raw_filename() {
        assert_eq!(
            default_raw_filename(Path::new("blackbox.E01")),
            "blackbox.raw"
        );
    }

    #[test]
    fn builds_ewfexport_args() {
        let args = ewfexport_args(
            Path::new("input.E01"),
            Path::new("output"),
            Some(1024),
            Path::new("export.log"),
        );
        assert!(args.contains(&"-u".to_string()));
        assert!(args.contains(&"-B".to_string()));
        assert_eq!(args.last().map(String::as_str), Some("input.E01"));
    }

    #[test]
    fn maps_requested_raw_path_to_ewfexport_target() {
        assert_eq!(
            ewfexport_target_for_output(Path::new("output.raw")),
            Path::new("output")
        );
        assert_eq!(
            expected_ewfexport_output(Path::new("output")),
            Path::new("output.raw")
        );
    }

    #[test]
    fn resolves_requested_ewfexport_output() {
        let dir =
            std::env::temp_dir().join(format!("frametrace-e01-output-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.raw");
        fs::write(&path, b"raw").unwrap();
        assert_eq!(
            resolve_ewfexport_output(&path).unwrap(),
            path.canonicalize().unwrap()
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_ewf_version_reports_unavailable() {
        assert!(
            ewf_command_version("frametrace-missing-ewf-binary", &["ewfinfo"])
                .contains("unavailable")
        );
    }
}
