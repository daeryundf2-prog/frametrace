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
