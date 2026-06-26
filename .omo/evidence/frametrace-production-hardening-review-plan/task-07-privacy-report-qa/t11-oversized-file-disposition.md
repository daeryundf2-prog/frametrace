# T7 Disposition For Oversized Files Carried Into T11

This T7 gate-blocker fix does not split Rust/product files. The blocker is resolved by making the disposition explicit and preserving the existing plan order: T11 owns module splitting after T8-T10 behavior and performance locks are green.

## Files Carried Into T11 Scope

- `src/qa_report_defense.rs`
- `src/qa_release.rs`
- `src/qa_tests.rs`
- `tests/cli_lifecycle.rs`
- `tests/cli_windows_prereq.rs`

## T11 Scope Widening Instruction

When T11 runs, it must include the five T7-touched files above in addition to the original T11 list: `src/cli/handlers.rs`, `src/scan.rs`, `src/html_report.rs`, `src/report.rs`, and `src/artifacts.rs`.

T11 must split by responsibility only after behavior is pinned, with no command-output or JSON-contract drift except changes already intentionally introduced by earlier todos. T7 remains a privacy/report-defense QA task and must not be marked complete based on this disposition alone.

## Why Not Split In T7

- The current user instruction is to fix T7 gate blockers only and not expand product scope.
- The gate explicitly allows updating the T11 plan text or adding a T7 evidence disposition.
- Broad module splitting would touch product Rust and require a new fmt/clippy/test cycle beyond the evidence blocker fix.
- The risk is now visible, named, and assigned to the planned refactor task instead of being buried in a T7 done claim.
