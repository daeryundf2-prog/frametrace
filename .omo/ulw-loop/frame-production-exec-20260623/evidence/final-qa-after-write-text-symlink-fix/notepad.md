# Final QA Notepad - write_text symlink fix

Tier: HEAVY.
Justification: final QA review requested for security-sensitive symlink attack coverage and policy regression gates.

Skills surveyed:
- tdd: relevant to test adequacy and RED/GREEN evidence, but this turn is read-only QA rather than authoring tests.
- code-review/review-work: relevant to final gate posture, but native subagent spawning is unavailable to this leaf reviewer; performing direct evidence-based review instead.
- omo:programming: relevant because this is Rust source/test inspection; using repo patterns and cargo validation.
- omo:lsp: relevant for diagnostics if compile/test failures need source-level narrowing; not needed before cargo test evidence.

Binding deliverable:
- Write final QA review at `.omo/ulw-loop/frame-production-exec-20260623/reviews/final-qa-review-after-write-text-symlink-fix.md`.
- Final line of that review must be exactly `APPROVE` or `BLOCKED`.

Success criteria:
1. Confirm HEAD is `c6a7abc` and do not modify source/test files.
2. Verify tests cover review/report final leaf symlink attacks.
3. Verify tests cover review/report symlinked parent directory attacks.
4. Verify existing derived-output policy tests and inventory export policy tests still cover outside-case, existing-output/source-output, and symlink-output constraints.
5. Run targeted tests plus enough full validation to support a final gate decision.

Scenario commands:
- `cargo test --test cli_output_policy -- --nocapture` -> PASS requires 4 review/report symlink policy tests pass.
- `cargo test derived_output_policy_tests -- --nocapture` -> PASS requires derived output symlink policy tests pass.
- `cargo test inventory_export_tests -- --nocapture` -> PASS requires inventory export policy tests pass.
- `cargo test --test cli_inventory inventory_commands_emit_bounded_sqlite_backed_json -- --nocapture` -> PASS requires CLI inventory export policy path pass.
- `cargo test --test media_contract report_discloses_derived_provenance_and_validation_failures -- --nocapture` -> PASS requires derived provenance report contract pass.
- `cargo test` -> PASS requires full suite green.

Review notes:
- `tests/cli_output_policy.rs` exercises the real compiled CLI binary and asserts both error text and that external symlink targets are not written.
- `src/cli/handlers.rs` calls `require_case_output_path` before `write_text` for review, evidence viewer, and report outputs.
- `src/tool_policy.rs` canonicalizes nearest existing parent and rejects symlink final leaf; this covers symlinked parent directories and final leaf symlinks before writes.
