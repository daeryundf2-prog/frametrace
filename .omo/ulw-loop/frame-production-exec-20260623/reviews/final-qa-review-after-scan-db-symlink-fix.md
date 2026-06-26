# Final QA Review After Scan DB Symlink Fix

Repository: `/Users/shinyoohag/Desktop/frametrace`  
Branch: `codex/frametrace-forensic-hardening`  
Verified HEAD: `f589deaafa1ef8d8c036b1734b6ca3230a266db3`  
Evidence directory: `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-scan-db-symlink-fix/`

## Summary

All required command surfaces were executed directly from the repository CLI. Every required check exited 0. Focused regression coverage included output path policy, symlink defenses, inventory/query, media/report DB selectors, full locked test suite, whitespace diff check, and cleanup receipt. No production code was changed.

## manualQa

### surfaceEvidence

| scenario id | criterion reference | surface | exact invocation | verdict | artifactRefs |
|---|---|---|---|---|---|
| SE-001 | provenance: target HEAD is f589dea | CLI | `git rev-parse HEAD` | PASS: stdout was `f589deaafa1ef8d8c036b1734b6ca3230a266db3`; exit 0 | A01 |
| SE-002 | formatting gate | CLI | `cargo fmt --all -- --check` | PASS: exit 0 | A02 |
| SE-003 | clippy gate with warnings denied | CLI | `cargo clippy --locked --all-targets --all-features -- -D warnings` | PASS: exit 0 | A03 |
| SE-004 | output path policy regression | CLI | `cargo test --locked --test cli_output_policy -- --nocapture` | PASS: 5 passed, 0 failed; exit 0 | A04 |
| SE-005 | symlink regression selector | CLI | `cargo test --locked symlink -- --nocapture` | PASS: selected symlink tests passed; exit 0 | A05 |
| SE-006 | inventory/query regression | CLI | `cargo test --locked --test cli_inventory -- --nocapture` | PASS: 1 passed, 0 failed; exit 0 | A06 |
| SE-007 | media/report DB regression selector | CLI | `cargo test --locked case_db:: -- --nocapture` | PASS: 18 passed, 0 failed; exit 0 | A07 |
| SE-008 | full locked suite | CLI | `cargo test --locked` | PASS: full suite passed; exit 0 | A08 |
| SE-009 | whitespace/diff hygiene | CLI | `git diff --check` | PASS: exit 0 | A09 |
| SE-010 | QA cleanup receipt | CLI/OS | `ps -axo pid,ppid,stat,command \| awk QA-specific patterns; tmux ls \| grep QA-specific sessions; find /tmp maxdepth 3 and TMPDIR maxdepth 1 for QA temp roots` | PASS: no QA-specific live processes, tmux sessions, or temp roots; cleanup command exit codes 0 | A10 |

### adversarialCases

| scenario id | criterion reference | adversarial class | expected behavior | verdict | artifactRefs |
|---|---|---|---|---|---|
| ADV-001 | scan-folder DB symlink escape fix | Symlinked case DB directory escaping case root | `scan-folder` must reject a symlinked `case/db` directory and must not write `case.db`, JSONL/TSV indexes, or scan state outside the case directory | PASS: `scan_folder_rejects_symlinked_db_directory_without_writing_case_state_outside` passed in output policy integration test | A04 |
| ADV-002 | report/review output symlink defenses | Symlinked report/review output leaf or directory | `make-report` and `make-review` must reject symlinked report/review outputs and avoid writing outside targets | PASS: four report/review symlink-output tests passed in output policy integration test | A04 |
| ADV-003 | symlink handling regression set | Symlink-focused behavior across crate tests | Symlink-specific selectors must remain green with no selected failures | PASS: `cargo test --locked symlink -- --nocapture` exited 0 with selected symlink tests passing | A05 |
| ADV-004 | inventory/query bounded output | SQLite-backed inventory/query surface | Inventory commands must emit bounded SQLite-backed JSON without regressions | PASS: `inventory_commands_emit_bounded_sqlite_backed_json` passed | A06 |
| ADV-005 | media/report DB state | Case DB media/report selector | Case DB media/report behavior must remain green under `case_db::` selector | PASS: 18 selected tests passed | A07 |
| ADV-006 | misleading success output | Command success text with hidden nonzero exit | PASS requires both success output and captured `--- exit_code: 0 ---` marker for every command artifact | PASS: all command artifacts A01-A10 include exit 0 markers | A01, A02, A03, A04, A05, A06, A07, A08, A09, A10 |
| ADV-007 | cleanup completeness | Leftover QA temp/process/browser/worker state | No QA-specific live process, tmux session, browser worker, or temp root may remain after QA | PASS: final cleanup receipt contains empty QA-specific process/session/temp sections and cleanup command exit codes 0 | A10 |

### artifactRefs

| id | kind | description | path |
|---|---|---|---|
| A00 | note | Bootstrap/tier/success-criteria note | `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-scan-db-symlink-fix/bootstrap-notepad.md` |
| A01 | command transcript | `git rev-parse HEAD` | `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-scan-db-symlink-fix/command-01-git-rev-parse-head.txt` |
| A02 | command transcript | `cargo fmt --all -- --check` | `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-scan-db-symlink-fix/command-02-cargo-fmt-check.txt` |
| A03 | command transcript | `cargo clippy --locked --all-targets --all-features -- -D warnings` | `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-scan-db-symlink-fix/command-03-cargo-clippy.txt` |
| A04 | command transcript | `cargo test --locked --test cli_output_policy -- --nocapture` | `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-scan-db-symlink-fix/command-04-cli-output-policy.txt` |
| A05 | command transcript | `cargo test --locked symlink -- --nocapture` | `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-scan-db-symlink-fix/command-05-cargo-test-symlink.txt` |
| A06 | command transcript | `cargo test --locked --test cli_inventory -- --nocapture` | `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-scan-db-symlink-fix/command-06-cli-inventory.txt` |
| A07 | command transcript | `cargo test --locked case_db:: -- --nocapture` | `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-scan-db-symlink-fix/command-07-case-db.txt` |
| A08 | command transcript | `cargo test --locked` | `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-scan-db-symlink-fix/command-08-cargo-test-locked.txt` |
| A09 | command transcript | `git diff --check` | `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-scan-db-symlink-fix/command-09-git-diff-check.txt` |
| A10 | cleanup receipt | Final QA-specific cleanup receipt | `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-scan-db-symlink-fix/cleanup-receipt-final.txt` |
| A11 | cleanup receipt | Broad initial cleanup scan, retained as supplemental context | `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-scan-db-symlink-fix/cleanup-receipt.txt` |
| A12 | cleanup receipt | QA-specific cleanup scan with protected macOS subtree note, superseded by A10 | `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-scan-db-symlink-fix/cleanup-receipt-precise.txt` |

## Command Results

| command | result |
|---|---|
| `git rev-parse HEAD` | PASS, exit 0, `f589deaafa1ef8d8c036b1734b6ca3230a266db3` |
| `cargo fmt --all -- --check` | PASS, exit 0 |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | PASS, exit 0 |
| `cargo test --locked --test cli_output_policy -- --nocapture` | PASS, 5 passed, 0 failed, exit 0 |
| `cargo test --locked symlink -- --nocapture` | PASS, selected symlink tests passed, exit 0 |
| `cargo test --locked --test cli_inventory -- --nocapture` | PASS, 1 passed, 0 failed, exit 0 |
| `cargo test --locked case_db:: -- --nocapture` | PASS, 18 passed, 0 failed, exit 0 |
| `cargo test --locked` | PASS, full locked suite passed, exit 0 |
| `git diff --check` | PASS, exit 0 |
| cleanup receipt | PASS, no QA-specific live processes, tmux sessions, or temp roots; cleanup command exit codes 0 |

## Cleanup

The final cleanup receipt shows empty QA-specific process, tmux-session, `/tmp`, and top-level `$TMPDIR` sections. It also records `find_tmp=0` and `find_tmpdir=0`. Existing unrelated untracked ULW artifacts and unrelated user Chrome/tmux processes were not modified.

APPROVE
