# Filesystem Recovery Workflow

FrameTrace can now use external Sleuth Kit command-line tools for first-pass file-system-aware inspection and inode recovery from raw images.

This is separate from signature carving:

- `carve-file` scans byte ranges for contiguous video signatures.
- `inspect-image` asks Sleuth Kit to list active and deleted directory entries from a file system.
- `recover-inode` asks Sleuth Kit `icat` to extract one metadata/inode address into a derived artifact.

## Prerequisites

Install The Sleuth Kit tools and keep these binaries in `PATH`:

```text
mmls
fls
icat
```

FrameTrace records command versions, command arguments, source image path, partition offset, output paths, hashes, and chained audit fields in `evidence/logs/tsk-audit.jsonl`.

## Inspect A Raw Image

For a full disk image with a partition table:

```bash
cargo run -- inspect-image ./case-001 ./case-001/evidence/images/blackbox.raw
```

FrameTrace runs:

- `mmls` to discover partition offsets.
- `fls -r -p -o <offset>` to recursively list file names.

If `mmls` finds an allocated partition, the first allocated start sector is used. If the image is already a file-system image, or the offset must be examiner-selected, pass it explicitly:

```bash
cargo run -- inspect-image ./case-001 ./case-001/evidence/images/blackbox.raw --partition-offset 2048
cargo run -- inspect-image ./case-001 ./filesystem-only.img --partition-offset 0
```

Outputs:

```text
db/filesystem/tsk-inspection-*.json
db/filesystem/tsk-files-*.jsonl
evidence/logs/tsk-mmls-*.txt
evidence/logs/tsk-fls-*.txt
evidence/logs/tsk-audit.jsonl
```

## Recover One Inode

Use an inode/metadata address from the `fls` output:

```bash
cargo run -- recover-inode ./case-001 ./case-001/evidence/images/blackbox.raw 1304-128-1 --partition-offset 2048 --recover-deleted
```

Default recovered outputs are written under:

```text
artifacts/recovered/filesystem/
```

Recovered files are labeled `candidate-unvalidated`. Validate playback, container structure, timestamps, and source context before reporting them as usable evidence.

Run `validate-artifact` after recovery to record the first container/playability result:

```bash
cargo run -- validate-artifact ./case-001 ./case-001/artifacts/recovered/filesystem/inode_1304-128-1.bin
```

## Boundaries

- The source image is not modified.
- `recover-inode` does not reconstruct proprietary DVR circular file systems by itself.
- Fragmented/deleted files may be incomplete depending on file-system reuse and overwrite state.
- Inode recovery is examiner-directed; FrameTrace does not automatically bulk-export every deleted file.
- E01 images should be imported to raw first with `import-e01`, then inspected with `inspect-image`.

## References

- Sleuth Kit `mmls`: https://sleuthkit.org/sleuthkit/man/mmls.html
- Sleuth Kit `fls`: https://www.sleuthkit.org/sleuthkit/man/fls.html
- Sleuth Kit `icat`: https://www.sleuthkit.org/sleuthkit/man/icat.html


## Batch recovery from the viewer

The viewer lists deleted video candidates (fls entries flagged deleted +
video_candidate) as records of kind "candidate". Select them and download the
selection list, then run:

```text
frametrace.exe recover-batch <CASE_DIR> <IMAGE.raw> selection.json --partition-offset <offset>
```

Each recovery is audited (recover-inode events) and the batch outcome is
chained into artifacts/logs/batch-log.jsonl. Recovered outputs stay
candidate-unvalidated until validate-batch / playback review.

## Extracting allocated (live) videos: `--allocated-videos`

```text
frametrace.exe recover-batch <CASE_DIR> <IMAGE.raw> --allocated-videos --partition-offset <offset>
```

copies every allocated (not deleted) video candidate of the newest
`inspect-image` listing out of the image with `icat` (no `-r`), keeping the
image's folder tree under:

```text
artifacts/extracted/<image path>
```

The workstation's E01 pipeline runs this step after `inspect-image` and then
indexes `artifacts/extracted` instead of the evidence folder — but only when
this run's recover-batch reports files extracted or kept (its closing
`recover batch result: … ok=N … skipped=M` line). Without Sleuth Kit, an
inspection, or any extracted file, the evidence folder is indexed as before.

- **Not a recovery.** Each copy is logged in `evidence/logs/tsk-audit.jsonl`
  as an `extract-inode` event (`recover_deleted:false`). Only `recover-inode`
  events are filesystem *recoveries*: extracted files never appear in the
  viewer's Recovery/Filesystem tree, the carved/recovered metric or the
  report's "파일시스템 복구" technique rows. They are indexed by scan-folder
  as ordinary video records. The tsk audit table of the report still lists
  the `extract-inode` events (audit trail).
- **Derived source.** scan-folder registers `artifacts/extracted` (or a folder
  in it) with the evidence-source kind `derived-extracted`, not `folder`: the
  original evidence is the forensic image.
- **Names.** Image paths keep folder names and extensions; `..`, roots, drive
  letters, characters Windows forbids (`<>:"/\|?*`), trailing dots/spaces and
  device names (`CON`, `NUL`, `CONIN$`, `CONOUT$`, `COM1`-`COM9`, `COM¹`-`COM³`,
  `LPT1`-`LPT9`, `LPT¹`-`LPT³` — these get a `_` prefix) are cleaned.
- **Never overwritten, never confused.** An existing file is kept and the item
  reported `skipped` only when a tsk audit record proves it is this item's own
  earlier output (same output path, image, partition offset, inode and size).
  A different source that maps to the same name — `/DCIM/Clip.MP4` vs
  `/DCIM/clip.mp4` on a case-insensitive Windows volume, `a:b.mp4` vs
  `a_b.mp4` after cleaning, another image or partition in a reused case — is
  written as `<name>__ino<inode>.<ext>` (then `…_001`, …). Deleted-file
  recovery (`{inode}_{name}` under `artifacts/recovered/filesystem`) uses
  `__off<offset>` the same way. Reruns find the same names again, so they never
  create extra copies (a 0-byte source included).
- **Disk space.** The `fls` listing carries no sizes, so the total cannot be
  checked up front: before each file the batch needs 1 GB free on the case
  volume plus the largest file extracted so far, and otherwise stops with
  "insufficient disk space for further extraction" (the remaining items are
  reported failed; finished files are kept and skipped on the rerun).
- **Interrupted runs.** Each file is written to a pid-tagged `.part` and only
  published when complete. The server's cancel sweep and the next
  recover-batch remove stale `.part` files anywhere under
  `artifacts/extracted` (subfolders included).
- `artifacts/extracted` is packaged with the case (`package-case`).
- **Known limits.**
  - The "same earlier output" check compares absolute paths. After the case
    folder or the image moves to another path or drive, a rerun treats every
    item as a different source and extracts a second copy as
    `<name>__ino<inode>` (nothing is overwritten; the copies are indexed
    twice).
  - An image file whose name looks like FrameTrace's own in-progress files
    (`.x.ft1234.part…`) is extracted with a `_` prefix so the stale-`.part`
    sweep never removes it.
  - Cases extracted by the PR #6 build before this change logged allocated
    extractions as `recover-inode`; the viewer and report still list those
    records under filesystem recovery until the case is extracted again.
  - "New folder" refuses a parent path that passes through any link,
    junction or folder-mounted volume (`C:\Mount\Data`), and on Unix any
    symlinked ancestor such as a linked `~/Documents`; the error names the
    link. Pick the real folder instead.
