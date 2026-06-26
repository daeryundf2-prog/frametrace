# Final Security Review After Log Output Symlink Fix

Repository: `/Users/shinyoohag/Desktop/frametrace`
Reviewed HEAD: `a961661e52d0b04d08c8c835f291596754fb5352` (`Block forensic log symlink escapes`)
Previous commit reviewed: `552b3fc` (`Block default artifact symlink escapes`)
Scope: `src/tsk.rs`, `src/e01.rs`, `src/validation.rs`, `src/playback.rs`, `tests/cli_default_output_policy.rs`, `tests/cli_tsk_log_output_policy.rs`, `tests/cli_e01_validation_log_output_policy.rs`

## Findings By Severity

### CRITICAL

None.

### HIGH

None.

### MEDIUM

None.

### LOW

1. Pre-existing maintainability risk: the touched production modules remain oversized under the loaded programming skill's 250 pure-LOC review lens.

   References:

   - `src/tsk.rs`: 600 pure LOC
   - `src/e01.rs`: 403 pure LOC
   - `src/validation.rs`: 459 pure LOC

   Security impact: not a blocker for this final security review. The diff is a scoped hardening patch and does not introduce a security-relevant abstraction layer or untyped escape hatch. Splitting these modules would be broader refactoring work and would increase risk if folded into this symlink fix.

2. Residual local-concurrency watch item: `audit::append_chained_jsonl` remains a read-modify-write append, not an OS-level atomic append.

   References:

   - `src/audit.rs:74`
   - `src/audit.rs:98`

   Security impact: not a blocker for the reviewed threat in this patch. The fixed paths now validate the case-owned output parent before calling the append helper. A concurrent local process with write access to the same case directory could still race audit appends, but that is pre-existing behavior outside the requested symlink/output-path closure.

## Prior Blockers

The blockers from `final-security-review-after-default-output-symlink-fix.md` are closed.

- TSK `inspect-image` log writes: closed. `src/tsk.rs:138` guards `mmls` logs and `src/tsk.rs:172` guards `fls` logs before any write.
- TSK filesystem db outputs: closed. `src/tsk.rs:200` guards `tsk-files-*.jsonl` and `src/tsk.rs:214` guards `tsk-inspection-*.json`.
- TSK audit append path: closed. `src/tsk.rs:401` to `src/tsk.rs:407` resolves the audit log through `require_case_output_path` and rejects equality with the selected source image path before `append_tsk_audit_at`.
- E01 info, verify, and export logs: closed. `src/e01.rs:53`, `src/e01.rs:93`, `src/e01.rs:102`, and `src/e01.rs:110` guard generated log paths before writes or external `-l` arguments.
- E01 audit append path: closed. `src/e01.rs:361` to `src/e01.rs:364` applies the same case-containment and source-output equality checks before append.
- Validation/playback log appends: closed for the reviewed symlinked log-parent class. `src/validation.rs:206` to `src/validation.rs:208` and `src/playback.rs:55` to `src/playback.rs:57` guard `validation-log.jsonl` before append.

## Security Review Notes

- Original evidence immutability: no remaining reviewed path can write through a symlinked `evidence/logs` or `db/filesystem` parent into the selected source evidence path. The TSK and E01 audit-log helpers additionally reject output equality with the source evidence path.
- Source path write prevention: `reject_source_output_path` is now used for TSK and E01 audit/log surfaces where a fixed generated path could otherwise coincide with a selected source evidence path.
- Symlinked case-owned parents: the guard in `src/tool_policy.rs:79` to `src/tool_policy.rs:107` canonicalizes the nearest existing parent and rejects parents resolving outside the canonical case root. The new call sites invoke it before writes or before passing external tool log-output arguments.
- External tool output args: `ewfverify -l` and `ewfexport -l` receive only paths checked by `require_e01_output_path`; TSK `mmls`/`fls` outputs are captured and written only after checked case-contained log paths.
- Path traversal: `require_case_output_path` uses lexical absolute normalization plus canonical parent containment. Reviewed generated and explicit outputs in the diff are bounded by that helper before mutation.
- Durable mutation audit chain: the symlink escape that allowed audit/log appends outside the case is closed. The audit hash-chain logic itself is unchanged by this commit.

## Skill Perspective Check

The required skill-perspective check ran.

- `omo:remove-ai-slops` was loaded and applied as an overfit/slop review pass. The new tests are not deletion-only, not tautological, and do not merely verify a requested removal. They drive the real CLI with symlinked parents and fake external tools, assert command failure, and assert no outside/source mutation. The production diff does not add unnecessary parsing, normalization, or speculative data extraction.
- `omo:programming` plus the Rust reference were loaded and applied. The diff does not add `unsafe`, production `unwrap`/`expect`, untyped escape hatches, or brittle implementation-mirroring tests. The pure-LOC ceiling flags pre-existing oversized production modules; I recorded that as LOW maintainability risk, not a security blocker for this scoped fix.
- `security-review` was loaded from `/Users/shinyoohag/.agents/skills/security-review/SKILL.md` after the `/Users/shinyoohag/.codex/skills/security-review/SKILL.md` path was unavailable. Its path traversal, command-injection, secrets, and logging/monitoring checks were applied to the scoped diff. No CRITICAL or HIGH security issue was found.
- `code-review` was loaded as a quality review lens. Independent subagent lanes were not spawned because this task is an explicit final-review assignment and the available subagent tool contract restricts spawning to explicit user requests for subagents. The review is therefore direct and artifact-backed.

## Verification Evidence

Evidence directory: `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/reviews/final-security-review-after-log-output-symlink-fix-evidence`

- Requested diff captured at `requested-diff.txt` with 852 lines.
- `cargo test --locked --test cli_default_output_policy -- --nocapture`: PASS, 4 passed, log `cargo-test-cli-default-output-policy.log`.
- `cargo test --locked --test cli_tsk_log_output_policy -- --nocapture`: PASS, 3 passed, log `cargo-test-cli-tsk-log-output-policy.log`.
- `cargo test --locked --test cli_e01_validation_log_output_policy -- --nocapture`: PASS, 4 passed, log `cargo-test-cli-e01-validation-log-output-policy.log`.
- `cargo test --locked symlink -- --nocapture`: PASS, including 9 filtered unit symlink tests, 4 default-output CLI tests, 4 E01/validation CLI tests, 5 existing output-policy CLI tests, and 3 TSK log-output CLI tests; log `cargo-test-symlink.log`.
- `git diff 552b3fc..HEAD --check -- <scoped files>`: PASS with no whitespace/error output.

## Verdict

Security status: CLEAR
codeQualityStatus: WATCH
recommendation: APPROVE
blockers: none

APPROVE
