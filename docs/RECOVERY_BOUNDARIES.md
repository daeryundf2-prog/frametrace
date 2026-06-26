# Recovery Boundaries

FrameTrace currently supports practical first-pass recovery triage, not full proprietary DVR/HDD reconstruction.

## Implemented

- E01/Ex01/S01/L01 inspection, verification, and raw export through external libewf tools.
- Folder scanning from copied evidence, mounted volumes, or mounted images.
- File-system-aware image listing through external Sleuth Kit `mmls`/`fls`.
- Examiner-selected inode recovery through external Sleuth Kit `icat`.
- Signature-based contiguous carving for MP4 `ftyp`, RIFF AVI, and Dahua DHAV candidates.
- ffprobe-based validation logging for indexed videos, carved candidates, and inode recoveries.
- Candidate hashes, offsets, output paths, validation notes, duplicate candidate marking, and chained carve logs.

## Not Claimed

- Direct raw `\\.\PhysicalDriveN` acquisition.
- Automatic bulk deleted-file reconstruction.
- Proprietary DVR/NVR file-system parsing.
- Fragmented video reconstruction.
- Report-defensible validation claims for proprietary recovered formats without examiner playback/container verification.

## Examiner Validation Rule

Every carved or inode-recovered file remains a candidate until validated. The carve log separates:

- `candidate-unvalidated`: first copy of a signature-based contiguous candidate.
- `duplicate-candidate`: carved output hash matches an earlier candidate.

Before reporting a recovered clip as usable evidence, validate container structure and playback with FFmpeg/ffprobe, a trusted vendor player, or a documented specialist workflow.

For Sleuth Kit inode recovery, also record the selected partition offset, inode/metadata address, and whether `icat -r` was used for deleted-file recovery. FrameTrace logs these values in `evidence/logs/tsk-audit.jsonl`.

`validate-artifact` can promote an artifact's review signal to `ffprobe-video-stream-confirmed` only when `ffprobe` parses a video stream. It does not prove event relevance, timestamp accuracy, or legal admissibility by itself.

## Distributable Path Privacy

FrameTrace redacts workstation-local source paths and `file://` URLs from distributable report, review, viewer, and package outputs by default. Shared outputs use source IDs, case-relative artifact paths, and redacted labels such as `[redacted-source:vid_000001]`.

Full local path display/export is available only through explicit local/operator mode (`--include-full-paths` on report, review, and package commands). That mode writes `privacy-full-path-disclosure.json` next to the generated distributable artifact so QA/review evidence can identify the operator opt-in.

Internal SQLite provenance and audit logs remain full-fidelity inside the active case directory. Redaction is applied when generating distributable outputs or copied package artifacts, not by removing source provenance from the working case.
