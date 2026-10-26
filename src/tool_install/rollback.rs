//! Fix round 13: what an incomplete rollback leaves behind, kept where no
//! sweep removes it and described so a person can finish the job.
//!
//! * `tools/rollback-backup-<hex>/` — an archive tool's previous files
//!   (`0.bak`, `1.bak`, … as `move_all_or_nothing` named them) and
//!   `RESTORE.txt`, one line per file: `<original path> <- <name here>`.
//! * `tools/.staging-<hex>/backup/RESTORE.txt` — the same, when the backup
//!   could not even be moved out of the staging dir (the sweep skips it).
//! * `tools/python/rollback-backup-<hex>/<tool>/` — a pip tool's previous
//!   venv, whole, with `RESTORE.txt` next to it.
//!
//! * Fix round 16 M1: `tools/.staging-<hex>/backup/MOVING` — the move
//!   journal of an install killed while it moved files; the sweep turns
//!   it into a `tools/rollback-backup-<hex>/` (RESTORE.txt made from it).
//!
//! The first line of `RESTORE.txt` is `tool: <name>`; [`rollback_backups`]
//! lists them for `/api/tools`, `frametrace tools list` and `doctor`.

use super::*;

/// The restore instructions next to the kept previous files.
pub const RESTORE_FILE: &str = "RESTORE.txt";

/// Name prefix of the kept-backup directories (never swept).
pub(crate) const ROLLBACK_BACKUP_PREFIX: &str = "rollback-backup-";

const TOOL_LINE: &str = "tool: ";

/// `RESTORE.txt`: an optional `tool:` line, how to restore, then one line
/// per file `<original> <- <kept copy, relative to folder>` (or a NEW file
/// with no previous copy, to delete).
pub(crate) fn restore_text(
    tool: Option<&str>,
    entries: &[(PathBuf, Option<PathBuf>)],
    folder: &Path,
) -> String {
    let mut text = String::new();
    if let Some(tool) = tool {
        text.push_str(&format!("{TOOL_LINE}{tool}\n"));
    }
    text.push_str(
        "FrameTrace could not finish undoing a failed install; the previous copies are kept in this folder.\n\
         To restore by hand: close FrameTrace (and anything running the tool); for each line below, \
         move the item named after \"<-\" from this folder to the path before it, replacing what is there \
         (a line marked NEW has no previous copy: delete that path); then delete this folder.\n",
    );
    for (original, kept) in entries {
        match kept {
            Some(kept) => {
                let shown = kept.strip_prefix(folder).unwrap_or(kept);
                text.push_str(&format!("{} <- {}\n", original.display(), shown.display()));
            }
            None => text.push_str(&format!(
                "{} <- (none: NEW file with no previous copy — delete it)\n",
                original.display()
            )),
        }
    }
    text
}

/// Puts `tool: <name>` first in `RESTORE.txt` (written by
/// `move_all_or_nothing`, which does not know the tool). Fix round 14 L4:
/// when that best-effort write left no file, it is created here (the
/// `tool:` line and how to restore; the file list is then only in the
/// error, which the install audit log keeps). `Err` when it still cannot
/// be written — the caller says so instead of pointing at the file.
pub(crate) fn name_tool_in_restore_file(path: &Path, tool: &str) -> Result<(), String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) if text.starts_with(TOOL_LINE) => return Ok(()),
        Ok(text) => format!("{TOOL_LINE}{tool}\n{text}"),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let folder = path.parent().unwrap_or(path);
            format!(
                "{}{MISSING_LIST_NOTE}",
                restore_text(Some(tool), &[], folder)
            )
        }
        Err(err) => return Err(err.to_string()),
    };
    std::fs::write(path, text).map_err(|err| err.to_string())
}

/// The last line of a `RESTORE.txt` created without its file list.
pub(crate) const MISSING_LIST_NOTE: &str = "(The list of files could not be written here: the install error — also in tools/install-audit.jsonl — names each file and the name of its previous copy in this folder.)\n";

/// The note added to an incomplete-rollback error when `RESTORE.txt`
/// could not be written (Fix round 14 L4).
pub(crate) fn restore_unwritten_note(err: &str) -> String {
    format!("({RESTORE_FILE} could not be written: {err})")
}

fn restore_file_tool(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let tool = text.lines().next()?.strip_prefix(TOOL_LINE)?.trim();
    (!tool.is_empty()).then(|| tool.to_string())
}

/// `std::fs::write` of a `RESTORE.txt`; a test can make it fail
/// (Fix round 14 L4).
fn write_restore_file(path: &Path, text: &str) -> std::io::Result<()> {
    #[cfg(test)]
    if FAIL_RESTORE_WRITE.with(|fail| fail.get()) {
        return Err(std::io::Error::other("test: RESTORE.txt write fails"));
    }
    std::fs::write(path, text)
}

#[cfg(test)]
thread_local! {
    pub(crate) static FAIL_RESTORE_WRITE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// One kept backup of an incomplete rollback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackBackup {
    /// The directory holding the previous copies.
    pub path: PathBuf,
    /// Its restore instructions (may be missing: a round-12 leftover).
    pub restore_file: PathBuf,
    /// The tool it belongs to, when known.
    pub tool: Option<String>,
    pub size_bytes: u64,
}

fn dir_size(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.file_type() {
            Ok(kind) if kind.is_symlink() => 0,
            Ok(kind) if kind.is_dir() => dir_size(&entry.path()),
            Ok(_) => entry.metadata().map(|meta| meta.len()).unwrap_or(0),
            Err(_) => 0,
        })
        .sum()
}

fn real_dirs(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| {
            entry
                .file_type()
                .is_ok_and(|kind| kind.is_dir() && !kind.is_symlink())
        })
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                entry.path(),
            )
        })
        .collect()
}

/// Every kept backup under `tools_dir` (see the module doc).
pub fn rollback_backups(tools_dir: &Path) -> Vec<RollbackBackup> {
    // `tools/` first, then `tools/python/`, each sorted by path.
    let mut top = Vec::new();
    let mut python = Vec::new();
    for (name, path) in real_dirs(tools_dir) {
        let restore_file = if name.starts_with(ROLLBACK_BACKUP_PREFIX) {
            path.join(RESTORE_FILE)
        } else if name.starts_with(".staging-")
            && std::fs::symlink_metadata(path.join("backup").join(RESTORE_FILE)).is_ok()
        {
            path.join("backup").join(RESTORE_FILE)
        } else if name.starts_with(".staging-")
            && let Some(journal) = read_journal(&path.join("backup"))
            && !journal_restore_entries(&journal, &path.join("backup")).is_empty()
        {
            // Fix round 16 M1: an install killed while moving files, not
            // yet rescued by a sweep (doctor runs none) — listed already.
            top.push(RollbackBackup {
                tool: journal.tool,
                size_bytes: dir_size(&path),
                restore_file: path.join("backup").join(RESTORE_FILE),
                path,
            });
            continue;
        } else {
            continue;
        };
        top.push(RollbackBackup {
            tool: restore_file_tool(&restore_file),
            size_bytes: dir_size(&path),
            path,
            restore_file,
        });
    }
    for (name, path) in real_dirs(&tools_dir.join("python")) {
        if !name.starts_with(ROLLBACK_BACKUP_PREFIX) {
            continue;
        }
        let restore_file = path.join(RESTORE_FILE);
        let tool = restore_file_tool(&restore_file)
            .or_else(|| real_dirs(&path).into_iter().next().map(|(name, _)| name));
        python.push(RollbackBackup {
            tool,
            size_bytes: dir_size(&path),
            path,
            restore_file,
        });
    }
    top.sort_by(|a, b| a.path.cmp(&b.path));
    python.sort_by(|a, b| a.path.cmp(&b.path));
    top.extend(python);
    top
}

/// [`rollback_backups`] of this executable's `tools` dir (none when it
/// cannot be found).
pub fn rollback_backups_for_current_exe() -> Vec<RollbackBackup> {
    InstallDirs::for_current_exe()
        .map(|dirs| rollback_backups(&dirs.tools_dir))
        .unwrap_or_default()
}

/// The warning shown for a kept backup (UI, `tools list`, `doctor`).
pub fn rollback_hint(backup: &RollbackBackup) -> String {
    let size = backup.size_bytes as f64 / (1024.0 * 1024.0);
    format!(
        "이전 설치 되돌리기가 끝나지 않아 이전 파일이 {} ({size:.1} MB)에 남아 있습니다 — 도구 파일이 새것·옛것으로 섞였을 수 있습니다. {}의 안내대로 직접 복원하거나 도구를 다시 설치한 뒤 이 폴더를 지우십시오",
        backup.path.display(),
        backup.restore_file.display()
    )
}

/// Fix round 13 M: undoes a failed pip venv rebuild — removes the new,
/// partial `target` and renames `backup` (the previous venv) back, both
/// with the bounded retry. When that cannot be finished the previous
/// venv is moved to `python_root/rollback-backup-<hex>/<tool>` (no sweep
/// removes it) with `RESTORE.txt`, and the returned error starts with
/// [`ROLLBACK_INCOMPLETE_PREFIX`]. When even that move fails the backup
/// stays as `.<tool>.old-<hex>`, which the sweep keeps while `target` is
/// not a working venv.
pub(crate) fn undo_venv(
    err: String,
    tool: &str,
    target: &Path,
    backup: Option<&Path>,
    python_root: &Path,
) -> String {
    let removed = remove_dir_all_with_retry(target);
    let Some(backup) = backup else {
        return match removed {
            Ok(()) => err,
            Err(remove_err) => format!(
                "{err} (the partial venv {} could not be removed: {remove_err}; delete it by hand)",
                target.display()
            ),
        };
    };
    let problem = match removed {
        Err(remove_err) => format!(
            "{} is the NEW, partial venv (cannot remove it: {remove_err})",
            target.display()
        ),
        Ok(()) => match rename_with_retry(backup, target) {
            Ok(()) => return err,
            Err(restore_err) => format!(
                "{} is missing (cannot restore it: {restore_err})",
                target.display()
            ),
        },
    };
    let kept_dir = python_root.join(format!("{ROLLBACK_BACKUP_PREFIX}{}", random_hex()));
    let kept = kept_dir.join(tool);
    let moved = std::fs::create_dir(&kept_dir).and_then(|()| rename_with_retry(backup, &kept));
    let where_kept = match moved {
        Ok(()) => {
            let restore = kept_dir.join(RESTORE_FILE);
            let written = write_restore_file(
                &restore,
                &restore_text(
                    Some(tool),
                    &[(target.to_path_buf(), Some(kept.clone()))],
                    &kept_dir,
                ),
            );
            // Fix round 14 L4: never point at a file that is not there.
            let how = match written {
                Ok(()) => format!("{} says how", restore.display()),
                Err(err) => format!(
                    "move it back to {} {}",
                    target.display(),
                    restore_unwritten_note(&err.to_string())
                ),
            };
            format!(
                "the previous venv is kept at {} (restore it by hand — {how})",
                kept.display(),
            )
        }
        Err(move_err) => {
            let _ = std::fs::remove_dir(&kept_dir);
            format!(
                "the previous venv is kept at {} — could not move it to {}: {move_err}; it is not removed while {} is not a working venv; restore it by hand before installing again",
                backup.display(),
                kept.display(),
                target.display()
            )
        }
    };
    format!(
        "{ROLLBACK_INCOMPLETE_PREFIX}{err}; the rollback could NOT restore the previous venv — {problem}; {where_kept}"
    )
}

// ------------------------------------------------------- move journal (M1)

/// Fix round 16 M1: the intent journal `move_all_or_nothing` writes into
/// `backup/` before its first move — `tool: <name>`, then one line per
/// planned move `plan\t<index>\t<existed|new>\t<destination>` (the
/// previous copy of an `existed` destination is `<index>.bak` beside it).
/// It is removed when the moves succeed and when a rollback completes, so
/// a journal left in a stale `.staging-*/backup/` means the install was
/// killed (Ctrl+C, console closed, power lost) while files were moving:
/// the sweep then keeps the previous copies ([`rescue_interrupted_move`])
/// instead of deleting them.
pub(crate) const MOVE_JOURNAL: &str = "MOVING";

/// Last line appended when a successful install could not remove its
/// journal: the sweep then treats the folder as an ordinary leftover.
const JOURNAL_COMPLETE: &str = "complete";

/// One planned move read back from a journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JournalMove {
    pub(crate) index: usize,
    pub(crate) existed: bool,
    pub(crate) dest: PathBuf,
}

/// A parsed journal ([`read_journal`] gives `None` when there is none or
/// it says the moves completed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Journal {
    pub(crate) tool: Option<String>,
    pub(crate) moves: Vec<JournalMove>,
}

#[cfg(test)]
thread_local! {
    /// Tests: makes [`write_journal`] fail on this thread.
    pub(crate) static FAIL_JOURNAL_WRITE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Writes (and flushes to disk) the journal for `moves` into `backup_dir`
/// before anything moves. `Err` refuses the install: without the journal
/// a kill during the moves could lose the previous files.
pub(crate) fn write_journal(
    backup_dir: &Path,
    tool: &str,
    moves: &[(PathBuf, PathBuf)],
) -> Result<(), String> {
    use std::io::Write as _;
    let path = backup_dir.join(MOVE_JOURNAL);
    let mut text = format!("{TOOL_LINE}{tool}\n");
    for (index, (_, dest)) in moves.iter().enumerate() {
        let existed = std::fs::symlink_metadata(dest).is_ok();
        text.push_str(&format!(
            "plan\t{index}\t{}\t{}\n",
            if existed { "existed" } else { "new" },
            dest.display()
        ));
    }
    let write = || -> std::io::Result<()> {
        #[cfg(test)]
        if FAIL_JOURNAL_WRITE.with(|fail| fail.get()) {
            return Err(std::io::Error::other("test: journal write fails"));
        }
        let mut file = std::fs::File::create(&path)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()
    };
    write().map_err(|err| {
        let _ = std::fs::remove_file(&path);
        format!(
            "cannot write the install journal {} (nothing was replaced): {err}",
            path.display()
        )
    })
}

/// Removes the journal once nothing needs restoring (all moves done, or a
/// rollback that completed). When it cannot be removed after a success it
/// is marked complete so the sweep does not mistake the folder for an
/// interrupted move.
pub(crate) fn close_journal(backup_dir: &Path, success: bool) {
    use std::io::Write as _;
    let path = backup_dir.join(MOVE_JOURNAL);
    if remove_file_with_retry(&path).is_ok() || !success {
        return;
    }
    if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(&path) {
        let _ = file.write_all(format!("{JOURNAL_COMPLETE}\n").as_bytes());
        let _ = file.sync_all();
    }
}

/// The journal in `backup_dir`, unless missing or marked complete.
pub(crate) fn read_journal(backup_dir: &Path) -> Option<Journal> {
    let text = std::fs::read_to_string(backup_dir.join(MOVE_JOURNAL)).ok()?;
    let mut journal = Journal {
        tool: None,
        moves: Vec::new(),
    };
    for line in text.lines() {
        if line == JOURNAL_COMPLETE {
            return None;
        }
        if let Some(tool) = line.strip_prefix(TOOL_LINE) {
            let tool = tool.trim();
            journal.tool = (!tool.is_empty()).then(|| tool.to_string());
            continue;
        }
        let mut parts = line.splitn(4, '\t');
        if let (Some("plan"), Some(index), Some(kind), Some(dest)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
            && let Ok(index) = index.parse::<usize>()
            && matches!(kind, "existed" | "new")
            && !dest.is_empty()
        {
            journal.moves.push(JournalMove {
                index,
                existed: kind == "existed",
                dest: PathBuf::from(dest),
            });
        }
    }
    Some(journal)
}

/// What a person must still undo after an interrupted move, in
/// `RESTORE.txt` form: an `existed` destination whose `<index>.bak` is
/// still in `backup_dir` gets that copy back; a `new` destination that
/// now exists was put there by the killed install (the install lock keeps
/// anything else from writing there meanwhile) — delete it. Everything
/// else was not touched or was already restored. Empty: nothing to do.
pub(crate) fn journal_restore_entries(
    journal: &Journal,
    backup_dir: &Path,
) -> Vec<(PathBuf, Option<PathBuf>)> {
    journal
        .moves
        .iter()
        .filter_map(|planned| {
            if planned.existed {
                let backup = backup_dir.join(format!("{}.bak", planned.index));
                std::fs::symlink_metadata(&backup)
                    .is_ok()
                    .then(|| (planned.dest.clone(), Some(backup)))
            } else {
                std::fs::symlink_metadata(&planned.dest)
                    .is_ok()
                    .then(|| (planned.dest.clone(), None))
            }
        })
        .collect()
}

/// The note under the `tool:` line of a `RESTORE.txt` made from an
/// interrupted move's journal.
pub(crate) const INTERRUPTED_NOTE: &str = "The install was interrupted (FrameTrace was closed or the PC stopped) while it moved the new files into place: some files may be new and some old.\n";

/// Whether the `staging` dir's `backup/` holds an active journal (its
/// previous files must be rescued, never swept).
pub(crate) fn staging_has_active_journal(staging: &Path) -> bool {
    read_journal(&staging.join("backup")).is_some()
}

/// Fix round 16 M1: a stale `staging` dir whose `backup/` holds an active
/// journal (an install killed while moving files). Writes `RESTORE.txt`
/// from the journal (tool named) into `backup/` and moves `backup/` to
/// `tools/rollback-backup-<hex>`, so `status()`, `doctor` and the UI show
/// `rollback_incomplete`. When nothing needs restoring the journal is
/// removed and the folder is left to the ordinary sweep. Afterwards the
/// sweep's own checks decide: a staging dir still holding `backup/` with
/// `RESTORE.txt` or an active journal is never removed.
pub(crate) fn rescue_interrupted_move(tools_dir: &Path, staging: &Path) {
    let backup_dir = staging.join("backup");
    let Some(journal) = read_journal(&backup_dir) else {
        return;
    };
    let entries = journal_restore_entries(&journal, &backup_dir);
    if entries.is_empty() {
        close_journal(&backup_dir, false);
        return;
    }
    let mut text = restore_text(journal.tool.as_deref(), &entries, &backup_dir);
    let at = if journal.tool.is_some() {
        text.find('\n').map_or(0, |at| at + 1)
    } else {
        0
    };
    text.insert_str(at, INTERRUPTED_NOTE);
    if write_restore_file(&backup_dir.join(RESTORE_FILE), &text).is_err() {
        // The journal still keeps the sweep away; retried next time.
        return;
    }
    let kept = tools_dir.join(format!("{ROLLBACK_BACKUP_PREFIX}{}", random_hex()));
    // When this fails `backup/RESTORE.txt` keeps the staging dir instead.
    let _ = rename_with_retry(&backup_dir, &kept);
}
