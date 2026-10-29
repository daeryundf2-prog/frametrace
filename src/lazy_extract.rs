//! Live (allocated) files of a filesystem inspection, read out of the
//! image only when they are needed.
//!
//! The E01 pipeline lists the card with fls and indexes every live video
//! under the path its extraction gets (`artifacts/extracted/<card path>`)
//! without copying anything, so the viewer opens as soon as the listing is
//! done. The first time a clip is played, exported or validated,
//! [`ensure_materialized`] copies just that inode out with icat — the same
//! audited, hashed recovery a bulk extraction runs.

use crate::selection::SelectionItem;
use crate::tsk::{self, TskRecoverOptions};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// `<case>/artifacts/extracted`, spelled with native separators so record
/// paths built on it compare equal to scanned ones.
pub fn extracted_root(case_dir: &Path) -> PathBuf {
    case_dir.join("artifacts").join("extracted")
}

/// Relative path under `artifacts/extracted` for an allocated file's
/// filesystem path: the card's folder and file names as they are (Korean,
/// spaces, the extension), with only Windows-illegal characters replaced
/// and device names / names shaped like our `.part` files prefixed
/// ([`crate::cli::handlers::forensic::clean_relative_path`]).
pub(crate) fn extracted_relative_path(raw: &str) -> PathBuf {
    crate::cli::handlers::forensic::clean_relative_path(raw)
}

/// The pre-fix spelling ([`crate::util::sanitize_filename`] per component:
/// `clip.avi` → `clip_avi`, Korean → `_`). Only used to recognise files an
/// older build already extracted into an existing case.
pub(crate) fn legacy_extracted_relative_path(raw: &str) -> PathBuf {
    relative_path_with(raw, crate::util::sanitize_filename)
}

fn relative_path_with(raw: &str, clean_part: fn(&str) -> String) -> PathBuf {
    let raw = raw.trim().trim_end_matches("(deleted)").trim();
    let norm = raw.replace('\\', "/");
    let mut path = PathBuf::new();
    for part in norm.trim_start_matches('/').split('/') {
        let clean = clean_part(part);
        if !clean.is_empty() && clean != "." && clean != ".." {
            path.push(clean);
        }
    }
    if path.as_os_str().is_empty() {
        PathBuf::from("unnamed.bin")
    } else {
        path
    }
}

/// Case- and separator-insensitive key: Windows (and the FAT/exFAT card
/// itself) treat `A.AVI` and `a.avi` as one file, and a case-relative
/// `artifacts/…` path joined onto a native case folder mixes separators.
pub(crate) fn path_key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_lowercase()
}

/// `dir/name.ext` → `dir/name~<tag>.ext`.
fn with_name_tag(path: &Path, tag: &str) -> PathBuf {
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_default();
    let name = match path.extension() {
        Some(ext) => format!("{stem}~{tag}.{}", ext.to_string_lossy()),
        None => format!("{stem}~{tag}"),
    };
    path.with_file_name(name)
}

/// One extraction target per selection item (`None` for a recover item),
/// guaranteed distinct: two inodes whose names map to the same path (names
/// differing only in a replaced character or in case) get an `~<inode>`
/// tag instead of the second one being skipped as "already exists" — that
/// skip silently dropped a live video from the review.
pub(crate) fn plan_extract_targets(
    selection: &[SelectionItem],
    is_extract: bool,
) -> Vec<Option<PathBuf>> {
    let inode_of = |item: &SelectionItem| {
        let selector = item.selector.trim();
        selector
            .strip_prefix("fls:")
            .unwrap_or(selector)
            .to_string()
    };
    let base: Vec<Option<PathBuf>> = selection
        .iter()
        .map(|item| {
            (is_extract || item.action.as_deref() == Some("extract")).then(|| {
                let inode = inode_of(item);
                extracted_relative_path(item.notes.as_deref().unwrap_or(&inode))
            })
        })
        .collect();
    let mut counts = HashMap::new();
    for path in base.iter().flatten() {
        *counts.entry(path_key(path)).or_insert(0usize) += 1;
    }
    let mut used = HashSet::new();
    base.into_iter()
        .zip(selection)
        .map(|(path, item)| {
            let path = path?;
            let mut target = if counts[&path_key(&path)] > 1 {
                with_name_tag(&path, &crate::util::sanitize_filename(&inode_of(item)))
            } else {
                path
            };
            let mut n = 2;
            while !used.insert(path_key(&target)) {
                target = with_name_tag(&target, &n.to_string());
                n += 1;
            }
            Some(target)
        })
        .collect()
}

/// A file an older build extracted for this item under the legacy name,
/// reused as is so re-running an existing case neither duplicates it nor
/// orphans the marks and logs that point at it. Only when the legacy name
/// was unambiguous (no other item mapped onto it).
pub(crate) fn legacy_extracted_file(
    extracted_root: &Path,
    note: Option<&str>,
    legacy_counts: &HashMap<String, usize>,
) -> Option<PathBuf> {
    let legacy = legacy_extracted_relative_path(note?);
    if legacy_counts.get(&path_key(&legacy)).copied() != Some(1) {
        return None;
    }
    let path = extracted_root.join(legacy);
    (fs::metadata(&path).is_ok_and(|meta| meta.is_file() && meta.len() > 0)).then_some(path)
}

/// Newest `db/filesystem/tsk-files-*.jsonl` inspection output, or None
/// when no inspect-image/inspect-e01 --filesystem run exists yet. The
/// unix-timestamp suffix orders runs chronologically.
pub(crate) fn latest_entries_jsonl(case_dir: &Path) -> Option<PathBuf> {
    newest_in_filesystem_dir(case_dir, "tsk-files-", ".jsonl")
}

fn newest_in_filesystem_dir(case_dir: &Path, prefix: &str, suffix: &str) -> Option<PathBuf> {
    fs::read_dir(case_dir.join("db/filesystem"))
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(prefix) && name.ends_with(suffix))
        })
        .max()
}

/// Allocated (live) video-candidate selectors from an inspect-image entries
/// file, the inode as selector and the filesystem path as note.
pub(crate) fn allocated_video_selectors(
    entries_jsonl: &Path,
) -> Result<Vec<SelectionItem>, String> {
    Ok(read_entries(entries_jsonl)?
        .into_iter()
        .map(|entry| entry.item)
        .collect())
}

/// `fls -l` sizes of the live videos in an entries file, by inode.
pub(crate) fn listed_sizes(entries_jsonl: &Path) -> Result<HashMap<String, u64>, String> {
    Ok(read_entries(entries_jsonl)?
        .into_iter()
        .filter_map(|entry| Some((entry.item.selector, entry.size_bytes?)))
        .collect())
}

/// One live video row of an entries file.
struct AllocatedEntry {
    item: SelectionItem,
    size_bytes: Option<u64>,
}

fn read_entries(entries_jsonl: &Path) -> Result<Vec<AllocatedEntry>, String> {
    let text = fs::read_to_string(entries_jsonl)
        .map_err(|err| format!("failed to read {}: {err}", entries_jsonl.display()))?;
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    for (line_no, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(trimmed).map_err(|err| {
            format!(
                "invalid entries line {}:{}: {err}",
                entries_jsonl.display(),
                line_no + 1
            )
        })?;
        let deleted = value.get("deleted").and_then(|v| v.as_bool()) == Some(true);
        let video = value.get("video_candidate").and_then(|v| v.as_bool()) == Some(true);
        let Some(inode) = value.get("inode").and_then(|v| v.as_str()) else {
            continue;
        };
        if !deleted && video && !inode.is_empty() && seen.insert(inode.to_string()) {
            entries.push(AllocatedEntry {
                item: SelectionItem {
                    selector: inode.to_string(),
                    kind: Some("filesystem".to_string()),
                    action: Some("extract".to_string()),
                    format: None,
                    time_seconds: None,
                    notes: value
                        .get("path")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                },
                size_bytes: value.get("size_bytes").and_then(|v| v.as_u64()),
            });
        }
    }
    Ok(entries)
}

/// One live video of the newest inspection and where it is (or will be)
/// under `artifacts/extracted`.
pub struct ListedFile {
    pub inode: String,
    pub card_path: String,
    pub relative: PathBuf,
    /// From `fls -l`; `None` for an inspection made before sizes were listed.
    pub size_bytes: Option<u64>,
}

/// The newest inspection's image and its live videos.
pub struct AllocatedListing {
    pub image_path: PathBuf,
    pub partition_offset: u64,
    pub files: Vec<ListedFile>,
}

/// The newest inspection's live videos with their planned paths (the same
/// plan `recover-batch --allocated-videos` extracts to). `Ok(None)` when
/// the case has no filesystem inspection.
pub fn allocated_listing(case_dir: &Path) -> Result<Option<AllocatedListing>, String> {
    let Some(summary_path) = newest_in_filesystem_dir(case_dir, "tsk-inspection-", ".json") else {
        return Ok(None);
    };
    let Some(entries_path) = latest_entries_jsonl(case_dir) else {
        return Ok(None);
    };
    let summary: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&summary_path)
            .map_err(|err| format!("failed to read {}: {err}", summary_path.display()))?,
    )
    .map_err(|err| {
        format!(
            "invalid inspection summary {}: {err}",
            summary_path.display()
        )
    })?;
    let image_path = summary
        .get("image_path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .ok_or_else(|| format!("{} names no image_path", summary_path.display()))?;
    let partition_offset = summary
        .get("partition_offset")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| format!("{} names no partition_offset", summary_path.display()))?;
    let entries = read_entries(&entries_path)?;
    let items: Vec<SelectionItem> = entries.iter().map(|entry| entry.item.clone()).collect();
    let targets = plan_extract_targets(&items, true);
    // Merge review M1 for the listing: a file already at a planned path is
    // this listing's only when the tsk audit proves it came from this image,
    // partition and inode — otherwise (a reused case, another card) the
    // entry moves to the `__ino<inode>` name recover-batch would write, so
    // the review never shows another source's bytes under this record.
    let root = extracted_root(case_dir);
    let image_identity = crate::util::canonicalize_display(&image_path)
        .unwrap_or_else(|_| image_path.clone())
        .to_string_lossy()
        .to_string();
    let mut proofs = None;
    let files = entries
        .into_iter()
        .zip(targets)
        .filter_map(|(entry, target)| {
            let planned = target?;
            let base = root.join(&planned);
            let relative = if fs::symlink_metadata(&base).is_err() {
                planned
            } else {
                let proofs = proofs.get_or_insert_with(|| {
                    crate::cli::handlers::forensic::inode_output_records_by_path(case_dir)
                });
                let inode = entry.item.selector.as_str();
                let chosen = match crate::cli::handlers::forensic::choose_output_path_by(
                    &base,
                    &format!("__ino{}", crate::util::sanitize_filename(inode)),
                    |candidate| {
                        crate::cli::handlers::forensic::proven_own_output_in(
                            candidate,
                            &image_identity,
                            partition_offset,
                            inode,
                            proofs,
                        )
                    },
                ) {
                    crate::cli::handlers::forensic::OutputChoice::Same(path)
                    | crate::cli::handlers::forensic::OutputChoice::Fresh(path) => path,
                };
                chosen
                    .strip_prefix(&root)
                    .map(Path::to_path_buf)
                    .unwrap_or(planned)
            };
            Some(ListedFile {
                inode: entry.item.selector,
                card_path: entry.item.notes.unwrap_or_default(),
                relative,
                size_bytes: entry.size_bytes,
            })
        })
        .collect();
    Ok(Some(AllocatedListing {
        image_path,
        partition_offset,
        files,
    }))
}

/// Makes `path` exist when it is a live file this case listed but has not
/// read yet: copies that one inode out of the image (audited and hashed in
/// `evidence/logs/tsk-audit.jsonl`, like any icat recovery). `Ok(true)`
/// when the file exists afterwards; `Ok(false)` when `path` is not such a
/// file, so the caller reports it missing as before.
///
/// Concurrent callers (several Range requests of one video, the server and
/// a CLI export child) are serialized by a lock file per target, so one
/// copy runs and the rest find the published file.
pub fn ensure_materialized(case_dir: &Path, path: &Path) -> Result<bool, String> {
    if path.is_file() {
        return Ok(true);
    }
    // A deleted-video candidate (artifacts/Recovery/…) of the triage scan.
    if crate::recovery_scan::ensure_candidate(case_dir, path)? {
        return Ok(true);
    }
    let wanted = path_key(&crate::util::strip_windows_extended_prefix(path));
    let roots: Vec<PathBuf> = [
        Some(case_dir.to_path_buf()),
        crate::util::canonicalize_display(case_dir).ok(),
    ]
    .into_iter()
    .flatten()
    .map(|dir| extracted_root(&dir))
    .collect();
    if !roots.iter().any(|root| wanted.starts_with(&path_key(root))) {
        return Ok(false);
    }
    let Some(listing) = allocated_listing(case_dir)? else {
        return Ok(false);
    };
    let Some((target, file)) = listing.files.iter().find_map(|file| {
        roots
            .iter()
            .map(|root| root.join(&file.relative))
            .find(|candidate| path_key(candidate) == wanted)
            .map(|candidate| (candidate, file))
    }) else {
        return Ok(false);
    };
    // Keyed by inode, not by the requested spelling: recover-batch's workers
    // take the same lock for the same inode.
    let _lock = lock_target(
        case_dir,
        &inode_lock_key(listing.partition_offset, &file.inode),
    )?;
    if target.is_file() {
        return Ok(true);
    }
    let context = tsk::RecoverContext::new(&listing.image_path, "icat")?;
    let options = TskRecoverOptions {
        partition_offset: listing.partition_offset,
        inode: file.inode.clone(),
        output_path: Some(target.clone()),
        recover_deleted: false,
        include_slack: false,
        skip_sparse_holes: true,
        icat_bin: "icat".to_string(),
        timeout_secs: None,
        // A live-file copy is an extraction, not a recovery (merge review M2).
        extract: true,
    };
    match tsk::recover_inode_in(case_dir, &context, &options, true) {
        Ok(_) => Ok(true),
        Err(err) if target.is_file() => {
            eprintln!("warning: {err} (the file was read meanwhile)");
            Ok(true)
        }
        Err(err) => Err(format!("{} ({}): {err}", file.card_path, file.inode)),
    }
}

/// The [`lock_target`] key of one inode's read-out (`partition offset` +
/// inode), shared by playback/export reads and recover-batch.
pub(crate) fn inode_lock_key(partition_offset: u64, inode: &str) -> String {
    format!("inode:{partition_offset}:{}", inode.trim())
}

/// Exclusive lock on `db/extract-locks/<hash of key>.lock`, held until the
/// returned file is dropped. Blocks while another thread or process reads
/// the same target.
pub(crate) fn lock_target(case_dir: &Path, key: &str) -> Result<fs::File, String> {
    use fs2::FileExt;
    let dir = case_dir.join("db/extract-locks");
    fs::create_dir_all(&dir).map_err(|err| format!("failed to create {}: {err}", dir.display()))?;
    let digest = crate::sha256::digest_bytes(key.as_bytes());
    let path = dir.join(format!("{}.lock", &digest[..24]));
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(|err| format!("failed to open {}: {err}", path.display()))?;
    file.lock_exclusive()
        .map_err(|err| format!("failed to lock {}: {err}", path.display()))?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_inspection(case: &Path, entries: &str) {
        let dir = case.join("db/filesystem");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("tsk-inspection-100.json"),
            r#"{"image_path":"C:\\img\\card.E01","partition_offset":2048}"#,
        )
        .unwrap();
        fs::write(dir.join("tsk-files-100.jsonl"), entries).unwrap();
    }

    #[test]
    fn listing_plans_live_videos_only_with_sizes() {
        let case = std::env::temp_dir().join(format!("ft-lazy-listing-{}", std::process::id()));
        let _ = fs::remove_dir_all(&case);
        write_inspection(
            &case,
            concat!(
                r#"{"inode":"519","path":"EVENT/a.avi","deleted":false,"video_candidate":true,"size_bytes":3016}"#,
                "\n",
                r#"{"inode":"525","path":"EVENT/b.avi","deleted":true,"video_candidate":true,"size_bytes":10}"#,
                "\n",
                r#"{"inode":"600","path":"USER/x.txt","deleted":false,"video_candidate":false}"#,
                "\n",
                r#"{"inode":"1030","path":"상시/정상.avi","deleted":false,"video_candidate":true}"#,
                "\n"
            ),
        );
        let listing = allocated_listing(&case).unwrap().unwrap();
        assert_eq!(listing.partition_offset, 2048);
        assert_eq!(listing.image_path, PathBuf::from("C:\\img\\card.E01"));
        let rows: Vec<(String, String, Option<u64>)> = listing
            .files
            .iter()
            .map(|f| {
                (
                    f.inode.clone(),
                    f.relative.to_string_lossy().replace('\\', "/"),
                    f.size_bytes,
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                ("519".to_string(), "EVENT/a.avi".to_string(), Some(3016)),
                ("1030".to_string(), "상시/정상.avi".to_string(), None),
            ]
        );
        // Not a listed file: left to the caller, nothing is read.
        assert!(!ensure_materialized(&case, &case.join("artifacts/other/x.avi")).unwrap());
        assert!(!ensure_materialized(&case, &extracted_root(&case).join("EVENT/zzz.avi")).unwrap());
        let _ = fs::remove_dir_all(&case);
    }

    /// Merge review M1 for the lazy index: a file already at a planned path
    /// stays this listing's only when the tsk audit proves it came from this
    /// image, partition and inode; another source's file moves the entry to
    /// the `__ino<inode>` name recover-batch writes.
    #[test]
    fn listing_never_claims_another_sources_file() {
        let case = std::env::temp_dir().join(format!("ft-lazy-m1-{}", std::process::id()));
        let _ = fs::remove_dir_all(&case);
        write_inspection(
            &case,
            concat!(
                r#"{"inode":"519","path":"EVENT/a.avi","deleted":false,"video_candidate":true,"size_bytes":5}"#,
                "\n"
            ),
        );
        let existing = extracted_root(&case).join("EVENT").join("a.avi");
        fs::create_dir_all(existing.parent().unwrap()).unwrap();
        fs::write(&existing, b"other").unwrap();
        let relative = |case: &Path| {
            allocated_listing(case).unwrap().unwrap().files[0]
                .relative
                .to_string_lossy()
                .replace('\\', "/")
        };
        // No audit record proves it: another card's copy, never this entry.
        assert_eq!(relative(&case), "EVENT/a__ino519.avi");

        // The audit proves it is this image's inode 519: the entry keeps it.
        let lane = case.join("evidence/logs/tsk-audit.jsonl");
        fs::create_dir_all(lane.parent().unwrap()).unwrap();
        let row = serde_json::json!({
            "schema_version": 1,
            "event": "extract-inode",
            "image_path": "C:\\img\\card.E01",
            "partition_offset": 2048,
            "inode": "519",
            "output_path": crate::util::canonicalize_display(&existing).unwrap(),
            "size_bytes": 5,
        });
        crate::audit::append_chained_jsonl(&lane, &row.to_string()).unwrap();
        assert_eq!(relative(&case), "EVENT/a.avi");
        let _ = fs::remove_dir_all(&case);
    }

    #[test]
    fn a_case_without_inspection_has_no_listing() {
        let case = std::env::temp_dir().join(format!("ft-lazy-none-{}", std::process::id()));
        let _ = fs::remove_dir_all(&case);
        fs::create_dir_all(&case).unwrap();
        assert!(allocated_listing(&case).unwrap().is_none());
        assert!(!ensure_materialized(&case, &extracted_root(&case).join("a.avi")).unwrap());
        let _ = fs::remove_dir_all(&case);
    }
}
