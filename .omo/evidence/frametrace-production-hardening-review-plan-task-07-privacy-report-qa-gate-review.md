# T7 Gate Review

recommendation: REJECT

## originalIntent
T7 was intended to add executable privacy and report-defense QA surfaces for FrameTrace. The user-visible result should be `qa privacy-review`, `qa report-defense`, and `qa release` producing and consuming typed JSON evidence for privacy, report defensibility, banned wording, disclosure/leakage, and distinct QA states.

## desiredOutcome
- `reports/qa/privacy-review.json`, `reports/qa/report-defense-report.json`, and `reports/qa/report-defense-checklist.md` are generated from executable QA checks.
- `qa release` runs current privacy/report-defense checks and consumes typed JSON inputs, not stale markdown-only pass/fail evidence.
- Happy privacy/report-defense case passes.
- Banned wording and full-path leakage fail with exact keys `banned_legal_wording` and `full_path_leakage`.
- Failed, skipped, partial, unsupported, and not-applicable states remain distinct.
- Evidence cleanup and review artifacts are trustworthy despite the worker shutdown/running-state concern.

## userOutcomeReview
Product behavior is mostly present and fresh verification passed:
- `src/cli/commands.rs` exposes `qa privacy-review`.
- `src/cli/qa_cmd.rs` routes `qa privacy-review` to `privacy_review_check`.
- `src/qa_report_defense.rs` writes `privacy-review.json`, `report-defense-report.json`, and generated `report-defense-checklist.md`.
- `src/qa_release.rs` runs current privacy/report-defense checks and reads typed JSON before marking release checks.
- Manual QA artifacts show happy pass plus failure keys `banned_legal_wording` and `full_path_leakage`.
- Fresh verification passed for `cargo test --locked qa_tests:: -- --nocapture`, `cargo test --locked release_rejects_stale_report_defense_json_when_current_check_errors -- --nocapture`, `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo test --locked`, and `git diff --check`.

The gate still does not pass because evidence hygiene and slop-review requirements are not satisfied. The cleanup receipt claims no persistent temp cases outside evidence, but fresh inspection found T7-era temp case directories under `$TMPDIR`. The T7 code/slop review also does not explicitly cover the required overfit/slop criteria, and a direct programming/remove-ai-slops pass finds oversized touched Rust files without a T7 resolution.

## blockers
1. Cleanup receipt is contradicted by live filesystem evidence. `.omo/evidence/frametrace-production-hardening-review-plan/task-07-privacy-report-qa/manual-qa/cleanup-receipt.txt` says `persistent_temp_cases_outside_evidence=none`, but these T7 temp roots still exist outside evidence:
   - `/var/folders/32/z9wr5xpx5yzbyyynfrz1kjq80000gn/T/frametrace-privacy-leakage-test-39342`
   - `/var/folders/32/z9wr5xpx5yzbyyynfrz1kjq80000gn/T/frametrace-privacy-redacted-test-39342`
   - `/var/folders/32/z9wr5xpx5yzbyyynfrz1kjq80000gn/T/frametrace-release-stale-report-defense-test-37403`
   Their mtimes are 2026-06-24 19:43:31 and 19:57:28 KST, before this gate review and consistent with executor-era residue. This makes the worker shutdown/running-state concern material.
2. The required slop/overfit review coverage is missing. `code-slop-review.md` mentions over-defensive/stale-success and oversized files, but it does not explicitly cover excessive/useless tests, deletion-only tests, tautological tests, implementation-mirroring tests, unnecessary production extraction/parsing/normalization, or the full remove-ai-slops/programming criteria required by the gate.
3. Direct programming/remove-ai-slops pass found unresolved oversized touched Rust files:
   - `src/qa_report_defense.rs`: 800 pure LOC
   - `src/qa_release.rs`: 287 pure LOC
   - `src/qa_tests.rs`: 538 pure LOC
   - `tests/cli_lifecycle.rs`: 309 pure LOC
   - `tests/cli_windows_prereq.rs`: 264 pure LOC
   `code-slop-review.md` defers this to T11, but T11's plan text names other modules and does not explicitly include the new T7 QA modules.

## checkedArtifactPaths
- `.omo/plans/frametrace-production-hardening-review-plan.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-07-privacy-report-qa/doneclaim.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-07-privacy-report-qa/code-slop-review.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-07-privacy-report-qa/manual-qa/`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-07-privacy-report-qa/manual-qa/cleanup-receipt.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-07-privacy-report-qa/manual-qa/adversarial-classes.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-07-privacy-report-qa/hook-verification-claim-1/verification-transcript.log`
- `src/qa_report_defense.rs`
- `src/qa_release.rs`
- `src/qa.rs`
- `src/cli/commands.rs`
- `src/cli/qa_cmd.rs`
- `src/qa_tests.rs`
- `tests/cli_lifecycle.rs`
- `tests/cli_windows_prereq.rs`
- `tests/cli_smoke.rs`
- `.omo/start-work/ledger.jsonl`

## evidenceGaps
- Worker did not provide a final agent message after status check; ledger has a `task-completed` row with `session_id: codex:unknown`, but cleanup evidence is contradicted by persistent temp roots.
- Zero-byte evidence files exist. Most are harmless stdout-empty logs for commands that produced no stdout (`cargo fmt --check`, `git diff --check`, failed-command stdout captures), but their presence should be explicitly acknowledged by the fix worker.
- `manual-qa/adversarial-classes.json` claims stale-state before/after evidence, but only the final stale-state `privacy-review.json` was found; the passing pre-injection JSON snapshot was not present under the claimed path.

## freshCommands
- `cargo test --locked qa_tests:: -- --nocapture`: PASS, 19 passed, 0 failed.
- `cargo test --locked release_rejects_stale_report_defense_json_when_current_check_errors -- --nocapture`: PASS, 1 passed, 0 failed.
- `cargo fmt --all -- --check`: PASS, exit 0 with empty stdout.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: PASS, finished dev profile.
- `cargo test --locked`: PASS, 148 lib tests, 29 integration tests, 0 doc tests failed.
- `git diff --check`: PASS, exit 0 with empty stdout.
- `find .omo/evidence/frametrace-production-hardening-review-plan/task-07-privacy-report-qa -type f -empty -print | sort`: found harmless stdout-empty logs, but they are not documented in the existing cleanup receipt.
- `find "${TMPDIR:-/tmp}" -maxdepth 1 -type d -name 'frametrace-*' -print | sort`: found persistent T7 temp roots outside evidence.
- `ps -axo pid,command | rg '(/target/debug/frametrace|frametrace qa|cargo test|cargo clippy)'`: no persistent target/cargo process other than the probe itself.
- `lsof -nP -iTCP -sTCP:LISTEN | rg 'frametrace|target/debug'`: no matching listening ports.

## fixNeeded
Run a T7 fix/cleanup worker to remove or relocate the persistent T7 temp roots, regenerate a truthful cleanup receipt, add or update a T7 review artifact with explicit programming plus remove-ai-slops overfit/test-slop coverage, and resolve or explicitly plan-cover the oversized T7 QA modules before re-submitting for gate review.
