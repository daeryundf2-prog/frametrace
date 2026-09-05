# Windows Smoke Validation Receipt

Date: 2026-09-05  
Scope: WINDOWS_IMPLEMENTATION_HANDOFF Phase 2 + Phase 3 (Korean path)

## Phase 2 — synthetic MP4 workflow

Commands run against `target/release/frametrace.exe`:

1. `init-case C:\Temp\frametrace-case`
2. `scan-folder ... C:\Temp\frametrace-source --hash`
3. `validate-artifact ... vid_000001` → `ffprobe-video-stream-confirmed`
4. `qa anomalies` → 0 findings (clean sample)
5. `make-review` / `make-report` / `package-case`

Artifacts present:

- `db/case.db`, `db/video_index.json`
- `evidence/logs/validation-log.jsonl`, `evidence/logs/anomaly-log.jsonl`
- `review/index.html`, `review/evidence-viewer.html`
- `reports/case-report.html` (includes anomaly section)

## Phase 3 — Korean path

- Case: `C:\Temp\frametrace-한글\case`
- Source: `...\source\샘플.mp4`
- Result: scan + make-review OK; viewer and index written.

## Not run in this pass

- Real client evidence dry-run (Phase 5)
- E01/libewf on this machine (optional tools)
- 1,000-file scale viewer timing
