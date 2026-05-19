# Technical Stack

## Product Shape

FrameTrace is a Windows 10/11 x64 local workstation, not a server product.

Evidence stays on the examiner PC or attached storage. The application owns a local case folder containing logs, derived artifacts, thumbnails, proxies, notes, and reports. The source medium is treated as read-only.

## Recommended Stack

| Layer | Choice | Reason |
| --- | --- | --- |
| Final desktop app | C#/WinUI 3 production shell | Best fit for a Windows-only forensic workstation, native dialogs, admin flows, removable-drive UX, and Windows packaging |
| GUI timing | Last phase | The engine must prove recovery, parsing, indexing, export, and reporting behavior before GUI work wraps it |
| Alternative UI path | Tauri | Keep only as a fallback after the engine contract is stable |
| Core engine | Rust | Safe low-level I/O, predictable performance, good Windows support |
| Video tools | FFmpeg / ffprobe | Mature codec/container support and metadata extraction |
| Case state | SQLite | Durable local indexing for millions of artifacts without a server |
| Prototype state | JSON + JSONL | Easy to inspect while the model is still changing |
| Recovery | libewf CLI tools now, Sleuth Kit/libtsk later, custom carvers | E01/raw image import now; partition/file-system analysis later |
| Analysis | OpenCV/ONNX Runtime later | Motion/person/vehicle/license-plate candidate extraction |
| Reports | Local HTML first, PDF later | Reviewable and portable case output |
| Deliverable video | MP4 or AVI via FFmpeg | Client-friendly exports from selected evidence ranges |

## Windows Deployment Shape

- Build the final GUI and signed Windows `.exe` or MSIX installer after the core engine features are complete.
- Ship the Rust core as either:
  - a standalone engine executable called by the GUI, or
  - a Rust library exposed through a narrow FFI boundary after the command contract stabilizes.
- Bundle or validate FFmpeg/ffprobe explicitly. The prototype expects them in `PATH`.
- Use admin elevation only for raw physical-drive access. Folder scans, copied evidence, and mounted SD cards should run without elevation.
- Keep all case writes inside the selected case folder, never beside the source evidence by default.
- Treat vendor EXE players as evidence/export packages; do not execute them automatically.

## Processing Model

```text
Source media or image
  -> read-only evidence reader
  -> E01 inspection/verification/raw export where needed
  -> partition/file-system indexer
  -> normal files + deleted entries + unallocated ranges
  -> video candidate registry
  -> manufacturer/source parser detection
  -> ffprobe metadata
  -> carving candidates from image/raw files
  -> proxy/thumbnail/event generation
  -> review UI + report
```

## Build Order

1. Stabilize the Rust CLI engine and command outputs.
2. Move case/index state to SQLite with resumable jobs.
3. Add raw image and recovery workflows.
4. Implement vendor parser plugins and validation samples.
5. Expand thumbnail/proxy generation, carving validation, and review/report export completeness.
6. Only then build the Windows GUI shell around the stable engine contract.

## Large-Media Rules

- Never load entire evidence into memory.
- Prefer sequential reads.
- Store progress checkpoints by source, range, and job.
- Make full hashing explicit because it is expensive on terabyte-scale inputs.
- Generate proxies and thumbnails lazily.
- Analyze sampled frames first, then escalate to dense analysis only for selected ranges.
- Keep source evidence immutable; write only to the case folder.
- On Windows, first support mounted volumes and copied evidence folders; raw `\\.\PhysicalDriveN` access comes after imaging/recovery validation.
- Prefer a two-pass workflow: fast inventory first, then hash/ffprobe/export selected evidence.

## Prototype Boundary

Implemented now:

- Case layout.
- Folder-based video candidate scan.
- Manufacturer/source parser lane detection for researched dashcam and CCTV export families.
- Optional SHA-256 hashing.
- Optional ffprobe metadata.
- JSON/JSONL index.
- Serverless review dashboard.
- HTML case report.
- MP4/AVI export command.
- Review proxy and thumbnail generation.
- E01/Ex01/S01/L01 inspection, verification, and raw export through `ewfinfo`, `ewfverify`, and `ewfexport`.
- Contiguous MP4/AVI/Dahua-DAV carving from raw files or acquired image files.

Not implemented yet:

- Raw `\\.\PhysicalDriveX` acquisition.
- E01 creation/acquisition.
- Native in-process E01 parsing without external libewf command-line tools.
- File-system-aware extraction from E01/raw images without mounting.
- File-system-aware deleted file recovery.
- File-system-aware unallocated-space carving.
- SQLite job queue.
- Motion/object/license-plate analysis.
- PDF report rendering.
- Final Windows GUI shell.
