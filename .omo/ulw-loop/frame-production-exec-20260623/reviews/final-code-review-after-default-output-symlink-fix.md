# Final Code-Quality Review: default output symlink fix

Repository: `/Users/shinyoohag/Desktop/frametrace`  
Branch: `codex/frametrace-forensic-hardening`  
HEAD reviewed: `552b3fc` (`Block default artifact symlink escapes`)  
Scope: commits `f589dea` and `552b3fc`, with prior output-policy commits `541ec49`, `a42c3af`, `151fd5c`, and `c6a7abc` as context.

## Verdict

codeQualityStatus: BLOCK  
recommendation: REQUEST_CHANGES  
reportPath: `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/reviews/final-code-review-after-default-output-symlink-fix.md`

Blockers:
- HIGH: several case-owned durable evidence and filesystem outputs still bypass `require_case_output_path`, and CLI proofs show they write outside the canonical case tree through symlinked `case/evidence/logs` and `case/db/filesystem` parents.

## Skill-Perspective Check

Ran/consulted:
- `omo:remove-ai-slops`: applied the overfit/slop review pass to production code and tests.
- `omo:programming` plus Rust reference: applied Rust path-boundary, typed-boundary, test-shape, and maintainability criteria.
- `code-review`: loaded. Its independent subagent lanes were not run because the available `spawn_agent` tool explicitly forbids subagents unless the user asks for delegation; this report is a direct, evidence-backed review.

Perspective result:
- The new CLI tests in `tests/cli_default_output_policy.rs` and `tests/cli_output_policy.rs` are not tautological or deletion-only; they exercise the binary and assert the observable non-write to outside directories.
- The diff does not introduce needless production parsing or normalization for the paths it touches.
- The remaining issue violates both perspectives because the same case-owned output boundary is enforced in some writers but bypassed in E01/TSK/validation evidence-log writers, leaving a boundary invariant partially encoded and under-tested.

## Verification Performed

- `git show --stat --oneline 552b3fc`: `552b3fc Block default artifact symlink escapes`; 5 files changed, 271 insertions, 6 deletions. Files: `src/artifacts.rs`, `src/carve.rs`, `src/package.rs`, `src/video_export.rs`, `tests/cli_default_output_policy.rs`.
- `cargo test --locked --test cli_default_output_policy -- --nocapture`: PASS, 4 passed.
- `cargo test --locked --test cli_output_policy -- --nocapture`: PASS, 5 passed.
- `cargo test --locked symlink -- --nocapture`: PASS, symlink-filtered tests passed across unit and CLI suites.
- `cargo test --locked derived_output_policy_tests -- --nocapture`: PASS, 5 passed.
- `cargo test --locked`: PASS, 117 unit tests plus all integration tests passed.

Additional reviewer proofs:
- `inspect-image` with `case/evidence/logs` replaced by a symlink succeeded and wrote outside files: `outside-logs/tsk-audit.jsonl`, `outside-logs/tsk-fls-*.txt`, `outside-logs/tsk-mmls-*.txt`.
- `inspect-e01` with `case/evidence/logs` replaced by a symlink succeeded and wrote outside files: `outside-logs/e01-audit.jsonl`, `outside-logs/e01-info-*.txt`.
- `inspect-image` with `case/db/filesystem` replaced by a symlink succeeded and wrote outside files: `outside-dbfs/tsk-files-*.jsonl`, `outside-dbfs/tsk-inspection-*.json`.
- `validate-artifact` with `case/evidence/logs` replaced by a symlink succeeded and wrote outside file: `outside-logs/validation-log.jsonl`.

## Findings

### CRITICAL

None.

### HIGH

1. Remaining case-owned evidence and filesystem outputs can still escape through symlinked parents.

Evidence:
- `src/e01.rs:51` to `src/e01.rs:57` chooses `case/evidence/logs/e01-info-*.txt` and writes it via `write_text` without first calling `require_case_output_path`.
- `src/e01.rs:90` to `src/e01.rs:115` chooses `case/evidence/logs/e01-info-*.txt` / `e01-verify-*.txt`; the verify path is passed to `ewfverify -l` before any case-output containment guard.
- `src/e01.rs:143` to `src/e01.rs:149` passes `case/evidence/logs/e01-export-*.txt` to `ewfexport` without a case-output containment guard.
- `src/e01.rs:341` to `src/e01.rs:342` appends `case/evidence/logs/e01-audit.jsonl` through `audit::append_chained_jsonl` without a case-output containment guard.
- `src/tsk.rs:131` to `src/tsk.rs:172` writes `case/evidence/logs/tsk-mmls-*.txt` and `tsk-fls-*.txt` without a case-output containment guard.
- `src/tsk.rs:191` to `src/tsk.rs:221` writes `case/db/filesystem/tsk-files-*.jsonl` and `tsk-inspection-*.json` without a case-output containment guard.
- `src/tsk.rs:223` to `src/tsk.rs:242` and `src/tsk.rs:393` to `src/tsk.rs:394` append `case/evidence/logs/tsk-audit.jsonl` without a case-output containment guard.
- `src/validation.rs:199` to `src/validation.rs:207` and `src/playback.rs:53` to `src/playback.rs:55` append `case/evidence/logs/validation-log.jsonl` without a case-output containment guard.
- `src/audit.rs:74` to `src/audit.rs:99` uses `write_text`, which rejects symlink final leaves but does not reject symlinked case-owned parents.

Why this blocks approval:
The previous blocker class was that durable generated outputs/logs could write outside the canonical case tree through symlinked case-owned parents. `552b3fc` closes that class for default export-video, derived media, carve-file, package default, report/review, and scan DB outputs, but the same class remains reachable through E01, TSK, validation, and playback evidence-log paths. These are durable forensic artifacts, and the CLI can still report success while writing them outside the case tree.

Minimal fix guidance:
Route every case-owned E01/TSK/validation/playback log and `db/filesystem` output path through `require_case_output_path` before writing or passing paths to external tools. Prefer a small shared helper for case-owned audit/text outputs so future calls to `audit::append_chained_jsonl` cannot bypass containment accidentally. Add CLI regressions for symlinked `case/evidence/logs` and `case/db/filesystem` parents for `inspect-e01`, `import-e01`, `inspect-image`, `recover-inode`, `validate-artifact`, and `confirm-playback` where applicable.

### MEDIUM

None.

### LOW

None.

## Approved Portions

The `552b3fc` changes themselves are directionally correct for the paths they touch:
- `src/video_export.rs` guards default clip output and export log paths.
- `src/artifacts.rs` guards default proxy/thumbnail/frame outputs and derived artifact logs.
- `src/carve.rs` guards carved artifacts, carve results, and carve logs.
- `src/package.rs` guards the default package directory selected under `case/reports`.
- `f589dea` guards scan DB/index writes and SQLite opening under `case/db`.

The new tests are meaningful for those touched paths and passed locally.

BLOCKED
