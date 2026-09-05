# Commercial Tool Handoff — Amped FIVE / Magnet DVR Examiner

Status: documented (2026-09-05)  
Audience: examiners who finish triage in FrameTrace and continue analysis in Amped FIVE or Magnet DVR Examiner (DME).

FrameTrace stays a **local review / provenance workstation**. It does not replace Amped Authenticate-style tamper proofing or DME proprietary DVR filesystem recovery. This document only describes how to **hand off** a FrameTrace case package without breaking hashes or chain-of-custody notes.

## 1. Produce the package

```powershell
.\target\release\frametrace.exe make-report C:\Cases\case-001
.\target\release\frametrace.exe package-case C:\Cases\case-001
```

Default output: `C:\Cases\case-001\reports\package_<unix>\`

Before transfer, verify:

```powershell
Get-FileHash -Algorithm SHA256 .\package-manifest.json
# Or recompute every listed file against manifest.sha256
```

`manifest.sha256` lines are `sha256  relative/path` (two spaces). Any mismatch means the package was altered after packaging.

## 2. Package layout (what each tool needs)

| Path in package | Contents | FIVE | DME |
|---|---|---|---|
| `case.json` | Case id, operator, write-protect / acquisition notes | Read for notes | Read for notes |
| `db/case.db` | SQLite index (videos, jobs, review_marks) | Optional | Optional |
| `db/video_index.json` / `db/videos.jsonl` | Indexed media paths + hashes | Path/hash inventory | Path/hash inventory |
| `db/video_paths.tsv` | Tab-separated path list | Quick import list | Quick import list |
| `review/evidence-viewer.html` | Offline viewer (serverless) | Examiner preview only | Examiner preview only |
| `reports/case-report.html` | Report-defensible HTML (print → PDF) | Attach as work product | Attach as work product |
| `evidence/logs/*.jsonl` | Chained audit logs (scan, validation, carve, tsk, anomaly) | Provenance appendix | Provenance appendix |
| `artifacts/clips/` | Exported MP4/AVI deliverables + `export-log.jsonl` | **Primary FIVE input** | Secondary |
| `artifacts/proxies/` | Lower-bitrate review proxies | Draft review only | — |
| `artifacts/thumbnails/` | JPEG thumbs | — | — |
| `artifacts/carved/` | Carved candidates (`candidate-unvalidated`) | Only after FrameTrace `validate-artifact` | Recovery triage |
| `artifacts/recovered/` | Inode recoveries (`candidate-unvalidated`) | Same as carved | Recovery triage |
| `manifest.sha256` / `package-manifest.json` | Checksums + file inventory | Custody transfer | Custody transfer |

Source evidence (mounted volumes / E01 / raw images) is **not** copied into the package by default. Keep originals write-blocked; the package references paths and hashes.

## 3. Amped FIVE workflow

Goal: open **validated deliverable clips** in FIVE for enhancement / measurement, not to re-index the whole disk.

1. In FrameTrace viewer, mark items (`important` / `needs_verification`), download selection JSON, run:
   ```powershell
   .\frametrace.exe export-batch C:\Cases\case-001 selection.json
   .\frametrace.exe validate-batch C:\Cases\case-001 selection.json
   .\frametrace.exe package-case C:\Cases\case-001
   ```
2. Copy the package (or just `artifacts/clips/` + logs + report) to the FIVE workstation.
3. Open each clip under `artifacts/clips/` in FIVE. Prefer files whose validation log status is `ffprobe-video-stream-confirmed`.
4. Do **not** treat FrameTrace `candidate-finding` / `candidate-unvalidated` labels as FIVE authenticity results. Those are triage flags only.
5. Paste the FrameTrace case id and clip SHA-256 from `export-log.jsonl` into the FIVE case notes so the two products stay linked.
6. If FIVE produces new stills/clips, store them **outside** the FrameTrace package (or as a new dated folder) and record their hashes in your custody notes. Re-running `package-case` without including those files keeps the original package hash-stable.

## 4. Magnet DVR Examiner (DME) workflow

Goal: send **container/image recovery problems** to DME when FrameTrace hits proprietary DVR limits.

1. Keep the original E01 / Ex01 / raw image available (FrameTrace `import-e01` output under `evidence/images/` if you exported raw).
2. Hand DME:
   - the forensic image (or vendor-supported direct disk),
   - FrameTrace `evidence/logs/e01-audit.jsonl` and `tsk-audit.jsonl` (what was already tried),
   - `artifacts/carved/` and `artifacts/recovered/` as “already attempted” candidates.
3. Do **not** expect FrameTrace DAV/Hikvision lanes to match DME’s proprietary filesystem parsers. If DME recovers additional files, copy them into a new FrameTrace case folder and `scan-folder` / `validate-artifact` so they enter the audit chain.
4. Use FrameTrace again for bilingual review UI, batch export, and the checksummed report package after DME recovery.

## 5. Language and claims

- Prefer the term **report-defensible** for FrameTrace outputs.
- Do not claim court-ready authenticity from FrameTrace alone.
- `qa anomalies` findings (`candidate-finding`) are examiner prompts, not Amped Authenticate substitutes.

## 6. Checklist

- [ ] `package-case` completed; `manifest.sha256` verified on the receiving PC  
- [ ] Write-protect / acquisition notes present in `case.json`  
- [ ] FIVE inputs limited to validated clips (or explicitly labeled candidates)  
- [ ] DME receives original image + FrameTrace attempt logs  
- [ ] Any external-tool outputs re-ingested or noted with hashes  
