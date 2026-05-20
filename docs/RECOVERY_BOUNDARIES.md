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
- Court-ready validation of proprietary recovered formats without examiner playback/container verification.

## Examiner Validation Rule

Every carved or inode-recovered file remains a candidate until validated. The carve log separates:

- `candidate-unvalidated`: first copy of a signature-based contiguous candidate.
- `duplicate-candidate`: carved output hash matches an earlier candidate.

Before reporting a recovered clip as usable evidence, validate container structure and playback with FFmpeg/ffprobe, a trusted vendor player, or a documented specialist workflow.

For Sleuth Kit inode recovery, also record the selected partition offset, inode/metadata address, and whether `icat -r` was used for deleted-file recovery. FrameTrace logs these values in `evidence/logs/tsk-audit.jsonl`.

`validate-artifact` can promote an artifact's review signal to `verified-playable` only when `ffprobe` parses a video stream. It does not prove event relevance, timestamp accuracy, or court admissibility by itself.
