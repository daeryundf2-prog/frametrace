# Final Code Review After Oversized Module Split

## Findings

No code-quality, security, or behavior-preservation findings were identified in `c74e897` relative to `a961661`.

Verification blocker: LSP diagnostics could not be completed because the configured Rust LSP server cannot start in this workspace. `mcp__lsp.status` reports Rust LSP installed, but diagnostics fail with `Unknown binary 'rust-analyzer' in official toolchain '1.94.0-aarch64-apple-darwin'`, followed by daemon timeout responses. Because the review contract requires LSP diagnostics on modified files before approval, this report cannot end in `APPROVE` even though compiler, clippy, fmt, and tests passed.

Tooling gap: `sg` / `ast-grep` is not installed (`command not found` for both), so AST-pattern checks were unavailable. I used targeted `rg` text searches for hardcoded secret-like assignments, `unwrap()/expect()/panic!/allow` escape hatches, and fallback/workaround indicators as a fallback. This did not surface code findings.

## Scope Reviewed

Reviewed current `HEAD` `c74e897` (`Split forensic tool modules by responsibility`) against previously approved `a961661` (`Block forensic log symlink escapes`). The diff touches 11 Rust files:

- `src/e01.rs`
- `src/e01/commands.rs`
- `src/e01/output_policy.rs`
- `src/tsk.rs`
- `src/tsk/audit_log.rs`
- `src/tsk/commands.rs`
- `src/tsk/parse.rs`
- `src/tsk/types.rs`
- `src/validation.rs`
- `src/validation/log.rs`
- `src/validation/target.rs`

`git diff a961661..HEAD --stat` reported 1161 insertions and 1066 deletions, consistent with extraction rather than new feature expansion.

## Size And Responsibility Evidence

Pure LOC counts using `awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\/\/|#|--)/' <file> | wc -l`:

- `src/e01.rs`: 237
- `src/e01/commands.rs`: 162
- `src/e01/output_policy.rs`: 20
- `src/tsk.rs`: 242
- `src/tsk/audit_log.rs`: 57
- `src/tsk/commands.rs`: 139
- `src/tsk/parse.rs`: 121
- `src/tsk/types.rs`: 93
- `src/validation.rs`: 62
- `src/validation/log.rs`: 222
- `src/validation/target.rs`: 193

All touched Rust modules and new submodules are at or below 250 pure LOC. `src/e01.rs`, `src/tsk.rs`, and `src/validation/log.rs` remain in the 200-250 warning band, but the split is responsibility-oriented rather than `SIZE_OK` fiction:

- `src/e01.rs` now owns E01 orchestration, while `src/e01/commands.rs` owns libewf command construction/execution and `src/e01/output_policy.rs` owns E01 output/audit path guards.
- `src/tsk.rs` now owns inspect/recover orchestration, while `src/tsk/types.rs`, `src/tsk/parse.rs`, `src/tsk/commands.rs`, and `src/tsk/audit_log.rs` own data types, parser logic, Sleuth Kit commands, and audit/summary formatting respectively.
- `src/validation.rs` now owns validation orchestration, while `src/validation/target.rs` owns selector/log target resolution and `src/validation/log.rs` owns validation status/log JSON.

The diff preserves behavior by moving existing helper bodies into cohesive submodules. The only signature-level adjustment observed is `ewfexport_args` taking `Option<u64>` (`options.max_bytes`) instead of the full `E01Options`, which narrows dependency surface without changing command output; `cargo test --locked --lib e01:: -- --nocapture` covers the resulting args.

## Symlink Output Policy Interaction

The `a961661` contract remains in force:

- `src/e01.rs:58`, `src/e01.rs:98`, `src/e01.rs:107`, and `src/e01.rs:115` require E01 log paths before external tool calls or writes.
- `src/e01/output_policy.rs:11` runs `require_case_output_path`, and `src/e01/output_policy.rs:12` runs `reject_source_output_path`, preserving the source-output escape guard from `a961661`.
- `src/e01.rs:143` still gates raw E01 output with `require_case_output_path`.
- `src/tsk.rs:41`, `src/tsk.rs:75`, `src/tsk.rs:103`, `src/tsk.rs:117`, and `src/tsk.rs:194` guard TSK log/db/recovery outputs before writes or external command output.
- `src/tsk/audit_log.rs:52` gates the filesystem audit log inside the case tree, and `src/tsk/audit_log.rs:54` rejects overlap with the source image path.
- `src/validation/log.rs:37` gates the validation log before append.

Regression tests for the symlink-output-policy interaction passed on clean rerun:

- `cargo test --locked --test cli_default_output_policy -- --nocapture`: 4 passed, 0 failed.
- `cargo test --locked --test cli_e01_validation_log_output_policy -- --nocapture`: 4 passed, 0 failed.
- `cargo test --locked --test cli_tsk_log_output_policy -- --nocapture`: 3 passed, 0 failed.

Note: an earlier parallel run of `cli_default_output_policy` failed one test with `frametrace binary should run: No such file or directory` while a concurrent `cargo build --bin frametrace` was running. Rerunning the suite alone passed, so I do not treat that as a code finding.

## Requested Command Evidence

- `git diff a961661..HEAD --stat`: 11 files changed, 1161 insertions, 1066 deletions.
- `cargo fmt --all -- --check`: exit 0.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: exit 0.
- `cargo test --locked --lib tsk:: -- --nocapture`: 3 passed, 0 failed.
- `cargo test --locked --lib e01:: -- --nocapture`: 5 passed, 0 failed.
- `cargo test --locked --lib validation:: -- --nocapture`: 6 passed, 0 failed.

Additional review evidence:

- `cargo build --locked --bin frametrace`: exit 0.
- `rg` hardcoded secret-like assignment sweep over touched files: no matches.
- `rg` direct `unwrap()/expect()/panic!/allow` sweep over touched files: direct `unwrap()/expect()` matches were confined to test modules.
- Root-cause fallback/workaround guard: no newly introduced broad alternate execution path, silent default return, `SIZE_OK`, or suppression attribute was found. Existing tolerated branches are explicit domain behavior: TSK `mmls` unavailability is logged as a warning while `fls` remains the authoritative listing path, and validation log target lookup skips missing JSONL logs while still returning an explicit not-found error when no target resolves.

## Residual Risks

- Approval is blocked by local tooling, not by a code finding: LSP diagnostics could not complete because `rust-analyzer` is unavailable for the active Rust toolchain.
- AST-grep structural checks could not run because neither `sg` nor `ast-grep` is installed. The fallback `rg` sweep is useful but not equivalent to AST-aware matching.
- The main orchestrator files are below the ceiling but still close to it (`src/e01.rs` 237 pure LOC, `src/tsk.rs` 242 pure LOC). Future edits should split before adding responsibilities there.

BLOCKED
