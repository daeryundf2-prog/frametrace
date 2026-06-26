# Gate Review - FrameTrace T4 Audit Chain Required

recommendation: APPROVE

blockers:
- none

originalIntent:
Make missing required audit chains block `qa report-defense`. Required-vs-optional chains must be derived from the current case surface and report claims. The user-visible states must include `missing`, `empty`, `valid`, `tampered`, `unsupported`, and `not-applicable`, and optional unsupported/not-applicable chains must be visible without masquerading as pass.

desiredOutcome:
From the user's perspective, `frametrace qa report-defense <case>` fails closed when a report, validation claim, derived artifact, export, carving result, or recovered filesystem output depends on an audit chain that is missing, empty, or tampered. Cases without that surface still show typed optional states and can pass.

userOutcomeReview:
Confirmed for T4. Current `src/qa_report_defense.rs` makes validation report claims require `evidence/logs/validation-log.jsonl` and recovered filesystem artifacts/report claims require `evidence/logs/tsk-audit.jsonl`. Fresh CLI probes against the retained validation and recovered-filesystem cases both exited 1 with exact missing-chain blocker messages. The checklist output marked the implicated chain `required=yes` and left unrelated absent chains as `unsupported` or `not-applicable`.

Prior blocker resolution:
- Missing validation audit chain false pass: resolved. Fresh CLI probe returned exit code 1 with `validation: evidence/logs/validation-log.jsonl [missing]`.
- Missing recovered filesystem audit chain false pass: resolved. Fresh CLI probe returned exit code 1 with `filesystem recovery: evidence/logs/tsk-audit.jsonl [missing]`.
- Proxy-only overfit: resolved for T4. Focused tests now include proxy, validation-claim, and recovered-filesystem missing-chain cases plus empty, tampered, valid, and optional-state checks.
- Stale diff and size-gate disposition: current live code was inspected directly. Size remains explicit T11 refactor debt, documented in `size-gate-disposition.md`; I do not treat it as a T4 behavioral blocker after the user's instruction to decide T4 completion only.

checked artifact paths:
- `.omo/plans/frametrace-production-hardening-review-plan.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/stop-hook-direct-verification-6/direct-verification-summary.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/stop-hook-direct-verification-6/focused-report-defense.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/stop-hook-direct-verification-6/validation-claimed-missing-log.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/stop-hook-direct-verification-6/recovered-filesystem-missing-tsk-log.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/stop-hook-direct-verification-6/git-diff-check.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/stop-hook-direct-verification-6/target-cargo-fmt-check.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/stop-hook-direct-verification-6/clippy.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/final-manual-qa-happy.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/final-manual-qa-failure.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/t4-code-review-slop-report.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/size-gate-disposition.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/adversarial-summary.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/cleanup-receipt.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/fix-cleanup-receipt.md`
- `src/qa_report_defense.rs`
- `src/audit.rs`
- `src/qa_tests.rs`
- `tests/media_contract.rs`

fresh verification:
- `cargo test --locked report_defense_ -- --nocapture`: PASS, 10 report-defense tests passed.
- `git diff --check`: PASS.
- `cargo fmt -- --check src/qa_report_defense.rs src/qa_tests.rs src/audit.rs tests/media_contract.rs`: PASS.
- `target/debug/frametrace qa report-defense .../validation-claimed-case --output-dir /tmp/frametrace-t4-gate-validation-output`: expected FAIL, exit code 1, exact validation missing-chain blocker.
- `target/debug/frametrace qa report-defense .../recovered-filesystem-case --output-dir /tmp/frametrace-t4-gate-recovered-output`: expected FAIL, exit code 1, exact filesystem recovery missing-chain blocker.
- Checklist inspection under `/tmp`: validation and filesystem recovery cases rendered `required=yes` for the implicated chain and listed `## Audit Chain Failures`.

skill and slop review:
- `omo:programming` and its Rust reference were loaded. Current T4 logic uses typed states and exhaustive matches for the audit-chain state mappings. It still uses report text markers to infer claims, but this is scoped to generated report surfaces and is backed by regression tests for the prior false-pass classes.
- `omo:remove-ai-slops` was loaded. Direct overfit/slop pass found no deletion-only, tautological, implementation-mirroring, or proxy-only tests remaining for T4. The new tests assert observable report-defense errors/checklist states for proxy, validation, and recovered-filesystem surfaces.
- The supplied code-review report contains the required programming and remove-ai-slops/overfit coverage, but it is pre-fix and correctly records the old blockers. Current direct verification supersedes those specific findings for T4.

adversarial_classes:
- malformed_input: confirmed by existing tampered required audit-log test and fresh 10-test focused run.
- stale_state: confirmed by missing required proxy artifact evidence and fresh validation/recovered-filesystem missing-log probes.
- dirty_worktree: broad dirty worktree remains; T4-relevant files and evidence were inspected directly, and unrelated changes were not reverted.
- misleading_success_output: confirmed by checking exit codes and exact blocker keys, not success prose.
- flaky_tests: focused report-defense suite passed fresh with all 10 tests.
- hung_or_long_commands: fresh focused commands completed quickly; no running exec sessions remain.

exact evidence gaps:
- none for T4 completion.
- external/non-blocking: full-suite evidence records an unrelated T6 `tests/tool_policy_api.rs::resolved_external_tool_cannot_be_forged_by_downstream_crates` failure and unrelated full-format drift in `tests/tool_policy_api.rs`.
- external/non-blocking: `src/audit.rs`, `src/qa_report_defense.rs`, and `src/qa_tests.rs` remain over the programming 250 pure-LOC threshold; `size-gate-disposition.md` assigns the split to T11.

cleanup:
Temporary `/tmp/frametrace-t4-gate-validation-output` and `/tmp/frametrace-t4-gate-recovered-output` probe directories were removed. No source changes were made by this gate pass except this required evidence report update.
