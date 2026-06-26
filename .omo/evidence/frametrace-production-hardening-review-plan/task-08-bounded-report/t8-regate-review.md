recommendation: APPROVE

# T8 Re-Gate Review - Bounded Report Needs-Fix Pass

## originalIntent

T8 was intended to make default `make-report` safe for large cases by replacing full `db/video_index.json` report loading with a SQLite-backed bounded summary. The user-visible report should still preserve small-case report usefulness, clearly disclose that large video appendices are bounded/truncated, and direct operators to bounded inventory/review follow-up commands.

## desiredOutcome

- Default `make-report` uses SQLite query contracts rather than embedding every legacy JSON video row.
- Large reports show aggregate counts, at most a bounded sample appendix, truncation/boundary language, and follow-up commands.
- Small SQLite-backed reports still show expected video rows.
- Legacy JSON-only cases fail clearly with SQLite/migration guidance.
- The two prior gate blockers are resolved: missing programming/remove-ai-slops review coverage and duplicate SQLite aggregate work.

## userOutcomeReview

APPROVE. The shipped artifact satisfies the intended user outcome from the T8 evidence I inspected. The actual bounded report source now binds `confirmed_count(&conn)?` once at `src/case_db/report_summary.rs:53` and reuses it at lines 66-67 for both `confirmed_count` and `candidate_count`. The new review artifact `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/programming-remove-ai-slops-review.md` explicitly covers Rust programming criteria, remove-ai-slops categories, overfit/test-slop checks, and the oversized-module disposition.

Fresh re-gate checks passed:

- `git diff --check`: PASS, `EXIT_CODE=0`.
- `cargo test --locked --test cli_bounded_report -- --nocapture`: PASS, 3 passed.
- `cargo test --locked make_report -- --nocapture`: PASS, bounded report tests plus report output-policy tests passed.
- cleanup probe: PASS, no `/tmp/frametrace-t8-*` roots and no matching T8 report/generator processes.

Prior manual evidence remains consistent with the desired behavior: the 100k SQLite-only report has no `db/video_index.json`, reports `video_count:100000`, `sample_limit:100`, `videos_truncated:true`, embeds rows through `000099.mp4`, and omits later rows; the legacy JSON-only run exits 1 with the SQLite migration guidance and writes no report.

## blockers

None.

## directProgrammingAndSlopPass

Checked against loaded `omo:programming` and `omo:remove-ai-slops` criteria:

- Rust error/resource discipline: PASS. T8 production code returns `Result`, propagates SQLite errors with context, uses existing DB helpers, and adds no `unsafe`.
- Escape hatches: PASS for the reviewed production path. No production `unwrap`/`expect` in `src/case_db/report_summary.rs`.
- Hidden-cost slop: PASS after fix. The prior duplicate `confirmed_count(&conn)?` aggregate is removed.
- Over-defensive code: PASS. Missing SQLite is a boundary condition surfaced as an explicit migration error; no fallback to full legacy JSON was reintroduced.
- Needless abstraction: PASS. `report_summary.rs` is a focused module with one public bounded-report entry point and small private helpers.
- Dead/debug code: PASS in T8-reviewed source. No debug prints or unused obvious branches found.
- Duplication: PASS for the prior blocker; no repeated aggregate query remains.
- Performance equivalence: PASS. Default report path uses aggregate queries plus a 100-row inventory page, not full row embedding.
- Missing tests / overfit test slop: PASS. `tests/cli_bounded_report.rs` drives the real CLI and asserts observable output, not helper calls. Tests are not deletion-only, tautological, or pure implementation mirrors.
- Oversized module: PASS for the new module. `src/case_db/report_summary.rs` is 146 pure LOC, below the 250 LOC defect threshold. Existing oversized shared files are broader branch context, not introduced by the needs-fix pass.

## commands

Fresh re-gate transcripts written:

- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/regate-git-diff-check.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/regate-git-status.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/regate-aggregate-probe.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/regate-cleanup-check.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/regate-cli-bounded-report.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/regate-make-report-filter.txt`

Executor fix transcripts checked:

- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/fix-cli-bounded-report.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/fix-make-report-filter.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/fix-cargo-fmt-check.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/fix-git-diff-check.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/fix-cleanup-receipt.txt`

Prior T8 evidence checked:

- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/t8-gate-review.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/doneclaim.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/fix-doneclaim.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/manual-100k-checks-final.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/manual-100k-happy-path-final.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/manual-100k-case-report-final.html`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/manual-100k-report-excerpt-final.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/manual-legacy-json-only-final.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/manual-legacy-json-only-final.stderr`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/cargo-test-locked-final.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/cargo-clippy-final.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/cargo-fmt-check-final.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/git-diff-check-final.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/notepad.md`

## sourceEvidence

Checked source paths:

- `src/case_db/report_summary.rs`
- `src/case_db/mod.rs`
- `src/cli/handlers.rs`
- `src/report.rs`
- `tests/cli_bounded_report.rs`

Key source evidence:

- `src/cli/handlers.rs:335-340` calls `case_db::bounded_report_index_json(case_dir)?` and errors with SQLite migration guidance if absent.
- `src/case_db/report_summary.rs:8` sets `REPORT_VIDEO_SAMPLE_LIMIT` to 100.
- `src/case_db/report_summary.rs:21-28` requests a bounded inventory page using that limit.
- `src/case_db/report_summary.rs:53` binds `confirmed_count(&conn)?` once.
- `src/case_db/report_summary.rs:66-67` reuses the binding for `confirmed_count` and `candidate_count`.
- `src/case_db/report_summary.rs:55-61` emits source, counts, sample limit, truncation flag, and follow-up commands.
- `src/report.rs:179-184` reads bounded report fields from the summary payload.
- `src/report.rs:281-283` renders visible bounded/truncation/appendix-boundary language and inventory/review commands.
- `tests/cli_bounded_report.rs:19-99` covers small SQLite report preservation, SQLite-only bounded large report behavior, and JSON-only migration failure.

## evidenceGaps

No blocking evidence gaps remain.

Residual non-blocking context:

- Full `cargo test --locked` was not rerun by this re-gate after the needs-fix pass. I checked the prior final full-suite transcript (`EXIT_CODE=0`) and ran fresh focused T8/report checks because the needs-fix pass changed only the aggregate binding and review artifact.
- The repository remains broadly dirty with unrelated files and untracked `.omo` state. I isolated T8 source/evidence paths and treated the dirty worktree as an adversarial class rather than a blocker.
- Older non-final manual artifacts still exist from earlier iterations. I used final artifacts and fresh re-gate transcripts for the approval decision.

## cleanupAssessment

PASS. Executor cleanup transcript showed no T8 temp roots/processes. Fresh re-gate cleanup transcript at `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/regate-cleanup-check.txt` also showed no `/tmp/frametrace-t8-*` roots and no matching `generate_100k_sqlite_case` or `frametrace make-report` processes.

## AdversarialVerify

```json
{
  "verdict": "confirmed",
  "confidence": 0.91,
  "classes": {
    "malformed_input": {
      "result": "probed",
      "evidence": "Fresh cli_bounded_report includes the legacy JSON-only rejection case; prior manual legacy artifact exits 1 with SQLite migration guidance and no report write."
    },
    "dirty_worktree": {
      "result": "probed",
      "evidence": "regate-git-status.txt records a broadly dirty branch, including unrelated modified/untracked files. T8 files and evidence were inspected directly; fresh tests passed in this dirty state."
    },
    "stale_state": {
      "result": "probed",
      "evidence": "Prior rejected t8-gate-review.md was checked against fix-doneclaim.json, the new review artifact, actual source, and fresh regate transcripts. Decision uses final/manual and regate artifacts, not stale intermediate files."
    },
    "misleading_success_output": {
      "result": "probed",
      "evidence": "Did not rely on doneclaim prose. Verified source lines, aggregate probe, fresh tests, diff-check, cleanup probe, and final manual HTML/legacy artifacts."
    },
    "flaky_tests": {
      "result": "probed",
      "evidence": "Focused tests passed in executor fix transcripts and passed again in fresh regate-cli-bounded-report.txt and regate-make-report-filter.txt."
    },
    "hung_or_long_commands": {
      "result": "probed",
      "evidence": "Fresh focused cargo commands completed quickly; prior manual 100k run recorded 0.23s and cleanup found no lingering process."
    },
    "prompt_injection": {
      "result": "ruled_out_for_scope",
      "evidence": "The changed path consumes local manifest/SQLite/report data, not natural-language instructions. Dynamic report display is JSON-fed and escaped in the renderer."
    },
    "cancel_resume": {
      "result": "ruled_out_for_scope",
      "evidence": "T8 report generation does not introduce resumable jobs or cancellation state."
    },
    "repeated_interruptions": {
      "result": "ruled_out_for_scope",
      "evidence": "No interruption/retry protocol is part of the changed make-report path; cleanup probes found no leftover temp roots or report processes."
    }
  }
}
```

## confidence

High. The exact prior blockers are resolved, focused report behavior is freshly green, prior full/manual evidence remains consistent, and no direct programming/remove-ai-slops blocker remains in the T8 scope.
