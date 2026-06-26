# Findings

No security blockers found in c74e897.

The module split from a961661 to c74e897 preserves the symlink and source-output protections for the requested E01, TSK, validation, playback, and log paths. I found no newly introduced fallback/workaround branch that masks failures or bypasses the primary output-policy contract.

# Evidence

Review scope:
- HEAD verified as c74e8974f30e1bbada49f83e6100fceb2dc49528.
- Diff reviewed against a961661 for 11 Rust files: src/e01.rs, src/e01/commands.rs, src/e01/output_policy.rs, src/tsk.rs, src/tsk/audit_log.rs, src/tsk/commands.rs, src/tsk/parse.rs, src/tsk/types.rs, src/validation.rs, src/validation/log.rs, src/validation/target.rs.
- The change is a responsibility split: E01 command helpers moved to src/e01/commands.rs, E01 log/output policy moved to src/e01/output_policy.rs, TSK command/audit/parse/type helpers moved under src/tsk/, and validation target/log helpers moved under src/validation/.

Source-output and symlink policy:
- src/tool_policy.rs:79 keeps `require_case_output_path`, which canonicalizes the case root and nearest existing parent, rejects parents resolving outside the case directory, and rejects an existing symlink output leaf.
- src/tool_policy.rs:109 keeps `reject_source_output_path`, which canonicalizes the source and resolves the output path before rejecting source-equal outputs.
- src/util.rs:38 keeps `write_text` guarded by `reject_symlink_leaf` before `fs::write`, preserving a second leaf-symlink defense for text writes.
- src/e01/output_policy.rs:5 keeps `require_e01_output_path` as `require_case_output_path` plus `reject_source_output_path`; src/e01.rs:58, 98, 107, 115, and 116 route E01 info/verify/export/audit log paths through that helper before write/append.
- src/e01.rs:143 keeps the E01 raw output inside the case directory via `require_case_output_path`, and src/e01.rs:144 rejects an existing output before export.
- src/tsk.rs:41, 75, 103, 117, and 194 keep mmls/fls/filesystem-summary/recovery outputs behind `require_case_output_path`; src/tsk/audit_log.rs:47 keeps the TSK audit log behind `require_case_output_path` plus `reject_source_output_path`.
- src/validation/log.rs:29 keeps validation log append behind `require_case_output_path`.
- src/playback.rs:37 keeps playback confirmation append behind `require_case_output_path` before appending to evidence/logs/validation-log.jsonl.
- src/validation/target.rs:17 preserves canonicalization of direct, indexed-source, and log-derived validation targets before probing/logging.

Tool remediation evidence:
- .omo/ulw-loop/frame-production-exec-20260623/evidence/oversized-rust-module-split.txt records rust-analyzer installation, `rust-analyzer diagnostics .` exit 0, ast-grep availability through npx, and module declaration confirmation for validation log/target, E01 commands/output_policy, and TSK audit_log/commands/parse/types.
- I also ran MCP LSP diagnostics directly on all 11 modified Rust files in this review; each returned "No diagnostics found."
- I ran `rust-analyzer diagnostics .`; it completed successfully. Reported items were weak warnings for cfg-disabled test/windows/unix blocks and a pre-existing weak warning in src/scan.rs, not errors in the split modules.
- I ran ast-grep module declaration confirmation: `mod log; mod target;`, `mod commands; mod output_policy;`, and `mod audit_log; mod commands; mod parse; mod types;` are present in the expected root modules.

Regression evidence:
- `cargo test --locked --test cli_e01_validation_log_output_policy -- --nocapture`: PASS, 4 passed. Covers E01 inspect/import symlinked logs, validation symlinked logs, and playback symlinked logs without outside writes/appends.
- `cargo test --locked --test cli_tsk_log_output_policy -- --nocapture`: PASS, 3 passed. Covers TSK inspect/recover symlinked logs and filesystem DB directory without outside writes.
- `cargo test --locked --test media_contract -- --nocapture`: PASS, 3 passed. Covers playback confirmation separation and validation precondition behavior.
- `cargo test --locked --lib e01:: -- --nocapture`: PASS, 5 passed.
- `cargo test --locked --lib tsk:: -- --nocapture`: PASS, 3 passed.
- `cargo test --locked --lib validation:: -- --nocapture`: PASS, 6 passed.
- `git diff --check a961661..c74e897`: PASS.
- `cargo fmt --all -- --check`: PASS.

# Residual Risks

- This was a focused security re-review of c74e897 against a961661 for the requested output-policy surfaces, not a full product security audit.
- The case-output checks remain path-based and local-filesystem focused. As before, they assume the case directory is not concurrently mutated by an untrusted process between policy check and write. I did not treat that as a blocker because it is not introduced by c74e897 and the existing regression suites cover the concrete symlink/source-output protections under review.
- The working tree contains untracked .omo evidence/review artifacts. I ignored unrelated untracked state except for the user-requested evidence file and this review report.

APPROVE
