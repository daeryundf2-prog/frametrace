//! Disk-space preflight for long-running output operations.
//!
//! Failing a 500 GB E01 export after an hour because the volume filled up
//! wastes examiner time and leaves a torn artifact; these helpers check
//! free space up front. Free-space numbers come from `fs2`
//! (`GetDiskFreeSpaceExW` on Windows, `statvfs` elsewhere), so the target's
//! own volume — not the process working directory — is what gets checked.

use std::path::{Path, PathBuf};

/// The closest ancestor of `target` that exists on disk (the target itself
/// may not exist yet, and free space must be queried on a real path).
fn nearest_existing_dir(target: &Path) -> Result<PathBuf, String> {
    for candidate in target.ancestors() {
        if candidate.exists() {
            return if candidate.is_file() {
                candidate
                    .parent()
                    .map(Path::to_path_buf)
                    .ok_or_else(|| format!("no parent directory for {}", target.display()))
            } else {
                Ok(candidate.to_path_buf())
            };
        }
    }
    Err(format!(
        "no existing ancestor for output path {}",
        target.display()
    ))
}

/// Bytes still free on the volume containing `target`'s nearest existing
/// ancestor.
pub fn available_bytes(target: &Path) -> Result<u64, String> {
    let dir = nearest_existing_dir(target)?;
    fs2::available_space(&dir)
        .map_err(|err| format!("failed to query free space on {}: {err}", dir.display()))
}

/// Returns a human-readable error unless the volume holding `target` has at
/// least `needed` bytes free. `context` names the operation for the message.
pub fn ensure_available(target: &Path, needed: u64, context: &str) -> Result<(), String> {
    let free = available_bytes(target)?;
    if free < needed {
        return Err(format!(
            "{context}: insufficient disk space on {} — need {} free but only {} available",
            nearest_existing_dir(target)?.display(),
            human_bytes(needed),
            human_bytes(free),
        ));
    }
    Ok(())
}

pub fn human_bytes(value: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = value as f64;
    let mut unit = 0usize;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_free_space_on_existing_and_missing_targets() {
        let dir = std::env::temp_dir();
        let free = available_bytes(&dir).unwrap();
        assert!(free > 0, "temp volume must report some free space");

        // A not-yet-created child resolves through the existing ancestor.
        let missing = dir
            .join(format!("ft-diskspace-{}", std::process::id()))
            .join("deep");
        let free_nested = available_bytes(&missing).unwrap();
        assert!(free_nested > 0);
    }

    #[test]
    fn ensure_available_rejects_impossible_request() {
        let dir = std::env::temp_dir();
        let err = ensure_available(&dir, u64::MAX, "test-op").unwrap_err();
        assert!(err.contains("insufficient disk space"), "{err}");
        assert!(err.contains("test-op"), "{err}");
        // A tiny request on the same volume must pass.
        ensure_available(&dir, 1, "test-op").unwrap();
    }

    #[test]
    fn formats_bytes_for_error_messages() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1024), "1.0 KB");
        assert_eq!(human_bytes(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(human_bytes(3 * 1024 * 1024 * 1024), "3.0 GB");
    }
}
