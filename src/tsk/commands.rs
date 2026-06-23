use super::types::TskRecoverOptions;
use crate::audit;
use crate::tool_policy::{command_version, resolve_tool_binary};
use std::fs::{self, File};
use std::path::Path;
use std::process::{Command, Stdio};

pub(super) fn run_capture(
    binary: &str,
    allowed: &[&str],
    args: &[String],
) -> Result<CommandOutput, String> {
    let resolved_binary = resolve_tool_binary(binary, allowed)
        .map_err(|err| format!("{err} (install Sleuth Kit and ensure {binary} is in PATH)"))?;
    let output = Command::new(&resolved_binary)
        .args(args)
        .output()
        .map_err(|err| format!("failed to run {binary}: {err}"))?;
    Ok(CommandOutput {
        status_success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

pub(super) fn run_icat_to_file(
    binary: &str,
    args: &[String],
    output_path: &Path,
    inode: &str,
    partition_offset: u64,
) -> Result<(), String> {
    let icat_bin = resolve_tool_binary(binary, &["icat"])
        .map_err(|err| format!("{err} (install Sleuth Kit and ensure icat is in PATH)"))?;
    let output = File::create(output_path)
        .map_err(|err| format!("failed to create {}: {err}", output_path.display()))?;
    let result = Command::new(&icat_bin)
        .args(args)
        .stdout(Stdio::from(output))
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| format!("failed to run {binary}: {err}"))?;
    if !result.status.success() {
        let _ = fs::remove_file(output_path);
        return Err(format!(
            "icat failed for inode {} at offset {}: {}",
            inode,
            partition_offset,
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub(super) struct CommandOutput {
    pub(super) status_success: bool,
    pub(super) stdout: String,
    pub(super) stderr: String,
}

impl CommandOutput {
    pub(super) fn combined_text(&self) -> String {
        format!(
            "status_success: {}\n\nstdout:\n{}\n\nstderr:\n{}\n",
            self.status_success, self.stdout, self.stderr
        )
    }
}

pub(super) fn tsk_command_version(binary: &str, allowed: &[&str]) -> String {
    command_version(binary, allowed, "-V")
}

pub(super) fn fls_args(image_path: &Path, offset: u64) -> Vec<String> {
    vec![
        "-r".to_string(),
        "-p".to_string(),
        "-o".to_string(),
        offset.to_string(),
        tsk_path_string(image_path),
    ]
}

pub(super) fn icat_args(image_path: &Path, options: &TskRecoverOptions) -> Vec<String> {
    let mut args = Vec::new();
    if options.skip_sparse_holes {
        args.push("-h".to_string());
    }
    if options.recover_deleted {
        args.push("-r".to_string());
    }
    if options.include_slack {
        args.push("-s".to_string());
    }
    args.extend([
        "-o".to_string(),
        options.partition_offset.to_string(),
        tsk_path_string(image_path),
        options.inode.clone(),
    ]);
    args
}

pub(super) fn tsk_path_string(path: &Path) -> String {
    let raw = audit::path_string(path);
    raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_string()
}

pub(super) fn sanitize_filename(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "unknown".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{fls_args, icat_args};
    use crate::tsk::TskRecoverOptions;
    use std::path::Path;

    #[test]
    fn builds_tsk_command_args() {
        let image = Path::new("/cases/image.raw");
        assert_eq!(
            fls_args(image, 2048),
            vec!["-r", "-p", "-o", "2048", "/cases/image.raw"]
        );

        let options = TskRecoverOptions {
            partition_offset: 2048,
            inode: "1304-128-1".to_string(),
            output_path: None,
            recover_deleted: true,
            include_slack: false,
            skip_sparse_holes: true,
            icat_bin: "icat".to_string(),
        };
        assert_eq!(
            icat_args(image, &options),
            vec!["-h", "-r", "-o", "2048", "/cases/image.raw", "1304-128-1"]
        );
    }
}
