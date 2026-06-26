# Final QA Review After Derived Symlink Fix

Date: 2026-06-23
Repository: `/Users/shinyoohag/Desktop/frametrace`
QA scope: read-only validation of latest HEAD `151fd5c` for FrameTrace derived-output and unique-path symlink policy fixes.

## Verdict

APPROVE

HEAD `151fd5c` passes the focused symlink regression surface and the full locked Cargo suite. The requested prior evidence files are present and non-empty. Runtime cleanup inspection found no FrameTrace listener processes and no leftover `frametrace-unique-path-symlink-test-*` temp directories after the HEAD validation run.

Note: an earlier interrupted pass against prior commit context captured a full-suite failure in `util::tests::unique_path_treats_dangling_symlink_as_occupied`; the task update superseded that target with HEAD `151fd5c`, where the focused and full suites both pass.

## Checks

- `cargo test --locked symlink -- --nocapture` at HEAD `151fd5c`: PASS, 9 symlink-filtered tests passed.
- `cargo test --locked` at HEAD `151fd5c`: PASS, full suite passed.
- `git diff --check` at HEAD `151fd5c`: PASS.
- `.omo/ulw-loop/frame-production-exec-20260623/evidence/derived-output-symlink-policy-fix.txt`: present and non-empty.
- `.omo/ulw-loop/frame-production-exec-20260623/evidence/unique-path-symlink-policy-fix.txt`: present and non-empty.
- Cleanup: no matching FrameTrace symlink temp dirs, no FrameTrace listener processes.

## manualQa

### surfaceEvidence

| scenario id | criterion reference | surface | exact invocation | verdict | artifactRefs |
|---|---|---|---|---|---|
| surface-repo-head | C0 HEAD under review | CLI repository inspection | `git -C /Users/shinyoohag/Desktop/frametrace status --short && git -C /Users/shinyoohag/Desktop/frametrace log -1 --oneline --decorate && git -C /Users/shinyoohag/Desktop/frametrace show --stat --oneline --decorate --name-only 151fd5c` | PASS | artifact-repo-head |
| surface-policy-evidence | C1 Prior policy evidence exists | CLI artifact inspection | `test -s .omo/ulw-loop/frame-production-exec-20260623/evidence/derived-output-symlink-policy-fix.txt && sed -n '1,260p' .omo/ulw-loop/frame-production-exec-20260623/evidence/derived-output-symlink-policy-fix.txt; test -s .omo/ulw-loop/frame-production-exec-20260623/evidence/unique-path-symlink-policy-fix.txt && sed -n '1,260p' .omo/ulw-loop/frame-production-exec-20260623/evidence/unique-path-symlink-policy-fix.txt` | PASS | artifact-policy-evidence |
| surface-focused-symlink-tests | C2 Focused symlink regressions | CLI test runner | `cargo test --locked symlink -- --nocapture` | PASS | artifact-focused-symlink-tests |
| surface-full-locked-suite | C3 Full locked suite | CLI test runner | `cargo test --locked` | PASS | artifact-full-locked-suite |
| surface-diff-check | C4 Whitespace/diff hygiene | CLI whitespace checker | `git diff --check` | PASS | artifact-diff-check |
| surface-runtime-cleanup | C5 Temp/runtime cleanup | CLI runtime cleanup inspection | `find /var/folders/32/z9wr5xpx5yzbyyynfrz1kjq80000gn/T -maxdepth 1 -name 'frametrace-unique-path-symlink-test-*' -print; tmux ls 2>/dev/null || true; lsof -nP -iTCP -sTCP:LISTEN 2>/dev/null \| grep -i frametrace || true` | PASS | artifact-runtime-cleanup |

### adversarialCases

| scenario id | criterion reference | adversarial class | expected behavior | verdict | artifactRefs |
|---|---|---|---|---|---|
| adv-dangling-derived-output | C2 | dangling final symlink output for derived artifact commands | `make-proxy`, thumbnail, frame capture, export-video, and recover-inode reject dangling final symlink outputs before reaching `ffmpeg` or `icat`. | PASS | artifact-focused-symlink-tests, artifact-policy-evidence |
| adv-unique-path-dangling-symlink | C2 | dangling symlink leaf in unique output path allocation | `unique_path` treats a dangling symlink leaf as occupied and selects a suffixed output path instead of writing through the symlink. | PASS | artifact-focused-symlink-tests, artifact-policy-evidence |
| adv-full-suite-regression | C3 | regression outside focused filter | Full locked suite remains green with symlink policy changes included. | PASS | artifact-full-locked-suite |
| adv-leftover-runtime-state | C5 | stale temp/runtime state | QA leaves no FrameTrace listener processes and no `frametrace-unique-path-symlink-test-*` temp directories. | PASS | artifact-runtime-cleanup |

### artifactRefs

| id | kind | description | path |
|---|---|---|---|
| artifact-repo-head | cli-transcript | HEAD `151fd5c` status and commit context. | `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-derived-symlink-fix/repo-context-head-151fd5c.txt` |
| artifact-policy-evidence | cli-transcript | Inspection of derived-output and unique-path symlink policy evidence files. | `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-derived-symlink-fix/policy-evidence-files-head-151fd5c.txt` |
| artifact-focused-symlink-tests | cli-transcript | Focused symlink test run at HEAD `151fd5c`, including derived-output and unique-path symlink tests. | `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-derived-symlink-fix/cargo-test-symlink-nocapture-head-151fd5c.txt` |
| artifact-full-locked-suite | cli-transcript | Full `cargo test --locked` run at HEAD `151fd5c`. | `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-derived-symlink-fix/cargo-test-locked-head-151fd5c.txt` |
| artifact-diff-check | cli-transcript | `git diff --check` run at HEAD `151fd5c`. | `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-derived-symlink-fix/git-diff-check-head-151fd5c.txt` |
| artifact-runtime-cleanup | cli-transcript | Cleanup inspection showing no symlink-test temp dirs and no FrameTrace listener processes. | `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-derived-symlink-fix/runtime-cleanup-inspection-head-151fd5c.txt` |

APPROVE
