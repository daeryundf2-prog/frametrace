# T3 Distributable Redaction Gate Review

recommendation: REJECT

originalIntent: Default generated shared report, review, viewer, and package artifacts must omit absolute workstation/source paths and file URLs. Explicit `--include-full-paths` must retain full paths and emit privacy disclosure metadata/artifacts. Internal SQLite/audit provenance in the active case must remain intact.

desiredOutcome: A normal operator can run `make-report`, `make-review`, and `package-case` without leaking workstation-local paths or `file://` URLs into distributable artifacts unless they explicitly opt in.

userOutcomeReview: FAIL. The SQLite review and HTML package blockers appear fixed in the current implementation/evidence, but default `package-case` still leaks full workstation/source paths and encoded `file://` URLs inside the copied package SQLite database.

blockers:

1. Default package SQLite copy still leaks full path provenance.
   - `src/distributable_redaction.rs:217` redacts only `videos.source_path` in the copied SQLite database.
   - `src/case_db/schema.rs:31` through `src/case_db/schema.rs:55` show the same `videos` table also stores `file_url` and `record_json`.
   - Fresh read-only-to-repo smoke check on 2026-06-24 created a temp SQLite case, ran `init-case`, `scan-folder`, `make-report`, `make-review`, and default `package-case`, then queried `package/db/case.db`.
   - The copied package row was `vid_000001|[redacted-source:vid_000001]|file:///private/tmp/frametrace-t3-gate-sqlite-leak.../Client%20ACME%20Source/...|43|144`, proving `source_path` was redacted while `file_url` and `record_json` still contained the temp root / `file://` payload.
   - `strings package/db/case.db` from that same smoke check found the unredacted `/private/tmp/frametrace-t3-gate-sqlite-leak.../Client ACME Source/...` and the embedded raw `record_json` with both `source_path` and `file_url`.

2. The retained SQLite package evidence is too narrow and misses the leaked columns.
   - `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/manual-sqlite-redaction-pass-v2.md:67` through `:71` records a package-SQLite redaction check, but it proves only that a redacted label exists and that the searched source token was absent.
   - The artifact does not show a SQLite query over `videos.file_url` or `videos.record_json`, and the current code path leaves both unredacted.

3. Required completed code-review/slop coverage artifact remains absent.
   - `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/cleanup-receipt.md:27` through `:48` lists retained evidence artifacts and still contains no code-review report covering `remove-ai-slops` overfit/slop criteria or `programming` criteria.
   - `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/cleanup-receipt.md:25` says `subagent_gate_reviewer=running_at_cleanup_receipt_time`, so the retained task evidence still does not support a completed reviewer pass after implementation.

checkedArtifactPaths:

- `src/distributable_redaction.rs`
- `src/case_db/schema.rs`
- `src/review_bundle.rs`
- `src/report.rs`
- `src/html_report.rs`
- `src/cli/commands.rs`
- `src/cli/mod.rs`
- `src/cli/handlers.rs`
- `src/package.rs`
- `gui/evidence-viewer/app.js`
- `docs/RECOVERY_BOUNDARIES.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/manual-sqlite-redaction-pass-v2.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/manual-sqlite-opt-in-disclosure.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/verification-gates-rerun.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/dirty-worktree-snapshot-after-fix.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/cleanup-receipt.md`
- `.omo/evidence/frametrace-production-hardening-review-plan-task-03-redaction-gate-review.md`

exactEvidenceGaps:

- No evidence artifact queries copied package SQLite `videos.file_url` and `videos.record_json` for path leaks.
- No code-review report artifact explicitly covering `remove-ai-slops` overfit/slop checks and `programming` criteria after the SQLite fix.

directGateChecks:

- Loaded and applied `omo:remove-ai-slops` criteria for false confidence, missing behavioral coverage, overfit tests, and unresolved slop.
- Loaded and applied `omo:programming` criteria for Rust/JavaScript code and evidence sufficiency.
- Inspected current implementation, current task artifacts, and ran a fresh temp-case smoke check that reproduced the copied SQLite DB leak in default package output.
