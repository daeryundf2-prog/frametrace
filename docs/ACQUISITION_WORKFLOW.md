# Acquisition Workflow

FrameTrace remains a local workstation tool. Source evidence should be treated as read-only and every import, scan, or recovery action should write only inside the case folder.

## Source Registration

Register every received source before processing when the intake details are known:

```powershell
.\target\release\frametrace.exe register-source C:\Cases\case-001 E:\ --kind mounted-volume --write-protect "hardware write blocker" --acquisition-tool "FTK Imager" --evidence-hash "<device-or-image-sha256>" --notes "Client SD card"
```

Supported source kinds:

- `folder` for copied evidence folders.
- `mounted-volume` for read-only mounted SD/HDD/image volumes.
- `e01` for E01/Ex01/S01/L01 evidence containers.
- `raw-image` for exported `.raw`, `.img`, or similar image files.
- `physical-drive` as an intake record only; direct raw drive acquisition is not implemented yet.

`scan-folder`, `inspect-e01`, `import-e01`, and `carve-file` automatically register the source path if it is not already present. Manual registration is still preferred because it captures write-protection and acquisition notes before processing.

## Job Tracking

Long-running commands create rows in SQLite:

- `jobs` records the command, status, subject path, timestamps, options, and error text.
- `job_events` records start/progress/complete/failure events.

The current implementation records durable status for audit and resume design. Mid-command resume is intentionally not claimed yet; future jobs should add range checkpoints before exposing a resume flag.

## E01 Path

For E01 media:

1. Register or inspect the E01.
2. Run `import-e01` to verify/export raw through libewf.
3. Mount the E01/raw image read-only with a forensic mounter and run `scan-folder` on the mounted drive letter.
4. Run `carve-file` against the raw image only for signature-based contiguous recovery candidates.

Do not execute vendor player EXEs from evidence packages automatically. Preserve them as evidence/export context.
