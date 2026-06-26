use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

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
    reject_output_symlink_leaf(&absolute_output, label)?;

    Ok(())
}

pub fn reject_source_output_path(
    source_path: &Path,
    output_path: &Path,
    label: &str,
) -> Result<(), String> {
    let canonical_source = source_path
        .canonicalize()
        .map_err(|err| format!("failed to canonicalize source evidence path: {err}"))?;
    let resolved_output = if output_path.exists() {
        output_path
            .canonicalize()
            .map_err(|err| format!("failed to canonicalize {label} output path: {err}"))?
    } else {
        lexical_absolute_path(output_path)
            .map_err(|err| format!("failed to resolve {label} output path: {err}"))?
    };
    if resolved_output == canonical_source {
        return Err(format!(
            "{label} output cannot target the source evidence path {}",
            canonical_source.display()
        ));
    }
    Ok(())
}

pub(crate) fn lexical_absolute_path(path: &Path) -> Result<PathBuf, std::io::Error> {
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

fn reject_output_symlink_leaf(output_path: &Path, label: &str) -> Result<(), String> {
    match std::fs::symlink_metadata(output_path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(format!(
            "{label} output cannot be a symlink: {}",
            output_path.display()
        )),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "failed to inspect {label} output path {}: {err}",
            output_path.display()
        )),
    }
}
