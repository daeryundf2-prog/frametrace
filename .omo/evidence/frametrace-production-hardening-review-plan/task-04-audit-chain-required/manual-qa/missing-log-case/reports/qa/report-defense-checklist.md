# Report Defensibility Checklist

- [PASS] case manifest: `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/manual-qa/missing-log-case/case.json`
- [PASS] case database: `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/manual-qa/missing-log-case/db/case.db`
- [PASS] video JSON index: `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/manual-qa/missing-log-case/db/video_index.json`
- [PASS] video JSONL index: `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/manual-qa/missing-log-case/db/videos.jsonl`
- [PASS] video path TSV: `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/manual-qa/missing-log-case/db/video_paths.tsv`
- [PASS] case report: `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/manual-qa/missing-log-case/reports/case-report.html`

## Audit Chain Validation

- [not-applicable] clip export: `artifacts/clips/export-log.jsonl` required=no reason=no reported artifacts for this chain entries=- last=- error=audit log is not present
- [missing] proxy: `artifacts/proxies/proxy-log.jsonl` required=yes reason=case surface contains artifacts under artifacts/proxies entries=- last=- error=audit log is not present
- [not-applicable] thumbnail: `artifacts/thumbnails/thumbnail-log.jsonl` required=no reason=no reported artifacts for this chain entries=- last=- error=audit log is not present
- [not-applicable] frame capture: `artifacts/frames/frame-log.jsonl` required=no reason=no reported artifacts for this chain entries=- last=- error=audit log is not present
- [not-applicable] carving: `artifacts/carved/carve-log.jsonl` required=no reason=no reported artifacts for this chain entries=- last=- error=audit log is not present
- [unsupported] filesystem recovery: `evidence/logs/tsk-audit.jsonl` required=no reason=optional chain unsupported for this case surface entries=- last=- error=audit log is not present
- [not-applicable] validation: `evidence/logs/validation-log.jsonl` required=no reason=no reported artifacts for this chain entries=- last=- error=audit log is not present

## Audit Chain Failures

- proxy: artifacts/proxies/proxy-log.jsonl [missing] (audit log is not present)
