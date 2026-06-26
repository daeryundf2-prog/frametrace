use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedExternalTool {
    name: String,
    path: String,
    version: String,
}

impl ResolvedExternalTool {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}

pub fn resolve_tool_binary(input: &str, allowed_names: &[&str]) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "tool binary must not be empty; allowed tools: {}",
            allowed_names.join(", ")
        ));
    }

    if is_allowed_tool_name(trimmed, allowed_names) {
        return Ok(trimmed.to_string());
    }

    if !looks_like_path(trimmed) {
        return Err(format!(
            "unsupported tool binary '{trimmed}'; allowed tools: {}",
            allowed_names.join(", ")
        ));
    }

    let canonical = Path::new(trimmed)
        .canonicalize()
        .map_err(|err| format!("failed to canonicalize tool binary '{trimmed}': {err}"))?;
    if !canonical.is_file() {
        return Err(format!(
            "tool binary is not a file: {}",
            canonical.display()
        ));
    }
    let file_name = canonical
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            format!(
                "tool binary has an invalid file name: {}",
                canonical.display()
            )
        })?;
    if !is_allowed_tool_name(file_name, allowed_names) {
        return Err(format!(
            "unsupported tool binary '{}'; allowed tools: {}",
            canonical.display(),
            allowed_names.join(", ")
        ));
    }

    Ok(canonical.to_string_lossy().to_string())
}

pub fn command_version(input: &str, allowed_names: &[&str], version_arg: &str) -> String {
    let binary = match resolve_tool_binary(input, allowed_names) {
        Ok(binary) => binary,
        Err(err) => return format!("unavailable: {err}"),
    };
    version_for_binary(&binary, version_arg)
}

pub fn resolve_external_tool(
    input: &str,
    allowed_names: &[&str],
    version_arg: &str,
) -> Result<ResolvedExternalTool, String> {
    let binary = resolve_tool_binary(input, allowed_names)?;
    let resolved_path = resolve_executable_path(&binary)?;
    let file_name = resolved_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            format!(
                "tool binary has an invalid file name: {}",
                resolved_path.display()
            )
        })?;
    if !is_allowed_tool_name(file_name, allowed_names) {
        return Err(format!(
            "unsupported tool binary '{}'; allowed tools: {}",
            resolved_path.display(),
            allowed_names.join(", ")
        ));
    }
    let name = allowed_names
        .iter()
        .find(|allowed| is_allowed_tool_name(file_name, &[*allowed]))
        .copied()
        .unwrap_or(file_name)
        .to_string();
    let path = resolved_path.to_string_lossy().to_string();
    let version = version_for_binary(&path, version_arg);
    Ok(ResolvedExternalTool {
        name,
        path,
        version,
    })
}

pub fn run_external_tool(tool: &ResolvedExternalTool, args: &[String]) -> Result<Output, String> {
    Command::new(tool.path())
        .args(args)
        .output()
        .map_err(|err| format!("failed to run {} at {}: {err}", tool.name(), tool.path()))
}

fn version_for_binary(binary: &str, version_arg: &str) -> String {
    match Command::new(binary).arg(version_arg).output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            stdout
                .lines()
                .chain(stderr.lines())
                .find(|line| !line.trim().is_empty())
                .unwrap_or("unknown")
                .trim()
                .to_string()
        }
        Ok(output) => format!(
            "unavailable: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(err) => format!("unavailable: {err}"),
    }
}

fn resolve_executable_path(binary: &str) -> Result<PathBuf, String> {
    if looks_like_path(binary) {
        return Path::new(binary)
            .canonicalize()
            .map_err(|err| format!("failed to canonicalize tool binary '{binary}': {err}"));
    }
    find_on_path(binary).ok_or_else(|| format!("tool binary '{binary}' was not found on PATH"))
}

fn find_on_path(binary: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        for candidate in executable_candidates(&dir, binary) {
            if candidate.is_file() {
                return candidate.canonicalize().ok();
            }
        }
    }
    None
}

fn executable_candidates(dir: &Path, binary: &str) -> Vec<PathBuf> {
    let candidate = dir.join(binary);
    #[cfg(windows)]
    {
        let mut candidates = vec![candidate.clone()];
        if Path::new(binary).extension().is_none() {
            candidates.push(dir.join(format!("{binary}.exe")));
        }
        candidates
    }
    #[cfg(not(windows))]
    {
        vec![candidate]
    }
}

fn is_allowed_tool_name(candidate: &str, allowed_names: &[&str]) -> bool {
    allowed_names.iter().any(|allowed| {
        candidate == *allowed
            || candidate
                .strip_suffix(".exe")
                .is_some_and(|without_exe| without_exe == *allowed)
    })
}

fn looks_like_path(value: &str) -> bool {
    value.contains('/')
        || value.contains('\\')
        || value.starts_with('.')
        || Path::new(value).components().count() > 1
}
