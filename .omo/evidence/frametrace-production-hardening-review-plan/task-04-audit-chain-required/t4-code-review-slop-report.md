# T4 Code Review and Slop Report

Task: T4. Make missing required audit chains block report-defense
Plan: `.omo/plans/frametrace-production-hardening-review-plan.md`
Evidence root: `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/`
Notepad: `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/notepad.md`
Status: BLOCK
Recommendation: REQUEST_CHANGES

## Skill-Perspective Check

Ran. I loaded and applied:

- `omo:programming` main skill guidance and Rust reference: `/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/programming/SKILL.md` and `references/rust/README.md`
- `omo:remove-ai-slops` guidance: `/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/remove-ai-slops/SKILL.md`

Programming perspective result: violated. The implementation uses typed states and exhaustive Rust matches for the new enum mapping, but it still relies on string scanning over generated reports for required-chain discovery, misses validation and filesystem-recovery required-chain triggers, and leaves T4-touched modules over the 250 pure-LOC threshold.

Remove-ai-slops/overfit perspective result: violated. The production logic and tests are overfit to the proxy audit-chain case. The hook/manual QA reports claim broad T4 coverage, but the verified scenarios do not cover validation claims or recovered filesystem artifacts. This creates false confidence for the exact report-defense gap T4 is supposed to close.

## Evidence Reviewed

- Current source: `src/qa_report_defense.rs`, `src/audit.rs`, `src/qa_tests.rs`, `tests/media_contract.rs`
- T4 evidence: `hook-verification-3/hook-verification-report.md`, `hook-verification-3/focused-report-defense-tests.log`, `hook-verification-3/cargo-test-full.log`, `hook-verification-3/clippy.log`, `hook-verification-3/cli-happy.log`, `hook-verification-3/cli-failure.log`
- T4 manual/adversarial artifacts: `final-manual-qa-summary.txt`, `adversarial-summary.md`, `completion-audit/completion-audit-report.md`, `done-claim.json`, `review-gate-spawn-blocked.log`
- Existing gate review: `.omo/evidence/frametrace-production-hardening-review-plan-task-04-audit-chain-required-gate-review.md`
- Size evidence: `post-write-size-review.log`, `size-gate-disposition.md`

I also ran two fresh temporary CLI probes outside the repo tree and cleaned the scratch directories afterward:

- Validation stale-claim probe: `qa report-defense` returned `exit_code=0` while `reports/case-report.html` contained validation status claims and `evidence/logs/validation-log.jsonl` was absent. The checklist rendered validation as `[not-applicable]`.
- Filesystem recovery stale-claim probe: `qa report-defense` returned `exit_code=0` while `artifacts/recovered/filesystem/inode_1304.bin` existed, the report claimed the recovered artifact, and `evidence/logs/tsk-audit.jsonl` was absent. The checklist rendered filesystem recovery as `[unsupported]`.

## CRITICAL

None.

## HIGH

1. Missing validation audit chains can still pass report-defense.

`src/qa_report_defense.rs:90` to `src/qa_report_defense.rs:94` defines the validation chain with `artifact_dir: None`. Required-chain detection then only checks report claims when `artifact_dir` exists at `src/qa_report_defense.rs:210` to `src/qa_report_defense.rs:214`, and maps non-required absent validation logs to `NotApplicable` at `src/qa_report_defense.rs:232` to `src/qa_report_defense.rs:238`. The result is a false PASS when a stale report claims validation results but `evidence/logs/validation-log.jsonl` is missing. This violates T4's requirement that missing required chains fail when reports or validation claims depend on them.

2. Missing filesystem recovery audit chains can still pass report-defense.

`src/qa_report_defense.rs:84` to `src/qa_report_defense.rs:88` treats filesystem recovery as optional/unsupported unless `db/filesystem` is present or the report contains that directory string. Actual inode recovery writes outputs under `artifacts/recovered/filesystem` by default at `src/tsk.rs:188` to `src/tsk.rs:195`, and the report surface renders filesystem recovery records from the TSK audit chain at `src/report.rs:398` to `src/report.rs:410`. A recovered filesystem artifact can therefore be present and report-visible while a missing `evidence/logs/tsk-audit.jsonl` is rendered `[unsupported]` and the command exits 0. This violates the T4 recovery/audit-chain acceptance criteria.

3. The T4 tests and verification artifacts are overfit to proxy audit chains.

The new report-defense tests in `src/qa_tests.rs:178` to `src/qa_tests.rs:319` cover proxy missing, empty, valid, report-claimed proxy, optional states, and tampered proxy. They do not cover validation-log or TSK/recovered-filesystem required-chain triggers. Hook verification 3 likewise proves only proxy happy/failure CLI paths. This is a remove-ai-slops overfit issue: passing tests assert the implemented proxy constants rather than the broader required-chain contract.

## MEDIUM

1. Required-chain discovery is stringly and brittle.

`report_claims_chain` at `src/qa_report_defense.rs:275` to `src/qa_report_defense.rs:279` detects report claims by removing one log path string and searching HTML text for a directory substring. This is not parse-don't-validate. It can miss equivalent claims with different path spelling and can over-trigger on incidental text. For T4, this is secondary to the two false-pass blockers above, but it is the main maintainability risk in the current approach.

2. T4 increased module size beyond the programming threshold.

Current programming-threshold measurements using the documented pure-LOC command:

- `src/audit.rs`: 395 pure LOC, already oversized before T4 at 375 pure LOC
- `src/qa_report_defense.rs`: 308 pure LOC, increased from 114 pure LOC
- `src/qa_tests.rs`: 332 pure LOC, increased from 227 pure LOC

This violates the programming skill's 250 pure-LOC rule. I do not treat size alone as the primary T4 behavioral blocker, because T11 is the plan's module-split phase and the worktree is shared/dirty. However, the deferral must be explicit and T11 should include `src/qa_report_defense.rs`, `src/audit.rs`, and `src/qa_tests.rs`; T11's current named scope does not cover all three.

## LOW

1. The saved `final-t4-diff.patch` is stale relative to the live worktree.

The original `final-t4-diff.patch` omits later anchors such as `report_claims_chain` and the claimed-proxy test. A later `final-t4-diff-current.patch` exists, but the stale file remains in the evidence directory. This is not a product blocker by itself, but future reviewers should rely on the current worktree and `final-t4-diff-current.patch`, not the older patch.

## Acceptance Review

Typed states exist: PASS for the presence of `missing`, `empty`, `valid`, `tampered`, `unsupported`, and `not-applicable` in report-defense output.

Missing required chains fail: FAIL. Proxy missing chains fail, but validation and recovered-filesystem missing chains can still pass.

Optional unsupported/not-applicable visible and not pass-masked: PARTIAL. The optional states are visible and not labeled `[PASS]`, but filesystem recovery is incorrectly classified as unsupported in cases where a recovered filesystem artifact/report claim should make the TSK chain required.

Programming review: FAIL due false-pass state classification, stringly report-claim discovery, and oversized touched modules.

Remove-ai-slops/overfit review: FAIL due proxy-only coverage and broad PASS claims unsupported by validation/recovery scenarios.

## Blockers

- Fix required-chain detection so validation report claims make `evidence/logs/validation-log.jsonl` required and missing/empty/tampered states fail report-defense.
- Fix required-chain detection so recovered filesystem artifacts or report claims under `artifacts/recovered/filesystem` make `evidence/logs/tsk-audit.jsonl` required and missing/empty/tampered states fail report-defense.
- Add regression tests and CLI/manual QA evidence for validation and filesystem recovery missing-chain false-pass cases, not just proxy cases.
- Document or execute the size-gate disposition with T11 scope expanded to cover the T4-touched oversized files.

## Final Decision

codeQualityStatus: BLOCK
recommendation: REQUEST_CHANGES
reportPath: `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/t4-code-review-slop-report.md`
