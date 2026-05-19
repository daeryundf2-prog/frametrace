# FrameTrace MVP Status

FrameTrace is currently a Windows-first local forensic video workstation core. It is implemented as a Rust CLI prototype so the evidence-processing contract can stabilize before a Windows GUI shell is added. The GUI is intentionally a final phase, not an early milestone.

## Completed in This MVP

- Product name and executable name: `FrameTrace` / `frametrace.exe`.
- Windows 10/11 x64 local-only operating model.
- Case folder creation with evidence, database, review, report, and artifact directories.
- Folder scan for mounted drives, copied SD cards, exported CCTV folders, and extracted image contents.
- Cumulative case index across repeated scans, with per-run snapshots under `db/scan_runs`.
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
- JSON, JSONL, and TSV index output.
- Serverless HTML review dashboard.
- HTML case report with source/parser assessment.
- MP4 and AVI export using FFmpeg.
- Export audit log at `artifacts/clips/export-log.jsonl`.
- Review proxy MP4 generation.
- JPEG thumbnail generation.
- Default export/proxy/thumbnail output names avoid collisions; explicit `--output` paths are not overwritten.
- Contiguous MP4/AVI/Dahua-DAV candidate carving from raw files or acquired image files.
- Recovery artifact logs with source offsets, output hashes, and carved output paths.

## Deliberately Not Claimed Yet

- Raw `\\.\PhysicalDriveN` acquisition.
- E01 image creation/verification.
- File-system-aware deleted file reconstruction.
- File-system-aware unallocated-space carving.
- Proprietary DVR/NVR file-system recovery.
- Court/admissibility validation for recovered proprietary formats.
- Windows GUI packaging.
- Final Windows GUI shell.
- PDF report rendering.
- Motion/object/license-plate analysis.

## Next Product Milestones

1. Replace JSON/JSONL prototype state with SQLite job/index tables.
2. Add raw image/folder acquisition workflows with explicit write-protection guidance.
3. Add file-system-aware deleted-file recovery and unallocated-space carving.
4. Add stronger carving validation, preview triage, and deduplication.
5. Add per-vendor parser plugins one at a time, starting with Dahua DAV and BlackVue/Thinkware-style dashcam metadata.
6. Add report signing/export packaging and PDF rendering.
7. Add the C#/WinUI 3 shell only after the command contract and core forensic workflows are stable.
