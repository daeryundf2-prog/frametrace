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
    use super::{require_case_output_path, resolve_tool_binary};
    use std::fs;

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
