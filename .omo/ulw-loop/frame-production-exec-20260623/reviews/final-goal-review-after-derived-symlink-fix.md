# Final Goal Review After Derived Symlink Fix

## Verdict

APPROVE

The latest reviewed HEAD is `151fd5c` on branch `codex/frametrace-forensic-hardening`, not just `a42c3af`. The latest implementation aligns with the active ULW objective: it closes the derived-output symlink blocker from `a42c3af`, closes the neighboring default/generated-output symlink selection risk in `151fd5c`, preserves original evidence paths read-only, and continues to avoid claiming Windows/WinUI GA from macOS.

## Acceptance Target

- Active goal: `.omo/ulw-loop/frame-production-exec-20260623/goals.json`, goal `G001-complete-frametrace-production-conti`.
- Objective checked: complete macOS-compatible FrameTrace production work, preserve original evidence paths read-only, and stop short of Windows/WinUI GA claims unless native Windows/WinUI evidence exists.
- Latest commits reviewed:
  - `151fd5c Treat symlinked generated outputs as occupied`
  - `a42c3af Reject derived output symlink targets`
  - prior branch commits through `1e07753` for goal context and blocker history.

## Code Evidence

- `src/tool_policy.rs:79` now routes all `require_case_output_path` callers through final-leaf symlink inspection before allowing output writes.
- `src/tool_policy.rs:187` uses `std::fs::symlink_metadata`, so a dangling symlink leaf is detected as occupied/rejected instead of being missed by `Path::exists()`.
- `src/derived_output_policy_tests.rs:67`, `:85`, `:103`, `:121`, and `:141` cover proxy, thumbnail, frame capture, video export, and inode recovery dangling-symlink outputs before ffmpeg/icat execution.
- `src/util.rs:45` now calls `path_is_available` from `unique_path`; `src/util.rs:73` implements availability with `symlink_metadata`; `src/util.rs:240` adds the dangling-symlink generated-output regression.
- Scope check: `git diff a42c3af..HEAD --stat` reports only `src/util.rs` after `a42c3af`; `git diff HEAD~1..HEAD -- src/util.rs` shows a narrow replacement of `exists()` checks plus one regression test. No Windows/WinUI implementation or GA-claiming code was added by the latest unit.

## ULW Evidence Checked

- `goals.json` still records the active objective as macOS-compatible completion without claiming Windows/WinUI GA, and its criteria record passed evidence for dirty-worktree classification, Windows prerequisite negative readiness, and regression validation.
- `ledger.jsonl` tail records:
  - `a42c3af` remediation: derived artifact and inode recovery outputs reject dangling final-leaf symlinks.
  - `151fd5c` remediation: `unique_path` treats dangling symlink leaves as occupied for generated/default outputs.
- `.omo/ulw-loop/frame-production-exec-20260623/evidence/derived-output-symlink-policy-fix.txt` records fmt, clippy, symlink-filter tests, inventory export regression tests, CLI inventory/review tests, and full `cargo test --locked` evidence after `a42c3af`.
- `.omo/ulw-loop/frame-production-exec-20260623/evidence/unique-path-symlink-policy-fix.txt` records a red proof for `unique_path_treats_dangling_symlink_as_occupied`, then green targeted, symlink-filter, and full-suite validation after `151fd5c`.

## Fresh Verification

- `git log -6 --oneline --decorate` confirmed HEAD is `151fd5c` on `codex/frametrace-forensic-hardening`.
- `cargo fmt --all -- --check` exited 0.
- `cargo clippy --locked --all-targets --all-features -- -D warnings` exited 0.
- `cargo test --locked unique_path_treats_dangling_symlink_as_occupied -- --nocapture` exited 0: 1 passed.
- `cargo test --locked symlink -- --nocapture` exited 0: 9 passed, including derived-output, inventory-export, package, tool-policy, and `unique_path` symlink regressions.
- `cargo test --locked` exited 0: 117 library tests passed, plus `cli_inventory`, `cli_lifecycle`, `cli_review`, `cli_smoke`, `cli_windows_prereq`, `media_contract`, and doc-test targets.
- `node --check gui/evidence-viewer/app.js` exited 0.
- `git diff --check HEAD~2..HEAD` exited 0.

## Gaps

- Windows/WinUI native GA is not proven from this macOS host. This is an explicit goal constraint, not a failure of the reviewed macOS-compatible work.
- `goals.json` status remains `in_progress`; this review verifies the latest implementation and evidence against the active goal, but does not mutate ULW state.
- The workspace contains many untracked `.omo` artifacts. I treated them as evidence/artifact state and did not modify source files.

## Risks

- The symlink fixes reject or avoid pre-existing dangling symlink leaves, which closes the reviewed static path. They do not claim to eliminate every possible same-host time-of-check/time-of-use race by a concurrent actor with write access to the case tree.
- `src/tool_policy.rs` reports 250 pure LOC in the captured evidence, which is exactly at the stated ceiling, not above it.

## Stop Condition

The final security blocker from the prior review is closed for explicit derived outputs, the neighboring generated-output symlink gap is closed in HEAD, fresh validation passes, and Windows/WinUI GA remains unclaimed pending native Windows/WinUI evidence.

APPROVE
