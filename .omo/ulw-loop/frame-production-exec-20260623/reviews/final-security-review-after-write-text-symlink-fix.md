# Final Security Review After Write Text Symlink Fix

Scope: FrameTrace output path safety at HEAD `c6a7abcddfcdb5ca027c5b751545476829a7661b`, re-auditing the blockers from `.omo/ulw-loop/frame-production-exec-20260623/reviews/final-security-review-after-derived-symlink-fix.md`.

## Verdict

APPROVE

The previous BLOCKED finding is resolved for the requested scope. Static final-leaf symlink writes are rejected by `write_text`; `make-review` and `make-report` reject final symlink leaves and symlinked `review`/`reports` parent directories; derived outputs, inventory export, and `unique_path` symlink policies remain intact; and source-evidence overwrite guards remain covered.

## Files Reviewed

- `src/util.rs`
- `src/cli/handlers.rs`
- `tests/cli_output_policy.rs`
- `src/tool_policy.rs`
- `src/derived_output_policy_tests.rs`
- `src/case_db/inventory_export.rs`
- `src/case_db/inventory_export_tests.rs`
- `tests/cli_inventory.rs`
- `src/artifacts.rs`
- `src/video_export.rs`
- `src/tsk.rs`
- `src/e01.rs`

## Spec Compliance

PASS.

- `write_text` now calls `reject_symlink_leaf` before `fs::write` (`src/util.rs:38-43`, `src/util.rs:81-90`). The check uses `symlink_metadata`, so live and dangling final symlink leaves are rejected instead of followed.
- `unique_path` still uses `symlink_metadata` and treats any existing leaf, including dangling symlinks, as occupied (`src/util.rs:46-78`). Regression coverage remains at `src/util.rs:251-268`.
- `make_review` validates both `review/index.html` and `review/evidence-viewer.html` through `require_case_output_path` before `write_text` (`src/cli/handlers.rs:263-298`).
- `make_report` validates `reports/case-report.html` through `require_case_output_path` before `write_text` (`src/cli/handlers.rs:343-345`).
- `require_case_output_path` canonicalizes the case root, canonicalizes the nearest existing output parent, rejects parents resolving outside the case root, and rejects symlink final leaves (`src/tool_policy.rs:79-106`, `src/tool_policy.rs:187-199`).
- Derived outputs still call `require_case_output_path` and `reject_source_output_path` before writing or invoking external tools: proxies/thumbnails/frames (`src/artifacts.rs:81-99`, `src/artifacts.rs:142-160`, `src/artifacts.rs:203-221`), video export (`src/video_export.rs:57-80`), inode recovery (`src/tsk.rs:278-293`), and E01 raw export (`src/e01.rs:118-132`).
- Inventory export still applies case containment, registered source-evidence rejection, and existing/symlink output rejection before writing (`src/case_db/inventory_export.rs:66-96`, `src/case_db/inventory_export.rs:98-123`).
- Source evidence overwrite regression remains covered by `reject_source_output_path` (`src/tool_policy.rs:109-131`) and tests (`src/tool_policy.rs:269-283`, `src/case_db/inventory_export_tests.rs:118-147`).

## Findings

No CRITICAL, HIGH, MEDIUM, or LOW issues found in the requested output path safety scope.

## Exploit-Style Checks

Manual CLI checks against `target/debug/frametrace` used only temporary directories and verified non-creation of outside symlink targets.

PASS results:

- `write_text_final_leaf`: pre-created `case/case.json` as a symlink to an outside target, then ran `init-case`. Command exited non-zero with `output cannot be a symlink`; outside target was not created.
- `make_review_index_leaf`: pre-created `review/index.html` as a symlink to an outside target, then ran `make-review`. Command exited non-zero with `review html output cannot be a symlink`; outside target was not created.
- `make_review_viewer_leaf`: pre-created `review/evidence-viewer.html` as a symlink to an outside target, then ran `make-review`. Command exited non-zero with `evidence viewer html output cannot be a symlink`; outside target was not created.
- `make_review_parent_dir`: replaced `review` with a symlink to an outside directory, then ran `make-review`. Command exited non-zero with `output must be inside the case directory`; outside `index.html` was not created.
- `make_report_leaf`: pre-created `reports/case-report.html` as a symlink to an outside target, then ran `make-report`. Command exited non-zero with `case report html output cannot be a symlink`; outside target was not created.
- `make_report_parent_dir`: replaced `reports` with a symlink to an outside directory, then ran `make-report`. Command exited non-zero with `output must be inside the case directory`; outside `case-report.html` was not created.

Summary: `pass=6 fail=0`.

## Validation

- `git rev-parse HEAD`: `c6a7abcddfcdb5ca027c5b751545476829a7661b`.
- `git show --stat --oneline HEAD`: reviewed commit `c6a7abc Reject symlinked report output writes`; modified files are `src/cli/handlers.rs`, `src/util.rs`, and `tests/cli_output_policy.rs`.
- `cargo test --locked --test cli_output_policy -- --nocapture`: PASS, 4/4 tests.
- `cargo test --locked symlink -- --nocapture`: PASS, 9 symlink-focused tests, including derived outputs, inventory export, and `unique_path`.
- `cargo test --locked inventory_export -- --nocapture`: PASS, 4/4 inventory export tests.
- `cargo test --locked export_manifest_rejects_registered_source_evidence_output -- --nocapture`: PASS, 1/1 source-evidence overwrite regression test.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: PASS.
- `git diff --check HEAD^ HEAD`: PASS.
- `cargo fmt --all -- --check`: PASS.
- `cargo test --locked`: PASS, 117 library tests, all CLI integration tests, and doc tests.

## Tooling Limitations

- LSP diagnostics were attempted for all modified files (`src/util.rs`, `src/cli/handlers.rs`, `tests/cli_output_policy.rs`), but the configured Rust LSP was unavailable: the daemon timed out and `rust-analyzer` is missing from the active toolchain. Compiler-backed validation (`cargo test`, `cargo clippy`) passed.
- `sg`/`ast-grep` was not installed in this environment. Pattern review used `rg` fallback for write/copy/create and silent-default patterns.
- Native subagent review was unavailable in this leaf reviewer context; this report is based on direct source review, tests, clippy, and exploit-style CLI checks.

## Recommendation

The requested symlink/output-overwrite blockers are resolved. No exact blocker remains in scope.

APPROVE
