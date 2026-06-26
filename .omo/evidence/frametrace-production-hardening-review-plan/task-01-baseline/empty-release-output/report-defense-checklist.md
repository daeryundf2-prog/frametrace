# Report Defensibility Checklist

- [FAIL] case manifest: `/tmp/frametrace-t1-empty-case.bc1ba9/case.json`
- [FAIL] case database: `/tmp/frametrace-t1-empty-case.bc1ba9/db/case.db`
- [FAIL] video JSON index: `/tmp/frametrace-t1-empty-case.bc1ba9/db/video_index.json`
- [FAIL] video JSONL index: `/tmp/frametrace-t1-empty-case.bc1ba9/db/videos.jsonl`
- [FAIL] video path TSV: `/tmp/frametrace-t1-empty-case.bc1ba9/db/video_paths.tsv`
- [FAIL] case report: `/tmp/frametrace-t1-empty-case.bc1ba9/reports/case-report.html`

## Missing

- case manifest: /tmp/frametrace-t1-empty-case.bc1ba9/case.json
- case database: /tmp/frametrace-t1-empty-case.bc1ba9/db/case.db
- video JSON index: /tmp/frametrace-t1-empty-case.bc1ba9/db/video_index.json
- video JSONL index: /tmp/frametrace-t1-empty-case.bc1ba9/db/videos.jsonl
- video path TSV: /tmp/frametrace-t1-empty-case.bc1ba9/db/video_paths.tsv
- case report: /tmp/frametrace-t1-empty-case.bc1ba9/reports/case-report.html

## Audit Chain Validation

- [missing] clip export: `artifacts/clips/export-log.jsonl` entries=- last=- error=audit log is not present
- [missing] proxy: `artifacts/proxies/proxy-log.jsonl` entries=- last=- error=audit log is not present
- [missing] thumbnail: `artifacts/thumbnails/thumbnail-log.jsonl` entries=- last=- error=audit log is not present
- [missing] frame capture: `artifacts/frames/frame-log.jsonl` entries=- last=- error=audit log is not present
- [missing] carving: `artifacts/carved/carve-log.jsonl` entries=- last=- error=audit log is not present
- [missing] filesystem recovery: `evidence/logs/tsk-audit.jsonl` entries=- last=- error=audit log is not present
- [missing] validation: `evidence/logs/validation-log.jsonl` entries=- last=- error=audit log is not present
