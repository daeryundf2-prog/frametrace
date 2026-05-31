# Validation Corpus Manifest

Status: corpus structure defined. Real evidence files remain external and must not be committed.

## Storage Rule

Keep media, raw images, and E01 files outside git. Commit only manifests, hashes, expected-output summaries, and generated QA reports that do not contain sensitive evidence.

## Corpus A: Deleted File Recovery

- Purpose: validate deleted-video discovery and inode recovery from a raw filesystem image.
- Ground truth: generated filesystem fixture notes plus SHA-256 of deleted source file before deletion.
- Expected outputs: `inspect-image` identifies candidate path/inode; `recover-inode` writes a hashed artifact; `validate-artifact` marks it confirmed or failed.
- Pass criteria: required deleted video recovered or explicitly marked unsupported with reason; no output outside the case directory.

## Corpus B: Browser Artifacts

- Purpose: future browser-history/media-reference parsing.
- Ground truth: browser fixture export and expected URL/file-reference table.
- Expected outputs: design-candidate only until parser PRD exists.
- Pass criteria: not release-blocking for current video recovery release.

## Corpus C: Windows Event Logs

- Purpose: future event-log timeline context.
- Ground truth: EVTX fixture with known timestamps and event IDs.
- Expected outputs: design-candidate only until parser PRD exists.
- Pass criteria: not release-blocking for current video recovery release.

## Corpus D: Timeline Reconstruction

- Purpose: validate ordering across scan time, modified time, export time, recovery time, and validation time.
- Ground truth: fixture event table.
- Expected outputs: sorted report rows and deterministic normalized output.
- Pass criteria: timestamp order matches ground truth with zero P0 ordering errors.

## Corpus E: Large Evidence Dataset

- Purpose: validate that indexing and SQLite operations survive large case sizes.
- Ground truth: generated synthetic row count and optional media hash manifest.
- Expected outputs: `qa performance` report and completion log.
- Pass criteria: `rows_per_minute >= 50000` and no database migration/indexing failure.

## Corpus F: Mixed Real-World Case Dataset

- Purpose: validate operator workflow across generic video, DVR extensions, carved candidates, and filesystem recovery.
- Ground truth: examiner-approved TSV manifest plus source acquisition notes.
- Expected outputs: accuracy, reproducibility, report-defense, and package artifacts.
- Pass criteria: precision >= 0.98, recall >= 0.98, hash mismatch = 0, report-defense pass.

## Manifest Template

```text
source_path	sha256	case_id	corpus_id	priority	expected_status	notes
/absolute/path/to/video.mp4	<sha256-or-empty>	FT-CORPUS-A-001	A	P0	ffprobe-video-stream-confirmed	deleted source recovered by inode
```

`qa accuracy` currently reads the first two columns. Additional columns are retained for reviewer context and future corpus tooling.
