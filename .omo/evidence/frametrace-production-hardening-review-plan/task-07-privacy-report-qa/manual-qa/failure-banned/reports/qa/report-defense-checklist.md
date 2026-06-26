# Report Defensibility Checklist

Machine-readable source: `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-07-privacy-report-qa/manual-qa/failure-banned/reports/qa/report-defense-report.json`

- [PASS] case manifest: `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-07-privacy-report-qa/manual-qa/failure-banned/case.json`
- [PASS] case database: `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-07-privacy-report-qa/manual-qa/failure-banned/db/case.db`
- [PASS] video JSON index: `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-07-privacy-report-qa/manual-qa/failure-banned/db/video_index.json`
- [PASS] video JSONL index: `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-07-privacy-report-qa/manual-qa/failure-banned/db/videos.jsonl`
- [PASS] video path TSV: `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-07-privacy-report-qa/manual-qa/failure-banned/db/video_paths.tsv`
- [PASS] case report: `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-07-privacy-report-qa/manual-qa/failure-banned/reports/case-report.html`

## Disallowed Report Claims

- reports/case-report.html: contains disallowed claim `court-grade`

## Audit Chain Validation

- [not-applicable] clip export: `artifacts/clips/export-log.jsonl` required=no reason=no reported artifacts for this chain entries=- last=- error=audit log is not present
- [not-applicable] proxy: `artifacts/proxies/proxy-log.jsonl` required=no reason=no reported artifacts for this chain entries=- last=- error=audit log is not present
- [not-applicable] thumbnail: `artifacts/thumbnails/thumbnail-log.jsonl` required=no reason=no reported artifacts for this chain entries=- last=- error=audit log is not present
- [not-applicable] frame capture: `artifacts/frames/frame-log.jsonl` required=no reason=no reported artifacts for this chain entries=- last=- error=audit log is not present
- [not-applicable] carving: `artifacts/carved/carve-log.jsonl` required=no reason=no reported artifacts for this chain entries=- last=- error=audit log is not present
- [unsupported] filesystem recovery: `evidence/logs/tsk-audit.jsonl` required=no reason=optional chain unsupported for this case surface entries=- last=- error=audit log is not present
- [not-applicable] validation: `evidence/logs/validation-log.jsonl` required=no reason=no reported artifacts for this chain entries=- last=- error=audit log is not present
