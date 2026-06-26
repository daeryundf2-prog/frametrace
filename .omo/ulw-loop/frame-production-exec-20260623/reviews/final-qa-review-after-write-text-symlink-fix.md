# Final QA Review - write_text symlink fix

Repository: `/Users/shinyoohag/Desktop/frametrace`
Reviewed HEAD: `c6a7abc`
Evidence directory: `.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-write-text-symlink-fix/`

## Verdict

No blocking QA findings. The requested gate is covered and passes fresh validation.

## Coverage Review

### Review/report final leaf symlink attacks

Covered by `tests/cli_output_policy.rs` through the compiled CLI binary:

- `make_review_rejects_symlinked_review_outputs_without_writing_target` creates `review/index.html` as a symlink to an outside target, runs `make-review`, expects `cannot be a symlink`, and asserts the outside target was not written.
- `make_report_rejects_symlinked_report_outputs_without_writing_target` creates `reports/case-report.html` as a symlink to an outside target, runs `make-report`, expects `cannot be a symlink`, and asserts the outside target was not written.

Relevant lines: `tests/cli_output_policy.rs:55`, `tests/cli_output_policy.rs:101`.

### Review/report symlinked parent directory attacks

Covered by `tests/cli_output_policy.rs` through the compiled CLI binary:

- `make_review_rejects_symlinked_review_directory_without_writing_target` replaces `review/` with a symlink to an outside directory, runs `make-review`, expects an inside-case policy failure, and asserts outside `index.html` was not written.
- `make_report_rejects_symlinked_reports_directory_without_writing_target` replaces `reports/` with a symlink to an outside directory, runs `make-report`, expects an inside-case policy failure, and asserts outside `case-report.html` was not written.

Relevant lines: `tests/cli_output_policy.rs:79`, `tests/cli_output_policy.rs:136`.

The production path is also wired correctly for these tests: `make_review` and `make_report` call `require_case_output_path` before `write_text` for `review/index.html`, `review/evidence-viewer.html`, and `reports/case-report.html` (`src/cli/handlers.rs:263`, `src/cli/handlers.rs:295`, `src/cli/handlers.rs:343`). `require_case_output_path` canonicalizes the nearest existing parent and rejects parents resolving outside the case root, then rejects symlink leaf outputs with `symlink_metadata` (`src/tool_policy.rs:79`, `src/tool_policy.rs:187`).

### Existing derived-output policies

Covered by `src/derived_output_policy_tests.rs`:

- Proxy, thumbnail, frame capture, video export, and inode recovery all create dangling symlink output leaves and assert rejection before external tool execution and before outside target creation.

Relevant lines: `src/derived_output_policy_tests.rs:67`, `src/derived_output_policy_tests.rs:85`, `src/derived_output_policy_tests.rs:103`, `src/derived_output_policy_tests.rs:121`, `src/derived_output_policy_tests.rs:141`.

### Existing inventory export policies

Covered by `src/case_db/inventory_export_tests.rs` and CLI integration coverage in `tests/cli_inventory.rs`:

- Manifest export writes selected rows with output hash and paged-query policy metadata.
- Export rejects outside-case outputs.
- Export rejects existing outputs.
- Export rejects dangling symlink output leaves without creating the target.
- Export rejects registered source evidence output paths.
- CLI inventory flow verifies bounded SQLite-backed inventory JSON, inventory export manifest success, outside-case rejection, source path rejection, existing output rejection, and symlink output rejection.

Relevant lines: `src/case_db/inventory_export_tests.rs:5`, `src/case_db/inventory_export_tests.rs:40`, `src/case_db/inventory_export_tests.rs:82`, `src/case_db/inventory_export_tests.rs:118`, `tests/cli_inventory.rs:53`.

## Validation Evidence

- `cargo test --test cli_output_policy -- --nocapture` -> 4 passed, 0 failed.
  Evidence: `.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-write-text-symlink-fix/cargo-test-cli-output-policy.txt`
- `cargo test derived_output_policy_tests -- --nocapture` -> 5 passed, 0 failed.
  Evidence: `.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-write-text-symlink-fix/cargo-test-derived-output-policy.txt`
- `cargo test inventory_export_tests -- --nocapture` -> 4 passed, 0 failed.
  Evidence: `.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-write-text-symlink-fix/cargo-test-inventory-export-tests.txt`
- `cargo test --test cli_inventory inventory_commands_emit_bounded_sqlite_backed_json -- --nocapture` -> 1 passed, 0 failed.
  Evidence: `.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-write-text-symlink-fix/cargo-test-cli-inventory-policy.txt`
- `cargo test --test media_contract report_discloses_derived_provenance_and_validation_failures -- --nocapture` -> 1 passed, 0 failed.
  Evidence: `.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-write-text-symlink-fix/cargo-test-media-contract-derived-report.txt`
- `cargo test` -> 117 lib tests, 14 integration tests, and 0 doc tests passed; 0 failed.
  Evidence: `.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-write-text-symlink-fix/cargo-test-full.txt`

## Worktree

No tracked source/test diff was present after QA. The only files written by this review are the requested review report and QA evidence/notepad under `.omo/ulw-loop/frame-production-exec-20260623/`.

APPROVE
