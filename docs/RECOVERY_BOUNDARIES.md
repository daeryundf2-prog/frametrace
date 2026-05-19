# Recovery Boundaries

FrameTrace currently supports practical first-pass recovery triage, not full proprietary DVR/HDD reconstruction.

## Implemented

- E01/Ex01/S01/L01 inspection, verification, and raw export through external libewf tools.
- Folder scanning from copied evidence, mounted volumes, or mounted images.
- Signature-based contiguous carving for MP4 `ftyp`, RIFF AVI, and Dahua DHAV candidates.
- Candidate hashes, offsets, output paths, validation notes, duplicate candidate marking, and chained carve logs.

## Not Claimed

- Direct raw `\\.\PhysicalDriveN` acquisition.
- File-system-aware deleted file reconstruction.
- Proprietary DVR/NVR file-system parsing.
- Fragmented video reconstruction.
- Court-ready validation of proprietary recovered formats without examiner playback/container verification.

## Examiner Validation Rule

Every carved file remains a candidate until validated. The carve log now separates:

- `candidate-unvalidated`: first copy of a signature-based contiguous candidate.
- `duplicate-candidate`: carved output hash matches an earlier candidate.

Before reporting a recovered clip as usable evidence, validate container structure and playback with FFmpeg/ffprobe, a trusted vendor player, or a documented specialist workflow.
