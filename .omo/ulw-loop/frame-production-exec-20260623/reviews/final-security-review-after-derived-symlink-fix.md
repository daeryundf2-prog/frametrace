# Final Security Review After Derived Symlink Fix

Scope: FrameTrace output path safety at HEAD `151fd5ced5c01eb16ea95d34cab0cae047f3ae1d`, including prior commit `a42c3af` and latest commit `151fd5c`.

## Verdict

BLOCKED

The `require_case_output_path` final-leaf symlink fix and the `unique_path` dangling-symlink fix both work for the derived/tool output paths under review. However, HEAD still has reachable `write_text` output surfaces that follow final symlink leaves and can write outside the intended case output path.

## Files Reviewed

- `src/tool_policy.rs`
- `src/util.rs`
- `src/derived_output_policy_tests.rs`
- `src/artifacts.rs`
- `src/video_export.rs`
- `src/tsk.rs`
- `src/e01.rs`
- `src/case_db/inventory_export.rs`
- `src/cli/handlers.rs`
- `src/cli/commands.rs`
- `src/cli/qa_cmd.rs`
- `src/package.rs`
- `src/carve.rs`
- `src/qa_accuracy.rs`
- `src/qa_repro.rs`
- `src/qa_report_defense.rs`
- `src/qa_release.rs`
- `src/audit.rs`

## Findings

### [HIGH] Remaining report/write outputs follow final symlink leaves

File: `src/util.rs:38`

`write_text` creates parent directories and calls `fs::write(path, text)` directly. `fs::write` follows a final symlink leaf. Several user-facing output paths call this helper without the `require_case_output_path` final-leaf symlink rejection used by derived tool outputs.

Representative reachable surfaces:

- `src/cli/handlers.rs:262-263` writes `case_dir/review/index.html`.
- `src/cli/handlers.rs:293-294` writes `case_dir/review/evidence-viewer.html`.
- `src/cli/handlers.rs:340-341` writes `case_dir/reports/case-report.html`.
- `src/qa_accuracy.rs:71-75` creates a caller-controlled QA output directory and writes report leaves.
- `src/qa_repro.rs:15-18`, `src/qa_report_defense.rs:34-37`, and `src/qa_release.rs:65-70` have the same pattern.
- `src/package.rs:73-83` writes package manifests/README via `write_text`; `src/package.rs:254-259` copies package files to targets without rejecting symlink leaves in the destination tree.
- `src/audit.rs:98` appends chained logs by rewriting through `write_text`.

Impact: a case directory or output directory containing a malicious final symlink can cause FrameTrace to overwrite or create files outside the intended output tree with the privileges of the operator running the command. This is the same class of final-leaf symlink-follow issue as the prior derived-output blocker, just on non-FFmpeg/non-icat write surfaces.

Real-surface proof:

```text
Command shape:
tmp=$(mktemp -d)
cargo run --quiet -- init-case "$tmp/case" --title t --operator qa
mkdir -p "$tmp/case/review"
ln -s "$tmp/outside-review.html" "$tmp/case/review/index.html"
cargo run --quiet -- make-review "$tmp/case"

Observed:
status=0
outside_exists=yes
stdout included "review written: .../case/review/index.html"
outside file began with "<!doctype html>"
```

Fix: route these write surfaces through a shared symlink-safe output writer. At minimum, before writing, inspect the final leaf with `symlink_metadata` and reject symlinks, canonicalize and validate existing parents where case containment is required, and use non-following/create-new semantics where available for new files. Apply the same policy to `fs::copy`, `File::create`, and audit-log rewrites, not only tool-executed derived outputs. Add regression tests for `make-review`, `make-report`, QA reports, package outputs, and audit-log writes with dangling final symlink leaves.

## Positive Verification

### `require_case_output_path`

`src/tool_policy.rs:79-106` canonicalizes the case root, resolves the output path lexically, canonicalizes the nearest existing parent, rejects parent escape, then calls final-leaf rejection. `src/tool_policy.rs:187-199` uses `symlink_metadata`, so both live and dangling final symlink leaves are rejected.

All direct callers were inspected:

- `src/artifacts.rs:81-99` proxy explicit outputs.
- `src/artifacts.rs:142-160` thumbnail explicit outputs.
- `src/artifacts.rs:203-221` frame capture explicit outputs.
- `src/video_export.rs:57-80` video export explicit outputs.
- `src/tsk.rs:268-286` inode recovery outputs.
- `src/e01.rs:118-130` E01 raw output.
- `src/case_db/inventory_export.rs:71-77` inventory manifest output.

The prior blocker is covered for explicit derived outputs by `src/derived_output_policy_tests.rs`, including proxy, thumbnail, frame capture, video export, and inode recovery dangling-symlink regressions.

### `unique_path`

`src/util.rs:45-77` now treats any `symlink_metadata` success as occupied and only returns a path when metadata returns `NotFound`. This closes the generated/default output collision that previously let dangling symlink leaves be selected by `unique_path`.

Evidence file reviewed: `.omo/ulw-loop/frame-production-exec-20260623/evidence/unique-path-symlink-policy-fix.txt`.

## Validation Run

- `cargo test --locked symlink -- --nocapture`: PASS, 9 passed.
- `cargo test --locked`: PASS, 117 library tests plus integration/doc tests passed.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: PASS.
- LSP diagnostics were attempted for `src/tool_policy.rs`, `src/lib.rs`, `src/derived_output_policy_tests.rs`, and `src/util.rs`, but the LSP daemon timed out repeatedly. Compiler-backed validation above completed successfully.

## Recommendation

Do not approve the output-path safety fix as complete until the remaining `write_text`/`File::create`/`fs::copy` output surfaces reject final symlink leaves or use a shared non-following safe-write policy with regression coverage.

BLOCKED
