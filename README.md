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
- Carves contiguous MP4/AVI/Dahua-DAV candidates from raw files or forensic image files.

## Tech Stack Direction

- Target OS: Windows 10/11 x64.
- GUI timing: last phase, after the recovery, parsing, indexing, export, and reporting engine is stable.
- Desktop shell: C#/WinUI 3 for the final production Windows app, with Tauri kept only as an optional fallback if the engine contract is already stable.
- Core engine: Rust.
- Video metadata and transcode boundary: FFmpeg / ffprobe.
- Case DB: JSON/JSONL for the prototype, SQLite for the real workstation.
- Recovery future path: Sleuth Kit / libtsk for file-system analysis, libewf for E01 evidence containers, custom carving plugins for DVR/CCTV formats.
- Review UI: local web UI now, desktop webview later.

The product should stay local-first. It should not require a server for evidence processing.
Do not build the final GUI first; the CLI/engine contract is the source of truth until the core forensic workflows are complete.

## Planning Docs

- `docs/TECH_STACK.md` - architecture and implementation direction.
- `docs/WINDOWS_USAGE.md` - Windows setup, build, and field usage notes.
- `docs/MVP_STATUS.md` - completed MVP scope and future boundaries.
- `docs/MANUFACTURER_PARSER_RESEARCH.md` - manufacturer-specific parser targets, priority, detection rules, and source links.

## Windows Quick Start

Install Rust and FFmpeg first. `ffmpeg.exe` and `ffprobe.exe` must be available in `PATH`.

```powershell
cargo build --release
.\target\release\frametrace.exe init-case C:\Cases\case-001 --title "Sample CCTV review"
.\target\release\frametrace.exe scan-folder C:\Cases\case-001 E:\ --no-ffprobe
.\target\release\frametrace.exe scan-folder C:\Cases\case-001 E:\BLACKBOX --hash --max-depth 2
.\target\release\frametrace.exe make-review C:\Cases\case-001
.\target\release\frametrace.exe make-thumbnail C:\Cases\case-001 vid_000001 --time 5
.\target\release\frametrace.exe make-proxy C:\Cases\case-001 vid_000001
.\target\release\frametrace.exe export-video C:\Cases\case-001 vid_000001 --format mp4 --start 10 --duration 30
.\target\release\frametrace.exe make-report C:\Cases\case-001
```

Open `C:\Cases\case-001\review\index.html` in a browser after `make-review`.
Open `C:\Cases\case-001\reports\case-report.html` after `make-report`.

For terabyte-scale disks, run a fast first pass with `--no-ffprobe` and without `--hash`, then run deeper analysis only on selected folders or copied evidence. Repeated scans preserve the cumulative case index: `db/video_index.json`, `db/videos.jsonl`, and `db/video_paths.tsv` keep previously indexed videos while refreshed records for the same source path are updated. Each scan run is also saved under `db/scan_runs/`.

## Development Commands

```bash
cargo run -- init-case ./case-001 --title "Sample CCTV review"
cargo run -- scan-folder ./case-001 /path/to/evidence --no-ffprobe
cargo run -- scan-folder ./case-001 /path/to/evidence/BLACKBOX --hash --max-depth 2
cargo run -- make-review ./case-001
cargo run -- make-thumbnail ./case-001 vid_000001 --time 5
cargo run -- make-proxy ./case-001 vid_000001
cargo run -- export-video ./case-001 vid_000001 --format mp4 --start 10 --duration 30
cargo run -- export-video ./case-001 vid_000001 --format avi
cargo run -- carve-file ./case-001 /path/to/image-or-raw-file.bin --max-bytes 268435456
cargo run -- make-report ./case-001
cargo run -- inspect ./case-001
```

Open `case-001/review/index.html` in a browser after `make-review`.
Open `case-001/reports/case-report.html` after `make-report`.

By default, `scan-folder` skips full SHA-256 hashing because terabyte-scale evidence can take hours. Use `--hash` when the evidence workflow requires per-file hashes.

Exported clips are written to `case-001/artifacts/clips/` unless `--output` is provided.
Proxy files are written to `case-001/artifacts/proxies/`, thumbnails to `case-001/artifacts/thumbnails/`, and carved recovery candidates to `case-001/artifacts/carved/`.
Default clip/proxy/thumbnail names are made unique when a file already exists. Explicit `--output` paths are never overwritten; choose a new path if the target already exists.
