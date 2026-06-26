# T1 Baseline Snapshot

Task: Create a clean baseline snapshot and regression fixture set

## Source Context

- Plan: `.omo/plans/frametrace-production-hardening-review-plan.md`
- Review evidence read:
  - `.omo/ulw-loop/frame-review-progress-20260624/evidence/code-review-scan.md`
  - `.omo/ulw-loop/frame-review-progress-20260624/evidence/progress-readiness-audit.md`
  - `.omo/ulw-loop/frame-review-progress-20260624/evidence/remaining-steps.md`
  - `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md`
  - `docs/MVP_STATUS.md`

## Git Baseline

- HEAD: `c74e8974f30e1bbada49f83e6100fceb2dc49528`
- Branch: `codex/frametrace-forensic-hardening`
- Status source: `command-01-git-baseline-status.txt`
- Dirty worktree classification before helper implementation: untracked `.omo` plan/evidence/runtime artifacts only, plus the new T1 evidence receipts already created at snapshot time. No tracked Rust, JavaScript, docs, or product files were modified at the baseline snapshot.

## Paths Touched By T1

- `.omo/evidence/frametrace-production-hardening-review-plan/task-01-baseline/`
- `scripts/qa/verify-plan-evidence.py`

## Untouched Unrelated Dirty Paths

The pre-existing dirty worktree is untracked `.omo` material outside the T1 evidence directory. T1 did not edit or remove these paths:

- `.omo/boulder.json`
- `.omo/drafts/`
- `.omo/evidence/final-security-review-after-log-output-symlink-fix-code-review.md`
- `.omo/plans/`
- `.omo/start-work/`
- `.omo/ulw-loop/`

## Current Command Results

| Receipt | Command | Exit | Result |
| --- | --- | ---: | --- |
| `command-00-missing-helper-self-test.txt` | `python3 scripts/qa/verify-plan-evidence.py --self-test` before helper existed | 2 | Expected failing-first missing-helper baseline |
| `command-01-git-baseline-status.txt` | `git rev-parse HEAD && git rev-parse --abbrev-ref HEAD && git status --short --untracked-files=all` | 0 | PASS |
| `command-02-cargo-fmt-check.txt` | `cargo fmt --all -- --check` | 0 | PASS |
| `command-03-cargo-clippy.txt` | `cargo clippy --locked --all-targets --all-features -- -D warnings` | 0 | PASS |
| `command-04-cargo-test-locked.txt` | `cargo test --locked` | 0 | PASS |
| `command-05-node-check-app-js.txt` | `node --check gui/evidence-viewer/app.js` | 0 | PASS |
| `command-06-git-diff-check.txt` | `git diff --check` | 0 | PASS |
| `command-07-debug-binary-build-if-needed.txt` | `test -x target/debug/frametrace || cargo build --locked` | 0 | PASS; debug binary already existed, so build was not run |
| `command-08-empty-case-release-fail-closed.txt` | `target/debug/frametrace qa release <tmp-case> --output-dir <tmp-case>/reports/qa-review-audit` | 1 | Expected fail-closed release blockers |
| `command-09-helper-self-test.txt` | `python3 scripts/qa/verify-plan-evidence.py --self-test` after helper creation | 0 | PASS |
| `command-09a-helper-self-test-failed-misplaced-helper.txt` | same self-test during recovery from wrong workspace write | 2 | Recorded operational failure; corrected by moving helper into FrameTrace repo |

## Empty-Case Release Blockers

The empty-case release check failed closed with 28 blockers. Key blocker classes:

- `report_defense`: missing six required report/case artifacts.
- `workstation_shell_contract`: missing `case.json`.
- `windows_prerequisites`: unsupported macOS host, missing `dotnet`, missing WinUI project, missing WinUI build receipt.
- Review gates: missing `--review-manifest` for every typed review key.
- `accuracy`: missing `--corpus-manifest`.
- `reproducibility`: missing `--comparison-case`.

Copied release artifacts:

- `empty-release-output/release-readiness.json`
- `empty-release-output/release-readiness.md`
- `empty-release-output/report-defense-checklist.md`
- `empty-release-output/windows-prerequisites.json`
- `empty-release-output/performance/performance-report.json`
- `empty-release-output/performance/performance-report.md`
- `empty-release-output/performance/db/case.db`

## Blocker Fixture List

These are the review-blocker fixtures or fixture definitions T1 pins for downstream todos. T1 did not change product behavior.

| Blocker | Minimal fixture shape | Current evidence source |
| --- | --- | --- |
| Default path leakage | Case under a temp path containing client/user-like names; generate report/review/viewer/package and grep for absolute temp path | `code-review-scan.md` path-leak findings |
| Missing audit chain not blocking report defense | Case with report-visible derived/recovered/validation artifact and withheld corresponding audit log | `code-review-scan.md` missing-audit finding |
| Large-case full-load paths | Synthetic 100k/1M inventory with report and compatibility output generation | `code-review-scan.md` large-case finding; empty release performance artifact copied here |
| Validation target trust | Poisoned or stale JSONL audit record plus direct external path target | `code-review-scan.md` validation target finding |
| Direct ffmpeg execution | PATH-controlled fake/disallowed `ffmpeg` plus derived artifact operation | `code-review-scan.md` ffmpeg provenance finding |
| Oversized module maintainability | Pure LOC scan over `src/scan.rs`, `src/html_report.rs`, `src/cli/handlers.rs`, `src/artifacts.rs`, `src/report.rs` | `code-review-scan.md` oversized-module list |
| Missing Windows/WinUI/corpus evidence | Empty release case with missing Windows, WinUI, corpus manifest, comparison case, and review manifest inputs | `command-08-empty-case-release-fail-closed.txt`; `empty-release-output/` |

## Helper Baseline

- Pre-helper failure was captured in `command-00-missing-helper-self-test.txt` with exit code 2.
- Added helper: `scripts/qa/verify-plan-evidence.py`.
- Helper scope: non-product QA tooling only; it parses T-numbered plan todos, requires matching `task-XX-*` evidence directories, enforces known required T1 receipt names, and exposes `--self-test`.
- Self-test coverage: missing evidence root, missing task directories, missing required receipts, and valid evidence.
- Self-test output: `command-09-helper-self-test.txt`.

## Adversarial Class Notes

- dirty_worktree: classified as pre-existing untracked `.omo` material plus T1-created evidence/helper files; T1 touched only the paths listed above.
- hung_or_long_commands: every command transcript includes start/end timestamps and elapsed seconds; no command was left running.
- misleading_success_output: every receipt records `ExitCode`.
- stale_state: T1 evidence directory was newly populated in this run; no pre-existing files were found at `find .omo/evidence/frametrace-production-hardening-review-plan -maxdepth 3 -type f` before T1 receipts were created.
- flaky_tests: no baseline verification command failed unexpectedly; no rerun was needed for cargo/node/git checks.
- malformed_input: helper self-test covers missing root, missing task directories, and missing receipts.
- prompt_injection: not applicable; all inputs were local repo files named by the plan/user, and no external untrusted instructions were executed.
- cancel_resume: not applicable; no interruption or cancellation state affected T1.
- repeated_interruptions: not applicable; no repeated user/tool interruptions occurred.
