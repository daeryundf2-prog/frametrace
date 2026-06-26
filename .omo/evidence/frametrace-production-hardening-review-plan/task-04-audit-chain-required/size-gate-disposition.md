# T4 Size Gate Disposition

Scenario: programming size gate for T4 audit-chain required-trigger repair
Invocation: `for f in src/audit.rs src/qa_report_defense.rs src/qa_tests.rs tests/media_contract.rs; do awk 'NF && $1 !~ /^\/\// {n++} END {print FILENAME " pure_loc=" n}' "$f"; done`
Observable: three T4 product/test modules remain above the 250 pure LOC programming threshold.
Artifact: `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/size-gate-disposition.md`

## Current LOC

- `src/audit.rs` pure_loc=403
- `src/qa_report_defense.rs` pure_loc=361
- `src/qa_tests.rs` pure_loc=396
- `tests/media_contract.rs` pure_loc=87

## Disposition

T4 defers module splitting to T11. The T4 acceptance criteria are about report-defense behavior: missing, empty, and tampered required audit chains must block, while unsupported and not-applicable chains must remain visible without masquerading as pass. This blocker fix adds validation-claim and recovered-filesystem trigger coverage without moving module boundaries. Splitting `src/audit.rs`, `src/qa_report_defense.rs`, or `src/qa_tests.rs` now would be a broad refactor in a shared dirty worktree and would overlap the explicit T11 module-split todo after T8-T10 behavior locks.

No product code refactor was performed for this blocker repair. T11 refactor debt explicitly includes:

- `src/audit.rs`: split audit-chain verification/status reporting from unrelated audit helpers.
- `src/qa_report_defense.rs`: split required-chain surface classification from checklist rendering.
- `src/qa_tests.rs`: split report-defense regression tests from accuracy/reproducibility/performance QA tests.

The live T4 behavior is covered by the refreshed fix evidence:

- `cargo test --locked report_defense_ -- --nocapture`
- `target/debug/frametrace qa report-defense <validation-claimed-case>`
- `target/debug/frametrace qa report-defense <recovered-filesystem-case>`
- `cargo fmt --all -- --check`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `cargo test --locked`
- `git diff --check`

The size gate is therefore recorded as an accepted T4 deferral, not resolved by code movement. T11 remains the correct place to split oversized modules after the larger behavior surface is pinned.
