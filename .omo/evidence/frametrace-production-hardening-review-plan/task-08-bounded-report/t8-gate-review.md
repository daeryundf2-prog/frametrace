recommendation: REJECT

# T8 Gate Review - Bounded Report Generation

## originalIntent

T8 was intended to move default `make-report` away from loading the full `db/video_index.json` into one HTML table. The expected result is a bounded SQLite-backed report path with aggregate counts, a bounded top-N video appendix, explicit truncation/appendix language, and no regression for small-case report assertions.

## desiredOutcome

- Default report generation uses SQLite query contracts, not full legacy JSON.
- Large reports document that the embedded appendix is bounded/truncated and point users to bounded inventory/review follow-up commands.
- Small SQLite-backed reports still include expected video rows.
- Legacy JSON-only cases fail clearly with SQLite/migration guidance instead of producing a misleading report.

## userOutcomeReview

The user-visible behavior is mostly implemented: `make_report` calls `case_db::bounded_report_index_json` and errors when it returns `None`, the report renderer exposes `sample_limit`, `videos_truncated`, and visible appendix-boundary text, and focused integration tests plus final 100k manual artifacts support the bounded behavior.

However, approval is blocked by process and direct anti-slop findings required by the gate contract:

1. The required independent/code-review report with explicit programming-skill and remove-ai-slops/overfit coverage is absent. The doneclaim only records a self-review fallback and says no child reviewer agents were spawned.
2. Direct slop pass found a redundant SQLite aggregate query in new T8 production code: `confirmed_count(&conn)?` is called twice in the same JSON formatting block.

## blockers

### B1 - Missing required code-review/slop coverage artifact

Evidence:

- No standalone code-review report artifact was present in `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/`.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/doneclaim.json:100-103` states: `self-review completed` and notes that `review-work` was loaded but no child reviewer agents were spawned.
- I did not find a report that explicitly covers the `omo:programming` perspective plus remove-ai-slops overfit/slop criteria: excessive/useless tests, deletion-only tests, tautological tests, implementation-mirroring tests, needless abstraction, parsing/normalization bloat, hidden-cost slop, and oversized touched-file handling.

Why this blocks:

The final gate instruction requires rejecting when report coverage is absent, missing, or unsupported. The doneclaim is not a substitute for a code-review report with explicit criterion coverage.

Minimal fix guidance:

- Add a T8 code-review report artifact under the T8 evidence directory.
- The report must explicitly show `omo:programming` and `omo:remove-ai-slops`/overfit coverage, including tests and production code, and must cite concrete paths/lines.
- Re-run the focused T8 tests and diff check after any fix.

### B2 - Direct anti-slop finding: duplicated SQLite count query

Evidence:

- `src/case_db/report_summary.rs:65-66` calls `confirmed_count(&conn)?` twice while rendering one report summary object.
- This is hidden-cost slop under the remove-ai-slops performance-equivalence category: a redundant DB aggregate query in a large-case report path.

Why this blocks:

The gate requires rejection if the direct slop pass finds unresolved slop. This is small but real, and it sits in new T8-owned production code.

Minimal fix guidance:

- Bind once before the `format!`, for example:
  - `let confirmed = confirmed_count(&conn)?;`
  - use `confirmed` for `confirmed_count`
  - use `page.total_rows.saturating_sub(confirmed)` for `candidate_count`
- Re-run:
  - `cargo test --locked --test cli_bounded_report -- --nocapture`
  - `cargo test --locked make_report -- --nocapture`
  - `git diff --check`

## sourceEvidence

Checked source paths:

- `src/case_db/mod.rs`
- `src/case_db/report_summary.rs`
- `src/cli/handlers.rs`
- `src/report.rs`
- `tests/cli_bounded_report.rs`

Acceptance evidence found:

- `src/cli/handlers.rs:335-340` uses `case_db::bounded_report_index_json(case_dir)?` and returns the SQLite-required migration guidance when no DB-backed summary is available. I did not find a default `make_report` fallback to reading `db/video_index.json`.
- `src/case_db/report_summary.rs:8-28` sets a 100-row report sample limit and routes through `list_inventory` with that page size.
- `src/case_db/report_summary.rs:53-75` emits `source: "sqlite-bounded-report"`, aggregate fields, `sample_limit`, `videos_truncated`, `report_summary.sample_count`, `report_summary.total_rows`, and bounded follow-up commands.
- `src/report.rs:179-184` derives `videos`, `reportSummary`, `videoTotal`, `videoSampleLimit`, and `videosTruncated` from the bounded summary payload.
- `src/report.rs:281-283` renders visible bounded/truncation/appendix-boundary language and includes inventory/review follow-up commands.
- `tests/cli_bounded_report.rs:19-43` covers small SQLite-backed report rows even when legacy JSON also exists.
- `tests/cli_bounded_report.rs:45-79` covers SQLite-only bounded report behavior without `db/video_index.json`.
- `tests/cli_bounded_report.rs:81-99` covers legacy JSON-only rejection and no report write.

Slop/overfit direct-pass notes:

- Tests are not deletion-only or removal-only. They exercise CLI-observable behavior.
- Tests are not purely implementation-mirroring; they invoke the real binary and inspect generated report output.
- Some assertions are broad (`contains("bounded")`, `contains("700")`), but the suite also asserts no final row and bounded row count. Manual 100k evidence adds stronger boundary evidence.
- Unresolved production slop remains at `src/case_db/report_summary.rs:65-66`.
- Touched files `src/cli/handlers.rs` and `src/report.rs` are oversized by programming-skill criteria, but these were pre-existing shared files in a broader dirty branch. I did not make that the primary blocker; the missing review report should explicitly justify this touched-file risk.

## commandsRun

Fresh gate commands:

- `cargo test --locked --test cli_bounded_report -- --nocapture`
  - Result: PASS, 3 passed.
  - Transcript: `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/gate-cli-bounded-report.txt`
- `cargo test --locked make_report -- --nocapture`
  - Result: PASS, 5 tests matched and passed across the filtered suite.
  - Transcript: `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/gate-make-report-filter.txt`
- `git diff --check`
  - Result: PASS.
  - Transcript: `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/gate-git-diff-check.txt`
- Fresh cleanup check for T8 temp roots and report/generator processes.
  - Result: no `/tmp/frametrace-t8-*` roots and no matching running processes.
  - Transcript: `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/gate-cleanup-check.txt`

I did not run full `cargo test --locked` during this gate because the requested priority was focused bounded verification and diff/source inspection. I inspected the worker's final full-suite transcript instead: `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/cargo-test-locked-final.txt`, which ends with `EXIT_CODE=0`.

## evidencePathsChecked

- `.omo/plans/frametrace-production-hardening-review-plan.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/doneclaim.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/notepad.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/t8-owned-diff.patch`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/t8-owned-changes-final.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/red-cli-bounded-report.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/green-cli-bounded-report-after-review.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/focused-make-report-tests.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/cargo-test-locked-final.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/cargo-clippy-final.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/cargo-fmt-check-final.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/git-diff-check-final.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/manual-100k-case-report-final.html`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/manual-100k-checks-final.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/manual-100k-happy-path-final.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/manual-100k-report-excerpt-final.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/manual-legacy-json-only-final.stderr`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/cleanup-receipt-final.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/gate-cli-bounded-report.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/gate-make-report-filter.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/gate-git-diff-check.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/gate-cleanup-check.txt`

## evidenceGaps

- Required code-review report with explicit `omo:programming` and `omo:remove-ai-slops`/overfit coverage is missing.
- The branch is broadly dirty with many non-T8 files; I isolated T8 paths, but the shared-file diff artifact is not a clean T8-only patch.
- Intermediate stale manual artifacts remain and contain earlier failed values, for example non-final 100k report files. The final `*-final.*` artifacts support the worker's corrected claim, but stale intermediate files increase review risk.
- Full `cargo test --locked` was not rerun by this gate; I relied on the worker transcript after fresh focused tests passed.

## cleanupAssessment

Worker cleanup receipt:

- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/cleanup-receipt-final.txt`
- It records no remaining T8 tmp roots or processes.

Fresh gate cleanup check:

- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/gate-cleanup-check.txt`
- It found no `/tmp/frametrace-t8-*` directories and no `generate_100k_sqlite_case` or `frametrace make-report` processes.

cleanup: PASS

## AdversarialVerify

```json
{
  "verdict": "needs-fix",
  "confidence": 0.88,
  "classes": {
    "malformed_input": {
      "result": "probed",
      "evidence": "Fresh cli_bounded_report run passed the legacy JSON-only rejection test; stderr guidance also present in manual-legacy-json-only-final.stderr."
    },
    "dirty_worktree": {
      "result": "probed",
      "evidence": "git status shows a broadly dirty branch with many non-T8 changes and untracked .omo state; T8 source/evidence paths were isolated for this review."
    },
    "stale_state": {
      "result": "probed",
      "evidence": "Older non-final 100k artifacts contain stale pre-fix values, but final artifacts contain bounded/fixed values. Stale intermediate artifacts remain as a review risk."
    },
    "misleading_success_output": {
      "result": "probed",
      "evidence": "Did not rely on doneclaim prose alone; ran fresh focused tests, diff check, source inspection, and direct final HTML checks for sample_limit/videos_truncated/boundary text."
    },
    "flaky_tests": {
      "result": "probed",
      "evidence": "Focused cli_bounded_report and make_report-filter tests passed fresh and match worker final transcripts."
    },
    "hung_or_long_commands": {
      "result": "probed",
      "evidence": "Fresh commands completed quickly; worker 100k manual run transcript reports 0.23s and no process remained in cleanup probe."
    },
    "prompt_injection": {
      "result": "ruled_out_for_scope",
      "evidence": "Report generation consumes local case/SQLite data, not natural-language instructions. HTML rendering uses json_for_script and escapeHtml for dynamic display paths."
    },
    "cancel_resume": {
      "result": "ruled_out_for_scope",
      "evidence": "T8 did not add resumable report jobs or cancellation semantics."
    },
    "repeated_interruptions": {
      "result": "ruled_out_for_scope",
      "evidence": "No interruption/resume mechanism is part of the changed report-generation path."
    }
  }
}
```

## finalVerdict

REJECT. The T8 behavior is close and focused verification passed, but approval is blocked by the missing required review/slop coverage artifact and the unresolved duplicate SQLite count query in new T8 production code.
