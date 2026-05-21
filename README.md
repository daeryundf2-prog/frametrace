# FrameTrace

Windows local workstation concept for reviewing large blackbox, CCTV, SD card, and hard disk video evidence. This is not a server product.

This first implementation is the local core prototype:

- Creates a case folder with a forensic-friendly layout.
- Indexes video candidates from mounted media, copied evidence, or exported disk-image contents.
- Classifies likely manufacturer/source parser lanes such as generic media, dashcam SD-card layouts, and CCTV/NVR exports.
- Collects size, path, modification time, optional SHA-256, and optional `ffprobe` metadata.
- Generates a serverless HTML review dashboard that can be opened directly from disk.
- Generates an HTML case report.
- Exports selected source videos or time ranges as MP4 or AVI deliverables.
- Generates review proxy MP4 files and JPEG thumbnails.
- Inspects/verifies E01/Ex01/S01/L01 evidence containers through libewf tools and exports them to raw images for downstream carving or mounting.
- Lists active/deleted file-system entries from raw images through Sleuth Kit `mmls`/`fls` and recovers examiner-selected inodes through `icat`.
- Carves contiguous MP4/AVI/Dahua-DAV candidates from raw files or forensic image files.
- Validates indexed videos, carved candidates, and inode recoveries with `ffprobe` and records validation audit logs.
- Tracks evidence sources and long-running jobs in SQLite.
- Builds a checksummed report/review package directory.
- Includes a serverless Evidence Viewer GUI prototype for the final viewer-first desktop shell.

## Tech Stack Direction

- Target OS: Windows 10/11 x64.
- GUI timing: last phase, after the recovery, parsing, indexing, export, and reporting engine is stable.
- Desktop shell: C#/WinUI 3 for the final production Windows app, with Tauri kept only as an optional fallback if the engine contract is already stable.
- Core engine: Rust.
- Video metadata and transcode boundary: FFmpeg / ffprobe.
- Case DB: SQLite for the primary local index, with JSON/JSONL/TSV compatibility artifacts kept for review/export flows.
- Recovery path: libewf CLI tools for E01 evidence import now, Sleuth Kit / libtsk later for file-system analysis, custom carving plugins for DVR/CCTV formats.
- Review UI: local web UI now, desktop webview later.

The product should stay local-first. It should not require a server for evidence processing.
Do not build the final GUI first; the CLI/engine contract is the source of truth until the core forensic workflows are complete.

## Planning Docs

- `docs/TECH_STACK.md` - architecture and implementation direction.
- `docs/WINDOWS_USAGE.md` - Windows setup, build, and field usage notes.
- `docs/WINDOWS_IMPLEMENTATION_HANDOFF.md` - Windows-side continuation checklist for build, real evidence validation, GUI shell, and release packaging.
- `docs/WINDOWS_RISK_REVIEW.md` - Windows/GUI operational risks to resolve before production use.
- `docs/ACQUISITION_WORKFLOW.md` - source registration, write-protection, E01, and job tracking workflow.
- `docs/RECOVERY_BOUNDARIES.md` - implemented recovery scope and validation limits.
- `docs/FILESYSTEM_RECOVERY.md` - Sleuth Kit image inspection and inode recovery workflow.
- `docs/PERFORMANCE_VALIDATION.md` - SQLite scale benchmark and large-media rules.
- `docs/WINDOWS_VALIDATION.md` - reproducible Windows validation commands and CI.
- `docs/MVP_STATUS.md` - completed MVP scope and future boundaries.
- `docs/MANUFACTURER_PARSER_RESEARCH.md` - manufacturer-specific parser targets, priority, detection rules, and source links.
- `docs/EVIDENCE_VIEWER_GUI.md` - viewer-first GUI plan, screen model, and production boundary.
- `docs/OPENDESIGN_ADAPTATION.md` - OpenDesign-compatible FrameTrace design-system handoff notes.

## Windows Quick Start

For the remaining Windows-only work, start with `docs/WINDOWS_IMPLEMENTATION_HANDOFF.md`. It is the handoff checklist for cloning the repo on Windows, validating the current engine, testing E01/raw-image workflows, and then implementing the production Windows GUI shell.

Install Rust and FFmpeg first. `ffmpeg.exe` and `ffprobe.exe` must be available in `PATH`.

```powershell
cargo build --release
.\target\release\frametrace.exe init-case C:\Cases\case-001 --title "Sample CCTV review"
.\target\release\frametrace.exe import-e01 C:\Cases\case-001 D:\Images\blackbox.E01 --output C:\Cases\case-001\evidence\images\blackbox.raw
.\target\release\frametrace.exe inspect-image C:\Cases\case-001 C:\Cases\case-001\evidence\images\blackbox.raw --partition-offset 2048
.\target\release\frametrace.exe recover-inode C:\Cases\case-001 C:\Cases\case-001\evidence\images\blackbox.raw 1304-128-1 --partition-offset 2048 --recover-deleted
.\target\release\frametrace.exe register-source C:\Cases\case-001 E:\ --kind mounted-volume --write-protect "hardware write blocker"
.\target\release\frametrace.exe scan-folder C:\Cases\case-001 E:\ --no-ffprobe
.\target\release\frametrace.exe scan-folder C:\Cases\case-001 E:\BLACKBOX --hash --max-depth 2
.\target\release\frametrace.exe make-review C:\Cases\case-001
.\target\release\frametrace.exe make-thumbnail C:\Cases\case-001 vid_000001 --time 5
.\target\release\frametrace.exe make-proxy C:\Cases\case-001 vid_000001
.\target\release\frametrace.exe export-video C:\Cases\case-001 vid_000001 --format mp4 --start 10 --duration 30
.\target\release\frametrace.exe validate-artifact C:\Cases\case-001 vid_000001
.\target\release\frametrace.exe make-report C:\Cases\case-001
.\target\release\frametrace.exe package-case C:\Cases\case-001
```

Open `C:\Cases\case-001\review\index.html` in a browser after `make-review`.
Open `C:\Cases\case-001\review\evidence-viewer.html` for the viewer-first real case page after `make-review`.
Open `C:\Cases\case-001\reports\case-report.html` after `make-report`.

For terabyte-scale disks, run a fast first pass with `--no-ffprobe` and without `--hash`, then run deeper analysis only on selected folders or copied evidence. Repeated scans preserve the cumulative case index: `db/case.db` is the primary SQLite index, while `db/video_index.json`, `db/videos.jsonl`, and `db/video_paths.tsv` remain compatibility artifacts for review/export flows. Each scan run is also saved under `db/scan_runs/`.

## Development Commands

```bash
cargo run -- init-case ./case-001 --title "Sample CCTV review"
cargo run -- inspect-e01 ./case-001 /path/to/blackbox.E01
cargo run -- import-e01 ./case-001 /path/to/blackbox.E01 --output ./case-001/evidence/images/blackbox.raw
cargo run -- inspect-image ./case-001 ./case-001/evidence/images/blackbox.raw --partition-offset 2048
cargo run -- recover-inode ./case-001 ./case-001/evidence/images/blackbox.raw 1304-128-1 --partition-offset 2048 --recover-deleted
cargo run -- register-source ./case-001 /path/to/evidence --kind folder --write-protect "copied evidence"
cargo run -- scan-folder ./case-001 /path/to/evidence --no-ffprobe
cargo run -- scan-folder ./case-001 /path/to/evidence/BLACKBOX --hash --max-depth 2
cargo run -- make-review ./case-001
cargo run -- make-thumbnail ./case-001 vid_000001 --time 5
cargo run -- make-proxy ./case-001 vid_000001
cargo run -- export-video ./case-001 vid_000001 --format mp4 --start 10 --duration 30
cargo run -- export-video ./case-001 vid_000001 --format avi
cargo run -- validate-artifact ./case-001 vid_000001
cargo run -- carve-file ./case-001 /path/to/image-or-raw-file.bin --max-bytes 268435456
cargo run -- make-report ./case-001
cargo run -- package-case ./case-001
cargo run -- inspect ./case-001
cargo run -- list-parsers
cargo run -- benchmark-db ./target/frametrace-db-bench --rows 10000
```

Open `case-001/review/index.html` in a browser after `make-review`.
Open `case-001/review/evidence-viewer.html` to review actual indexed/candidate media in a viewer-first page.
Open `case-001/reports/case-report.html` after `make-report`.
Open `gui/evidence-viewer/index.html` to review the viewer-first GUI prototype. The prototype defaults to Korean and can be switched to English from the top-right language button.

By default, `scan-folder` skips full SHA-256 hashing because terabyte-scale evidence can take hours. Use `--hash` when the evidence workflow requires per-file hashes.

Exported clips are written to `case-001/artifacts/clips/` unless `--output` is provided.
Proxy files are written to `case-001/artifacts/proxies/`, thumbnails to `case-001/artifacts/thumbnails/`, and carved recovery candidates to `case-001/artifacts/carved/`.
Filesystem inode recoveries are written to `case-001/artifacts/recovered/filesystem/`.
Default clip/proxy/thumbnail names are made unique when a file already exists. Explicit `--output` paths are never overwritten; choose a new path if the target already exists.

`init-case` can record chain-of-custody context up front:

```bash
cargo run -- init-case ./case-001 --title "Client CCTV review" --operator "Examiner" --device-id "SD-001" --device-serial "SN123" --write-protect "hardware write blocker" --acquisition-tool "FTK Imager" --evidence-hash "<device-or-image-sha256>"
```

Export, proxy, thumbnail, validation, and carve logs include SHA-256 values and tamper-evident hash-chain fields. Carved and inode-recovered files are intentionally labeled as `candidate-unvalidated` until playback/container validation is performed.

E01 support requires libewf command-line tools in `PATH`: `ewfinfo`, `ewfverify`, and `ewfexport`. `import-e01` verifies the E01, exports a raw image, hashes the raw output, and writes `evidence/logs/e01-audit.jsonl`. To inspect file-system contents, mount the E01/raw image read-only with a forensic mounter and run `scan-folder` on the mounted volume. To recover contiguous embedded video candidates directly from the raw image, run `carve-file` against the exported `.raw`.
For file-system-aware deleted-file triage, install Sleuth Kit tools in `PATH` and run `inspect-image`/`recover-inode` against the exported `.raw`. These outputs remain `candidate-unvalidated` until examiner playback/container validation is recorded.
