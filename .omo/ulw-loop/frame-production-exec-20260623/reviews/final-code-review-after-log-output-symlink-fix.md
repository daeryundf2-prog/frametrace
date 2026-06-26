# Final Code-Quality Review: log output symlink fix

Repository: `/Users/shinyoohag/Desktop/frametrace`  
Branch: `codex/frametrace-forensic-hardening`  
HEAD reviewed: `a961661` (`Block forensic log symlink escapes`)  
Diff reviewed: `552b3fc..HEAD`  
Scope: `src/tsk.rs`, `src/e01.rs`, `src/validation.rs`, `src/playback.rs`, `tests/cli_default_output_policy.rs`, `tests/cli_tsk_log_output_policy.rs`, `tests/cli_e01_validation_log_output_policy.rs`

## Findings

### CRITICAL

None.

### HIGH

None.

### MEDIUM

None.

### LOW

1. `src/tsk.rs:121` and `src/e01.rs:45` remain oversized production modules under the `omo:programming` 250 pure-LOC perspective.

   Evidence: current pure LOC is 600 for `src/tsk.rs` and 403 for `src/e01.rs`; at `552b3fc` they were already 584 and 379. This commit adds narrow output-policy guards and tiny local helpers rather than speculative abstractions, so I am not treating the pre-existing module-size debt as a blocker for this targeted security/output-policy fix. It remains a future refactor risk.

## Prior Blocker Closure

Prior blocker from `final-code-review-after-default-output-symlink-fix.md`: E01, TSK, validation, and playback durable logs plus TSK `db/filesystem` outputs bypassed `require_case_output_path`, allowing symlinked `case/evidence/logs` and `case/db/filesystem` parents to redirect writes outside the canonical case tree.

Status: closed.

Evidence in the reviewed diff:
- `src/e01.rs:53`, `src/e01.rs:93`, `src/e01.rs:102`, `src/e01.rs:110`, and `src/e01.rs:361` preflight E01 info, verify, export, and audit logs before writes or before passing log paths to libewf tools.
- `src/tsk.rs:138`, `src/tsk.rs:172`, `src/tsk.rs:200`, `src/tsk.rs:214`, `src/tsk.rs:291`, and `src/tsk.rs:401` preflight TSK log, filesystem DB, recovered inode, and audit log paths.
- `src/validation.rs:206` and `src/playback.rs:55` preflight `validation-log.jsonl` before appending.
- `src/tool_policy.rs:79` canonicalizes the case root and nearest existing output parent, rejects parents resolving outside the case root, and rejects symlink output leaves.

## Skill-Perspective Check

Ran/consulted before judging test relevance and maintainability:
- `omo:remove-ai-slops`: applied the overfit/slop pass to production and tests.
- `omo:programming` plus Rust CLI/code-smell references: applied Rust path-boundary, test-shape, typed-boundary, file-size, and maintainability criteria.
- `code-review`: loaded for severity and report structure. Independent subagent lanes were not run because the available `spawn_agent` tool explicitly forbids spawning unless the user asks for delegation; this report is a direct artifact-backed review.

Perspective result:
- No deletion-only tests, tautological tests, prompt-string tests, or implementation-constant mirroring found.
- The new tests drive the compiled CLI through observable symlink-parent failure cases and assert the outside target remains unwritten.
- No needless production parsing, normalization, untyped escape hatch, or speculative abstraction was introduced for this goal.
- Non-blocking programming concern: the touched production modules were already oversized and remain oversized, noted under LOW.

## Verification Performed

Required commands:
- `git show --stat --oneline --decorate HEAD`: PASS. Output identified `a961661 (HEAD -> codex/frametrace-forensic-hardening) Block forensic log symlink escapes`; 7 files changed, 602 insertions, 47 deletions.
- `git diff 552b3fc..HEAD -- src/tsk.rs src/e01.rs src/validation.rs src/playback.rs tests/cli_default_output_policy.rs tests/cli_tsk_log_output_policy.rs tests/cli_e01_validation_log_output_policy.rs`: inspected. Diff adds output preflights for prior blocker paths and focused CLI regressions.
- `cargo test --locked --test cli_tsk_log_output_policy -- --nocapture`: PASS, 3 passed.
- `cargo test --locked --test cli_e01_validation_log_output_policy -- --nocapture`: PASS, 4 passed.

Additional repository commands run or inspected:
- `git status --short`
- `rg -n "append_chained_jsonl|write_text\\(|File::create|OpenOptions|\\.join\\(\"evidence/logs|\\.join\\(\"db/filesystem" ...`
- `rg -n "require_case_output_path\\(|require_e01_output_path\\(|tsk_audit_log_path\\(|e01_audit_log_path\\(|append_tsk_audit_at|append_e01_audit_at" ...`
- `nl -ba` reads for the focused production and test files.
- Pure LOC checks with `awk` for `src/tsk.rs`, `src/e01.rs`, and the new focused test files, plus baseline LOC checks from `552b3fc`.
- Inspected prior review `reviews/final-code-review-after-default-output-symlink-fix.md`.
- Inspected executor evidence summary `evidence/log-output-symlink-policy-fix.txt` and QA command summary under `evidence/final-qa-after-log-output-symlink-fix/`.
- Attempted `codegraph_explore`; unavailable because this repo has no `.codegraph/` index.

## Review Notes

The implementation guards output paths before writing or before passing paths to external commands that can write logs. The new tests are appropriately black-box for this CLI: they create symlinked case-owned parents, invoke the actual `frametrace` binary, expect failure containing `inside the case directory`, and verify the outside targets were not created or appended.

The only residual concern is structural size debt in existing production modules. Blocking this commit on a module split would expand the scope beyond the targeted security/output-policy closure, and I found no evidence that the added guards make the design materially harder to maintain.

## Verdict

codeQualityStatus: CLEAR  
recommendation: APPROVE  
reportPath: `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/reviews/final-code-review-after-log-output-symlink-fix.md`  
blockers: none

APPROVE
