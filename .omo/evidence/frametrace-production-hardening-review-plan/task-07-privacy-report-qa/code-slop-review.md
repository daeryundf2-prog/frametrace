# T7 Code And Slop Review

Scope: T7 privacy/report-defense QA surfaces only.

Files inspected:
- `src/qa_report_defense.rs`
- `src/qa_release.rs`
- `src/qa.rs`
- `src/cli/commands.rs`
- `src/cli/qa_cmd.rs`
- `src/qa_tests.rs`
- `tests/cli_lifecycle.rs`
- `tests/cli_windows_prereq.rs`

Checks performed:
- Behavior locked by regression tests before and after product changes:
  - `privacy_review_rejects_full_path_leakage_with_exact_finding_key`
  - `privacy_review_passes_redacted_report_defensible_case`
  - `release_reads_executable_privacy_json_before_claiming_privacy_review`
  - `release_rejects_stale_report_defense_json_when_current_check_errors`
- Dead/debug code scan: no `dbg!`, `debugger`, `breakpoint`, commented-out trial code, or temporary debug prints found in the T7 diff.
- Over-defensive/stale-success review: `qa release` now fails closed when the current privacy/report-defense subcheck returns `Err`, even if an old typed JSON artifact says `passed: true`.
- Output-source review: `report-defense-checklist.md` is generated from executable state and points to `report-defense-report.json`; the markdown is not the pass/fail source.
- State vocabulary review: executable JSON includes distinct `pass`, `failed`, `partial`, `skipped`, `unsupported`, and `not-applicable` statuses.
- Legal wording review: generated allowed-language metadata uses `report-defensible`, `reproducible analysis record`, `validated against the defined QA corpus`, `candidate-unvalidated`, `unsupported`, and `known limitation`; banned phrases are only scanner terms or test fixtures.
- Programming/remove-ai-slops review: explicit overfit/test-slop, implementation-mirroring, tautology, needless-abstraction, deletion-opportunity, direct-output-evidence, and no-product-refactor coverage is recorded in `programming-remove-ai-slops-review.md`.

Oversized-file finding:
- `src/qa_report_defense.rs`, `src/qa_release.rs`, and `src/qa_tests.rs` exceed the 250 pure-LOC rule after accumulated T1-T7 work.
- This is a real maintainability risk, but the approved plan assigns module splitting to T11 after T8-T10 behavior locks are green.
- T7 therefore did not perform a broad split. The risk is recorded in `doneclaim.json` and should be closed by T11 rather than mixed into this behavior change.
- The T7 gate-fix disposition carries `src/qa_report_defense.rs`, `src/qa_release.rs`, `src/qa_tests.rs`, `tests/cli_lifecycle.rs`, and `tests/cli_windows_prereq.rs` into T11 scope in `t11-oversized-file-disposition.md`.

Verification artifacts:
- `red-stale-release-artifact-test.log`: failing-first proof for stale release JSON.
- `green-stale-release-artifact-test.log`: regression passes after fail-closed fix.
- `cargo-clippy-after-gate-fix.log`: clippy passes with warnings denied.
- `cargo-test-locked-after-gate-fix.log`: full locked test suite passes.
- `manual-stale-release-transcript.log`: real CLI stale artifact release probe passes.

Verdict: PASS for T7 scope, with oversized-file risk deferred to planned T11.
