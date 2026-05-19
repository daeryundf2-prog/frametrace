# FrameTrace MVP Status

FrameTrace is currently a Windows-first local forensic video workstation core. It is implemented as a Rust CLI prototype so the evidence-processing contract can stabilize before a Windows GUI shell is added. The GUI is intentionally a final phase, not an early milestone.

## Completed in This MVP

- Product name and executable name: `FrameTrace` / `frametrace.exe`.
- Windows 10/11 x64 local-only operating model.
- Case folder creation with evidence, database, review, report, and artifact directories.
- Case manifest fields for operator, host, device ID/serial, write-protection state, acquisition tool, evidence hash, and notes.
- Folder scan for mounted drives, copied SD cards, exported CCTV folders, and extracted image contents.
- Cumulative case index across repeated scans, with per-run snapshots under `db/scan_runs`.
- SQLite primary video index at `db/case.db`, updated by `scan-folder`.
- SQLite evidence-source registry plus job/event tables for scan, E01 import, and carving auditability.
- E01/Ex01/S01/L01 container inspection and raw export through external libewf tools.
- Terabyte-aware defaults:
  - per-file SHA-256 is opt-in with `--hash`
  - `ffprobe` can be skipped with `--no-ffprobe`
  - scan depth can be limited with `--max-depth`
- Video/export candidate detection by extension and file signature.
- Manufacturer/source parser lane detection:
  - generic media
  - dashcam SD-card layouts
  - Dahua DAV
  - Hikvision path signals
  - BlackVue-style channel suffixes
  - Thinkware/iNavi-style event folders
  - Garmin GLV/DCIM
  - Hanwha/Wisenet NOV
  - Genetec G64/G64x
  - Avigilon AVE
  - Milestone/XProtect BLK/path signals
  - Axis/ONVIF and other researched vendor path signals
- JSON, JSONL, and TSV compatibility index output.
- Serverless HTML review dashboard.
- HTML case report with source/parser assessment.
- MP4 and AVI export using FFmpeg.
- Export audit log at `artifacts/clips/export-log.jsonl`.
- Review proxy MP4 generation.
- JPEG thumbnail generation.
- Default export/proxy/thumbnail output names avoid collisions; explicit `--output` paths are not overwritten.
- Derived export/proxy/thumbnail logs include output SHA-256, source-index SHA-256 when available, FFmpeg version, command arguments, and hash-chain fields.
- Contiguous MP4/AVI/Dahua-DAV candidate carving from raw files or acquired image files.
- Recovery artifact logs with source offsets, output hashes, carved output paths, duplicate-candidate marking, candidate-validation status, and hash-chain fields.
- E01 provenance logs under `evidence/logs/e01-audit.jsonl`.
- Checksummed case package output with `package-case`.
- Parser plugin catalog output with `list-parsers`.
- Synthetic SQLite scale benchmark with `benchmark-db`.

## Deliberately Not Claimed Yet

- Raw `\\.\PhysicalDriveN` acquisition.
- Native in-process E01 parsing without external libewf command-line tools.
- E01 image creation/acquisition.
- File-system-aware extraction from E01/raw images without mounting.
- File-system-aware deleted file reconstruction.
- File-system-aware unallocated-space carving.
- Mid-command resume from persisted job checkpoints.
- Proprietary DVR/NVR file-system recovery.
- Court/admissibility validation for recovered proprietary formats.
- Cryptographic signing or external timestamping of reports/logs.
- Windows GUI packaging.
- Final Windows GUI shell.
- PDF report rendering.
- Motion/object/license-plate analysis.

## Next Product Milestones

1. Add mid-command resume checkpoints for scan/carve/import jobs.
2. Add file-system-aware deleted-file recovery and unallocated-space carving.
3. Add stronger carving preview triage and container validation.
4. Add per-vendor parser implementations one at a time, starting with Dahua DAV and BlackVue/Thinkware-style dashcam metadata.
5. Add cryptographic signing/external timestamping and native PDF rendering.
6. Add the C#/WinUI 3 shell only after the command contract and core forensic workflows are stable.
