# T9 Programming + Remove-AI-Slops Review

Verdict: PASS
codeQualityStatus: WATCH
recommendation: APPROVE
blockers: none
confidence: 0.88

## Scope

Reviewed T9 only: current diff for `src/scan.rs` and `src/util.rs`, T9/T11 plan text, T9 done claim, prior T9 gate review, and T9 evidence transcripts under `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/`.

Skill-perspective check ran:
- Consulted `omo:programming` plus Rust-specific guidance: `programming/SKILL.md`, `programming/references/rust/README.md`, and `programming/references/rust/zero-cost-safety.md`.
- Consulted `omo:remove-ai-slops`: `remove-ai-slops/SKILL.md`.
- Result: the T9 diff satisfies the Rust streaming/error/test criteria for this task. It violates the raw programming size ceiling because `src/scan.rs` was already oversized and T9 adds to it, but that risk is explicitly assigned to T11 and is acceptable before T11 for this behavior-locking task.

## Findings By Severity

### CRITICAL

None.

### HIGH

None.

### MEDIUM

Deferred oversized-module risk: `src/scan.rs` is 1061 pure LOC now and was 897 pure LOC at `HEAD`, so T9 worsens an already oversized file. T9 adds focused streaming helpers/tests at `src/scan.rs:259`, `src/scan.rs:285`, `src/scan.rs:300`, `src/scan.rs:308`, and `src/scan.rs:1023`. T11 explicitly owns splitting `src/scan.rs` after behavior is pinned (`.omo/plans/frametrace-production-hardening-review-plan.md:175`). Immediate refactor is not required for T9 because the plan says T11 is blocked by T1-T10 and must not mix new behavior into refactor commits (`.omo/plans/frametrace-production-hardening-review-plan.md:176`). This should remain a T11 priority.

### LOW

`src/util.rs` is 242 pure LOC, up from 235 at `HEAD`, which puts it in the programming warning band but below the 250 pure-LOC defect threshold. The added `create_text_writer` at `src/util.rs:46` is acceptable because it preserves the same parent-directory and symlink-leaf policy as `write_text` for streamed output.

The fake-writer test includes supplemental write-count assertions at `src/scan.rs:1103` and `src/scan.rs:1104`. They are implementation-adjacent, but not blocking: the test also asserts returned `BrokenPipe` errors at `src/scan.rs:1101`, and the independent FIFO transcript verifies the user-visible failure path with nonzero exit and no success prose.

## Programming Rust Criteria

PASS.

- No added `unsafe` in the T9 diff.
- No added production `unwrap()` or `expect()`. Added unwrap-style calls are test-only at `src/scan.rs:1031`, `src/scan.rs:1036`, `src/scan.rs:1043`, `src/scan.rs:1045`, `src/scan.rs:1047`, `src/scan.rs:1049`, `src/scan.rs:1098`, and `src/scan.rs:1099`.
- No hidden full-string JSONL/TSV aggregation remains in the write path. `write_scan_outputs` now calls `write_case_stream` for `db/videos.jsonl` and `db/video_paths.tsv` at `src/scan.rs:259` and `src/scan.rs:265`; JSONL records are written row-by-row at `src/scan.rs:300`; TSV writes the header and each row at `src/scan.rs:308`.
- Typed result flow is preserved: writer helpers use `io::Result<()>` and propagate errors with `?`; the surrounding function maps those I/O errors into the existing `Result<(), String>` scan-output API at `src/scan.rs:291`.
- No weakened tests found. The diff adds compatibility tests and the transcripts show red, focused green, clippy, fmt, full test, and manual QA evidence.
- No new dependency is introduced by the reviewed T9 `src/scan.rs`/`src/util.rs` diff. The wider worktree has `Cargo.toml`/`Cargo.lock` changes for `serde`/`serde_json`, but those are outside T9's claimed changed files and outside this review scope.
- No obvious allocation regression for JSONL/TSV streaming. The former output-wide `String`/`String::from` aggregation was removed; TSV still allocates one row string at a time via `record.to_tsv_row()` at `src/scan.rs:313`, which is bounded per row rather than per file. The legacy JSON index remains full-string by design and declares `legacy_full_index` plus streaming alternatives at `src/scan.rs:649`.

## Remove-AI-Slops / Overfit / Test-Slop Criteria

PASS.

- No deletion-only tests and no tests that merely verify a requested removal were found in the T9 diff.
- Tests assert observable behavior: repeated scan row counts and duplicate prevention at `src/scan.rs:1051`; compatibility JSON policy keys at `src/scan.rs:1073`; writer failure propagation at `src/scan.rs:1101`. Manual QA also checks SQLite/JSONL/TSV counts and exit/failure state.
- No mock-call tautologies. `FailingWriter` is a narrow `Write` fake for I/O failure propagation, not a mocked production collaborator.
- No hardcoded temp paths. Tests use `std::env::temp_dir()` with process-scoped names; `/evidence/*.mp4` paths are inert record values in a pure writer test, not filesystem locations.
- No brittle exact prose beyond required compatibility metadata and blocker/failure keywords. The external failure probes assert exit status, state counts, and absence of success prose.
- No defensive redundant bloat found in production. `write_case_stream` is a small shared flow for two streamed outputs; `create_text_writer` centralizes the same output-path safety behavior as `write_text`.
- No broad one-off abstraction without need. The added helpers directly serve JSONL/TSV streaming and preserve output-boundary safety.
- No unnecessary production data extraction, parsing, or normalization beyond the T9 goal.

## Oversized File Disposition

Measured with:

```sh
awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\/\/|#|--)/' src/scan.rs | wc -l
awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\/\/|#|--)/' src/util.rs | wc -l
git show HEAD:src/scan.rs | awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\/\/|#|--)/' | wc -l
git show HEAD:src/util.rs | awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\/\/|#|--)/' | wc -l
```

Results:
- `src/scan.rs`: current 1061 pure LOC; `HEAD` 897 pure LOC. T9 worsens an existing oversized file.
- `src/util.rs`: current 242 pure LOC; `HEAD` 235 pure LOC. T9 moves it further into the warning band but not over the 250 defect threshold.

Disposition: acceptable before T11. T9 is a behavior-locking streaming change that the plan intentionally places before the oversized module split. T11 explicitly owns the split and requires behavior tests to pass unchanged. Immediate refactor in T9 would increase scope and conflict with the plan's "do not mix new behavior into refactor commits" constraint.

## Evidence Basis

Commands/inspections used:
- `sed`/`nl -ba` inspections of `.omo/plans/frametrace-production-hardening-review-plan.md`, `src/scan.rs`, and `src/util.rs`.
- `git diff -- src/scan.rs src/util.rs`
- `git diff -- src/scan.rs src/util.rs | rg ...` for added `unsafe`, `unwrap`, `expect`, hidden string aggregation, and test-slop patterns.
- `rg -n "String::new|String::from|push_str|write_jsonl_records|write_tsv_records|write_case_stream|create_text_writer|compatibility_json" src/scan.rs src/util.rs`
- Pure LOC `awk` measurements for current and `HEAD` versions of `src/scan.rs` and `src/util.rs`.
- Inspected `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/doneclaim.json`.
- Inspected `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/t9-gate-review.md`.
- Inspected T9 transcripts: `red-compatibility-streaming-tests.txt`, `focused-compatibility-green.txt`, `cargo-test-locked.txt`, `cargo-clippy.txt`, `cargo-fmt-check-rerun.txt`, `git-diff-check.txt`, `manual-qa-happy-path.txt`, `manual-qa-failure-path-rerun.txt`, and `t9-gate-midstream-fifo.txt`.

No fresh test command was rerun for this artifact; the review used source inspection plus existing T9 transcripts because the requested task was read-only review except for this report artifact.
