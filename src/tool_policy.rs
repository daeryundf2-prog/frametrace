use std::path::{Component, Path, PathBuf};
use std::process::Command;

pub fn resolve_tool_binary(input: &str, allowed_names: &[&str]) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "tool binary must not be empty; allowed tools: {}",
            allowed_names.join(", ")
        ));
    }

    if is_allowed_tool_name(trimmed, allowed_names) {
        return Ok(resolve_bare_tool_name(trimmed, allowed_names));
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
    match Command::new(&binary).arg(version_arg).output() {
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

pub fn require_case_output_path(
    case_dir: &Path,
    output_path: &Path,
    label: &str,
) -> Result<(), String> {
    let case_root = case_dir
        .canonicalize()
        .map_err(|err| format!("failed to canonicalize case directory: {err}"))?;
    let absolute_output = lexical_absolute_path(output_path)
        .map_err(|err| format!("failed to resolve {label} output path: {err}"))?;
    let existing_parent = nearest_existing_parent(
        absolute_output
            .parent()
            .ok_or_else(|| format!("{label} output path has no parent"))?,
    );
    let canonical_parent = existing_parent
        .canonicalize()
        .map_err(|err| format!("failed to canonicalize {label} output parent: {err}"))?;
    if !canonical_parent.starts_with(&case_root) {
        return Err(format!(
            "{label} output must be inside the case directory {}; parent resolves to {}",
            case_root.display(),
            canonical_parent.display()
        ));
    }

    Ok(())
}

fn is_allowed_tool_name(candidate: &str, allowed_names: &[&str]) -> bool {
    allowed_names.iter().any(|allowed| {
        candidate == *allowed
            || candidate
                .strip_suffix(".exe")
                .is_some_and(|without_exe| without_exe == *allowed)
    })
}

/// Bare tool names are resolved against PATH here instead of being handed to
/// `Command::new`, because the Windows loader also searches the current
/// directory, which would let a planted binary ride along with evidence media.
/// When the tool cannot be found, the bare name is returned so the downstream
/// failure message stays identical.
fn resolve_bare_tool_name(name: &str, allowed_names: &[&str]) -> String {
    let path_var = std::env::var("PATH").unwrap_or_default();
    if let Some(path) = find_in_path_dirs(name, allowed_names, &path_var) {
        return path.to_string_lossy().to_string();
    }
    // Portable layout: `<exe dir>/tools/bin` ships optional forensic tools
    // next to the binary so examiners never need admin rights to extend PATH.
    // The directory is part of the trusted install tree, unlike the CWD.
    if let Some(found) = find_in_tools_bin(name, allowed_names) {
        return found;
    }
    name.to_string()
}

/// Whether an optional forensic tool has been dropped into the portable
/// `tools/bin` directory next to the executable.
pub fn tools_bin_tool_exists(name: &str) -> bool {
    let Some(dir) = tools_bin_dir() else {
        return false;
    };
    [name.to_string(), format!("{name}.exe")]
        .iter()
        .any(|candidate| dir.join(candidate).is_file())
}

fn tools_bin_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join("tools").join("bin"))
}

fn find_in_tools_bin(name: &str, allowed_names: &[&str]) -> Option<String> {
    let dir = tools_bin_dir()?;
    for candidate_name in [name.to_string(), format!("{name}.exe")] {
        let candidate = dir.join(&candidate_name);
        let Ok(metadata) = std::fs::metadata(&candidate) else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let Ok(canonical) = candidate.canonicalize() else {
            continue;
        };
        let file_name = canonical.file_name().and_then(|n| n.to_str());
        if file_name.map(|n| is_allowed_tool_name(n, allowed_names)) == Some(true) {
            return Some(canonical.to_string_lossy().to_string());
        }
    }
    None
}

fn find_in_path_dirs(name: &str, allowed_names: &[&str], path_var: &str) -> Option<PathBuf> {
    for dir in std::env::split_paths(path_var) {
        // An empty PATH entry conventionally means the current directory.
        if dir.as_os_str().is_empty() {
            continue;
        }
        let Ok(dir) = dir.canonicalize() else {
            continue;
        };
        for candidate_name in [name.to_string(), format!("{name}.exe")] {
            let candidate = dir.join(&candidate_name);
            let Ok(metadata) = std::fs::metadata(&candidate) else {
                continue;
            };
            if !metadata.is_file() {
                continue;
            }
            let Ok(canonical) = candidate.canonicalize() else {
                continue;
            };
            if let Some(file_name) = canonical.file_name().and_then(|file| file.to_str())
                && is_allowed_tool_name(file_name, allowed_names)
            {
                return Some(canonical);
            }
        }
    }
    None
}

fn looks_like_path(value: &str) -> bool {
    value.contains('/')
        || value.contains('\\')
        || value.starts_with('.')
        || Path::new(value).components().count() > 1
}

fn lexical_absolute_path(path: &Path) -> Result<PathBuf, std::io::Error> {
    let raw = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(normalize_lexically(&raw))
}

fn normalize_lexically(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(component.as_os_str()),
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

fn nearest_existing_parent(path: &Path) -> PathBuf {
    let mut current = path.to_path_buf();
    loop {
        if current.exists() {
            return current;
        }
        if !current.pop() {
            return PathBuf::from(".");
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn resolves_bare_tools_from_exe_relative_tools_bin() {
        let exe = std::env::current_exe().unwrap();
        let tools_bin = exe.parent().unwrap().join("tools").join("bin");
        std::fs::create_dir_all(&tools_bin).unwrap();
        let probe = tools_bin.join("frametrace-probe-icat.exe");
        std::fs::write(&probe, b"stub").unwrap();
        let resolved =
            super::resolve_bare_tool_name("frametrace-probe-icat", &["frametrace-probe-icat"]);
        assert!(
            resolved.ends_with("frametrace-probe-icat.exe"),
            "unexpected resolution: {resolved}"
        );
        let _ = std::fs::remove_file(&probe);
    }

    use super::{find_in_path_dirs, require_case_output_path, resolve_tool_binary};
    use std::fs;
    use std::path::Path;

    #[test]
    fn rejects_unapproved_bare_tool_names() {
        let err = resolve_tool_binary("sh", &["ffprobe"]).unwrap_err();
        assert!(err.contains("unsupported tool binary"));
    }

    #[test]
    fn accepted_bare_tool_names_resolve_to_allowed_files_or_bare_name() {
        let resolved = resolve_tool_binary("ffprobe", &["ffprobe"]).unwrap();
        let file_name = Path::new(&resolved)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(resolved.as_str());
        assert!(
            file_name == "ffprobe"
                || file_name
                    .strip_suffix(".exe")
                    .is_some_and(|without_exe| without_exe == "ffprobe"),
            "unexpected resolution: {resolved}"
        );
    }

    #[test]
    fn finds_tool_in_custom_path_dirs_without_current_directory() {
        let base = std::env::temp_dir().join(format!(
            "frametrace-path-search-test-{}",
            std::process::id()
        ));
        let tool_dir = base.join("tools");
        let dot_dir = base.join(".");
        fs::create_dir_all(&tool_dir).unwrap();
        fs::write(tool_dir.join("ffprobe.exe"), b"not really ffprobe").unwrap();

        let custom_path = format!(";{};{}", dot_dir.display(), tool_dir.display());
        let found = find_in_path_dirs("ffprobe", &["ffprobe"], &custom_path).unwrap();
        assert_eq!(found.file_name().unwrap(), "ffprobe.exe");

        // A directory that only holds an unapproved variant must not match.
        let impostor_dir = base.join("impostor");
        fs::create_dir_all(&impostor_dir).unwrap();
        fs::write(impostor_dir.join("ffprobe.bat"), b"impostor").unwrap();
        let impostor_path = impostor_dir.display().to_string();
        assert!(find_in_path_dirs("ffprobe", &["ffprobe"], &impostor_path).is_none());

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
}
