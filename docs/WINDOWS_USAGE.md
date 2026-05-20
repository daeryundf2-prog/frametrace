# FrameTrace Windows Usage Notes

Target environment: Windows 10/11 x64, local examiner PC, attached HDD/SSD/SD card/USB media, no processing server.

## Install Prerequisites

1. Install Rust for Windows with the MSVC toolchain:
   - https://rustup.rs/
   - Visual Studio Build Tools may be required for the MSVC linker.
2. Install FFmpeg for Windows and make sure both tools are in `PATH`:
   - `ffmpeg.exe`
   - `ffprobe.exe`
3. Install libewf tools if E01/Ex01 evidence containers will be processed:
   - `ewfinfo.exe`
   - `ewfverify.exe`
   - `ewfexport.exe`
4. Install The Sleuth Kit tools if raw-image file-system inspection or deleted inode recovery will be processed:
   - `mmls.exe`
   - `fls.exe`
   - `icat.exe`
5. Use PowerShell or Windows Terminal.

Check the tools:

```powershell
rustc --version
cargo --version
ffmpeg -version
ffprobe -version
ewfinfo -V
ewfverify -V
ewfexport -V
mmls -V
fls -V
icat -V
```

## Build

From the project folder:

```powershell
cargo build --release
```

The executable is:

```text
.\target\release\frametrace.exe
```

## Recommended Field Workflow

Use a case folder on a fast local SSD when possible:

```powershell
.\target\release\frametrace.exe init-case C:\Cases\case-001 --title "Client CCTV review"
```

When the receiving form has these details, record them in the case manifest immediately:

```powershell
.\target\release\frametrace.exe init-case C:\Cases\case-001 --title "Client CCTV review" --operator "Examiner" --device-id "SD-001" --device-serial "SN123" --write-protect "hardware write blocker" --acquisition-tool "FTK Imager" --evidence-hash "<device-or-image-sha256>"
```

Run a fast inventory pass first. This avoids spending hours hashing or probing every file on a terabyte-scale source.

```powershell
.\target\release\frametrace.exe register-source C:\Cases\case-001 E:\ --kind mounted-volume --write-protect "hardware write blocker"
.\target\release\frametrace.exe scan-folder C:\Cases\case-001 E:\ --no-ffprobe
```

The scan still records likely source/parser lanes from extensions and folder names, even when `ffprobe` is skipped. It also records SQLite evidence-source and job rows.

If the blackbox SD card was acquired as E01, inspect and import it first:

```powershell
.\target\release\frametrace.exe inspect-e01 C:\Cases\case-001 D:\Images\blackbox.E01
.\target\release\frametrace.exe import-e01 C:\Cases\case-001 D:\Images\blackbox.E01 --output C:\Cases\case-001\evidence\images\blackbox.raw
```

`import-e01` uses `ewfinfo`, `ewfverify`, and `ewfexport`. It writes E01 provenance logs under `evidence\logs`, hashes the exported raw image, and appends `evidence\logs\e01-audit.jsonl`.

For normal video-file review, mount the E01 or exported raw image read-only with a forensic mounter and run `scan-folder` on the mounted drive letter. For direct signature recovery from the image, run `carve-file` on the exported raw file.

For file-system-aware triage without mounting, inspect the exported raw image with Sleuth Kit:

```powershell
.\target\release\frametrace.exe inspect-image C:\Cases\case-001 C:\Cases\case-001\evidence\images\blackbox.raw
```

If the correct file-system offset is known or must be selected manually:

```powershell
.\target\release\frametrace.exe inspect-image C:\Cases\case-001 C:\Cases\case-001\evidence\images\blackbox.raw --partition-offset 2048
```

Recover one inode/metadata address from the `fls` listing:

```powershell
.\target\release\frametrace.exe recover-inode C:\Cases\case-001 C:\Cases\case-001\evidence\images\blackbox.raw 1304-128-1 --partition-offset 2048 --recover-deleted
```

`inspect-image` writes `db\filesystem\tsk-files-*.jsonl` and raw tool logs under `evidence\logs`. `recover-inode` writes derived candidates under `artifacts\recovered\filesystem` and appends `evidence\logs\tsk-audit.jsonl`.

After the first inventory, run deeper metadata collection on a narrowed source folder when needed:

```powershell
.\target\release\frametrace.exe scan-folder C:\Cases\case-001 E:\BLACKBOX --hash --max-depth 2
```

Repeated scans are cumulative. `C:\Cases\case-001\db\case.db` is the primary SQLite index, the JSON/JSONL/TSV files remain compatibility artifacts, and every individual scan run is preserved under `C:\Cases\case-001\db\scan_runs`.

Generate the local review page:

```powershell
.\target\release\frametrace.exe make-review C:\Cases\case-001
```

Open:

```text
C:\Cases\case-001\review\index.html
C:\Cases\case-001\review\evidence-viewer.html
```

`evidence-viewer.html` is the viewer-first real case page. It uses the current video index, carving log, and validation log; no server is required.

Export client deliverables:

```powershell
.\target\release\frametrace.exe export-video C:\Cases\case-001 vid_000001 --format mp4 --start 10 --duration 30
.\target\release\frametrace.exe export-video C:\Cases\case-001 vid_000001 --format avi
```

Generate review artifacts:

```powershell
.\target\release\frametrace.exe make-thumbnail C:\Cases\case-001 vid_000001 --time 5
.\target\release\frametrace.exe make-proxy C:\Cases\case-001 vid_000001 --max-width 1280
```

Default export, proxy, and thumbnail names are made unique when a file already exists. When `--output` is provided, FrameTrace refuses to overwrite an existing file.
The derived artifact logs include output SHA-256 values, FFmpeg version text, command arguments, and hash-chain fields so later report review can detect missing or reordered log entries.

Carve contiguous video candidates from an acquired image file or raw export file:

```powershell
.\target\release\frametrace.exe carve-file C:\Cases\case-001 D:\Images\client-sdcard.img --max-bytes 536870912 --max-candidates 128
```

For an imported E01:

```powershell
.\target\release\frametrace.exe carve-file C:\Cases\case-001 C:\Cases\case-001\evidence\images\blackbox.raw --max-bytes 536870912 --max-candidates 128
```

`carve-file` is a candidate-recovery pass, not a full proprietary DVR file-system parser. It preserves offsets, hashes the carved outputs, marks duplicate carved hashes, and writes logs under `artifacts\carved`.
Carved artifacts remain `candidate-unvalidated` or `duplicate-candidate` until playback/container validation is done.

Validate an indexed video, carved candidate ID, recovered inode output path, or direct file path with `ffprobe`:

```powershell
.\target\release\frametrace.exe validate-artifact C:\Cases\case-001 vid_000001
.\target\release\frametrace.exe validate-artifact C:\Cases\case-001 carve_000001
.\target\release\frametrace.exe validate-artifact C:\Cases\case-001 C:\Cases\case-001\artifacts\recovered\filesystem\inode_1304-128-1.bin
```

Validation results are appended to `evidence\logs\validation-log.jsonl` and shown in the HTML report and evidence viewer. `verified-playable` means `ffprobe` found a video stream; the examiner should still perform playback review before final reporting.

Generate the current HTML report:

```powershell
.\target\release\frametrace.exe make-report C:\Cases\case-001
```

Open:

```text
C:\Cases\case-001\reports\case-report.html
```

Build a checksummed transfer/review package:

```powershell
.\target\release\frametrace.exe package-case C:\Cases\case-001
```

Open `reports\case-report.html` from the package and print to PDF when a PDF deliverable is required.

## Evidence Handling Rules

- Treat source media as read-only.
- Prefer write blockers or forensic images when the workflow requires strict preservation.
- Do not run vendor self-player `.exe` files automatically.
- Keep derived files, logs, reports, and exports inside the case folder.
- Start with mounted volumes and copied evidence folders. Raw `\\.\PhysicalDriveN` access should be added only after imaging and recovery behavior is validated.
- On unstable disks, image first with a dedicated acquisition tool, then scan the mounted image or extracted folder.
- For E01/Ex01 images, keep the original E01 set unchanged. Use `import-e01` only to create a derived raw working image inside the case folder.
- For Sleuth Kit recovery, preserve the selected partition offset and inode value in notes/report review. Inode-recovered files remain candidates until validated.

## FFmpeg Notes

The prototype calls `ffmpeg` and `ffprobe` by name, so Windows must be able to find them through `PATH`.

If a future GUI bundles FFmpeg, the app should validate the bundled binary path and log the exact binary/version used for every export.
