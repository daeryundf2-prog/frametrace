# Final Code Review After write_text Symlink Fix

**Repository:** `/Users/shinyoohag/Desktop/frametrace`  
**Branch:** `codex/frametrace-forensic-hardening`  
**HEAD:** `c6a7abcddfcdb5ca027c5b751545476829a7661b`  
**Scope:** Diff since `origin/codex/frametrace-forensic-hardening`, with emphasis on `src/util.rs`, `src/cli/handlers.rs`, `tests/cli_output_policy.rs`, and prior symlink/output-policy commits.

## Code Review Summary

**Files Reviewed:** 58 changed files by diff/stat; focused review on symlink/output-policy surfaces and the requested files.  
**Total Issues:** 1

### By Severity

- CRITICAL: 0
- HIGH: 1
- MEDIUM: 0
- LOW: 0

## Stage 1 - Spec Compliance

The `c6a7abc` fix covers the specific static final-leaf symlink issue for report/review HTML outputs:

- `src/util.rs:38-43` now rejects a symlink final component before `fs::write`.
- `src/cli/handlers.rs:263-297` and `src/cli/handlers.rs:343-345` now route review/report HTML outputs through `require_case_output_path`.
- `tests/cli_output_policy.rs:55-164` covers static dangling final leaves and symlinked `review`/`reports` parent directories for `make-review` and `make-report`.

However, the branch does not satisfy the broader output-policy hardening contract because shared case writers still permit writes through symlinked case subdirectories outside the intended case tree.

## Root-Cause Guard

The patch is not merely a fallback, but it is incomplete as a root-cause repair. It rejects static symlink leaves at `write_text`, while leaving parent-directory symlink traversal possible for unguarded `write_text` call sites. The fix should move the invariant to a single write policy that verifies case-root containment for the destination parent, rejects symlink components where case outputs are expected, and is applied consistently to every durable case output.

## Issues

### [HIGH] Unguarded case outputs still write through symlinked case subdirectories

**File:** `src/scan.rs:240`  
**Related shared writer:** `src/util.rs:38`

**Issue:** `scan-folder` writes durable case outputs with raw `write_text` calls under `case_dir/db` without first applying `require_case_output_path` or equivalent parent containment. Because `write_text` only checks the final leaf at `src/util.rs:42`, a symlinked parent directory is followed. A tampered case tree can redirect `db/case.db`, `db/video_index.json`, `db/videos.jsonl`, `db/video_paths.tsv`, and scan-run snapshots outside the case directory while the command reports success.

**Evidence:** I reproduced this at the CLI:

```text
case_db_link=lrwxr-xr-x ... /tmp/frametrace-parent-symlink-review.../case/db -> /tmp/frametrace-parent-symlink-review.../outside-db
outside_files=/tmp/frametrace-parent-symlink-review.../outside-db/case.db
/tmp/frametrace-parent-symlink-review.../outside-db/scan_runs/scan_1782216606.json
/tmp/frametrace-parent-symlink-review.../outside-db/video_index.json
/tmp/frametrace-parent-symlink-review.../outside-db/video_paths.tsv
/tmp/frametrace-parent-symlink-review.../outside-db/videos.jsonl
scan complete
```

**Why this blocks:** The branch is explicitly hardening forensic output policy after a symlink write blocker. Leaving other durable case outputs able to escape through symlinked parents preserves the same class of filesystem-confusion risk for case state and audit-relevant data, even though the report/review HTML path is now covered.

**Fix:** Centralize durable case-output writing behind a case-root-aware helper. For every case-bound output, resolve the nearest existing parent, verify it remains under the canonical case root, reject symlink final leaves, and use that helper instead of raw `write_text`, `File::create`, or `fs::copy` for case-owned outputs. Add regression tests for at least `scan-folder` with `case/db` as a symlink to an outside directory and assert the command fails without creating outside files.

## Verification

- `cargo fmt --all -- --check` passed.
- `cargo clippy --locked --all-targets --all-features -- -D warnings` passed.
- `cargo test --locked --test cli_output_policy -- --nocapture` passed: 4/4.
- `cargo test --locked symlink -- --nocapture` passed all symlink-filtered tests, including output-policy tests.
- `cargo test --locked --test cli_inventory -- --nocapture` passed: 1/1.
- `cargo test --locked --test cli_review -- --nocapture` passed: 2/2.
- `git diff --check origin/codex/frametrace-forensic-hardening...HEAD` passed.
- LSP diagnostics could not be collected: the LSP daemon timed out repeatedly for Rust files.

## Recommendation

BLOCKED until symlinked case subdirectories cannot redirect durable case outputs outside the canonical case tree.

BLOCKED
