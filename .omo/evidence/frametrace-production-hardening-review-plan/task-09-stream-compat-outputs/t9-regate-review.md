recommendation: APPROVE
AdversarialVerify:
  verdict: confirmed
  confidence: 0.91

# T9 Re-Gate Review: Stream Compatibility Outputs

## originalIntent
T9 asked FrameTrace to stop constructing the JSONL and TSV compatibility outputs as full in-memory strings during scan output generation. The expected user-visible behavior is that repeated scans still produce complete, non-duplicated compatibility outputs, while legacy `db/video_index.json` behavior is explicit and release-safe.

## desiredOutcome
- `db/videos.jsonl` and `db/video_paths.tsv` are written through streaming writer APIs row-by-row.
- Repeated scan does not duplicate or drop rows across SQLite, JSONL, and TSV.
- Legacy compatibility JSON remains explicit, documented by output metadata/tests, and is not silently removed.
- Compatibility-output write failures fail closed: nonzero exit, no misleading success output, failed job recorded, and no new successful scan run/report-ready state.
- Evidence includes tests, manual QA, cleanup, and a T9-specific code-review/slop report covering `omo:remove-ai-slops` and `omo:programming` criteria.

## blockers
None.

## userOutcomeReview
The shipped T9 artifact now satisfies the desired user outcome. The previous gate rejected only because the T9 evidence set lacked a code-review/slop artifact with explicit skill coverage. That missing artifact now exists at `programming-remove-ai-slops-review.md`, carries `Verdict: PASS`, `codeQualityStatus: WATCH`, `blockers: none`, and explicitly covers `omo:remove-ai-slops`, `omo:programming`, overfit/test-slop criteria, and the `src/scan.rs` oversized-file disposition.

Direct source inspection supports the same conclusion:
- `src/scan.rs:259` and `src/scan.rs:265` call `write_case_stream` for JSONL and TSV compatibility outputs.
- `src/scan.rs:300` writes JSONL records row-by-row with `Write::write_all`.
- `src/scan.rs:308` writes the TSV header and one row at a time.
- `src/util.rs:46` adds `create_text_writer`, preserving parent-directory creation and symlink-leaf rejection before `File::create(...).map(BufWriter::new)`.
- `src/scan.rs:649` declares the legacy full compatibility JSON mode and streaming alternatives.

The remaining WATCH item is valid but not blocking for T9: `src/scan.rs` is oversized and worsened by T9, but `.omo/plans/frametrace-production-hardening-review-plan.md:175` assigns the module split to T11 after T8-T10 behavior locks are green, and line 176 says not to mix new behavior into refactor commits.

## checkedArtifactPaths
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/t9-gate-review.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/programming-remove-ai-slops-review.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/doneclaim.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/red-compatibility-streaming-tests.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/focused-compatibility-green.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/cargo-test-locked.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/cargo-clippy.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/cargo-fmt-check-rerun.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/cargo-build-locked.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/git-diff-check.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/manual-qa-happy-path.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/manual-qa-failure-path-rerun.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/t9-gate-midstream-fifo.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/cleanup-receipt-final.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/notepad.md`
- `.omo/plans/frametrace-production-hardening-review-plan.md`
- `src/scan.rs`
- `src/util.rs`

## commands
- `sed` inspections of required skills: `omo:remove-ai-slops`, `omo:programming`, `programming/references/rust/README.md`, and `programming/references/code-smells.md`.
- `pwd`
- `git status --short`
- `rg --files .omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs`
- `git diff --name-only`
- `git diff --stat`
- `sed` inspections of T9 prior gate, new review artifact, doneclaim, focused test transcript, FIFO probe, manual QA, cleanup, build/test transcripts, and notepad.
- `git diff -- src/scan.rs src/util.rs`
- `rg -n "T9|task-09|T11|scan.rs|module split|oversized" .omo/plans/frametrace-production-hardening-review-plan.md`
- `git diff --check`
- Pure LOC measurements for current and `HEAD` versions of `src/scan.rs` and `src/util.rs`.
- `git diff -- src/scan.rs src/util.rs | rg ...` for added `unsafe`, `unwrap`, `expect`, full-string aggregation, helpers, and test-slop patterns.
- `nl -ba src/scan.rs | sed -n '248,320p'`
- `nl -ba src/scan.rs | sed -n '640,660p'`
- `nl -ba src/scan.rs | sed -n '1019,1112p'`
- `nl -ba src/util.rs | sed -n '36,58p'`
- `find ${TMPDIR:-/tmp} -maxdepth 1 -name 'frametrace-t9-*' -print`
- `pgrep -fl 'target/debug/frametrace'`
- `git diff -- .omo/boulder.json .omo/start-work/ledger.jsonl .omo/plans/frametrace-production-hardening-review-plan.md`

## evidence
- Prior rejection reason: `t9-gate-review.md` rejected only on missing code-review/slop coverage and unsupported oversized-file disposition.
- New review artifact: `programming-remove-ai-slops-review.md` explicitly records `Verdict: PASS`, `codeQualityStatus: WATCH`, no blockers, direct `omo:programming` and `omo:remove-ai-slops` coverage, overfit/test-slop coverage, and `src/scan.rs` T11 deferral.
- TDD evidence remains valid: `red-compatibility-streaming-tests.txt` failed before helpers existed; `focused-compatibility-green.txt` passed 3 compatibility tests.
- Full-suite transcript remains valid evidence: `cargo-test-locked.txt` shows `exit_code=0`; I did not rerun full `cargo test --locked` during re-gate because no product code changed for this re-gate and the user said not to run it unless necessary.
- Static/build transcripts remain valid: `cargo-clippy.txt`, `cargo-fmt-check-rerun.txt`, and `cargo-build-locked.txt` all show exit 0.
- Fresh `git diff --check` returned exit 0 with no output.
- Manual happy QA remains valid: 3500 fixture files, two scans, SQLite/JSONL/TSV counts all 3500, `scan_runs=2`, compatibility mode `legacy_full_index`.
- Manual failure QA remains valid: unwritable/read-failing `db/videos.jsonl` path exited 1, recorded one failed job, kept `scan_runs` at 1, produced no success prose, and produced no report-ready artifacts.
- Fresh prior-gate FIFO probe remains valid: streamed TSV failure returned exit 1 with BrokenPipe, no success output, complete=1/failed=1, `scan_runs=1`, and cleanup removed the temp root.

## directSlopAndProgrammingPass
`omo:remove-ai-slops` direct pass:
- No deletion-only tests found.
- No tests merely verify a requested removal.
- No tautological tests that stop at existence/non-null assertions.
- The `FailingWriter` test has implementation-adjacent write-count assertions, but it also asserts `BrokenPipe` propagation and is backed by the independent FIFO CLI failure probe; this is not unresolved false-confidence slop.
- No implementation-mirroring-only tests block approval: repeated-scan counts, compatibility metadata, and failure-state checks assert observable compatibility contracts.
- No unnecessary production extraction, parsing, or normalization found. `write_case_stream`, `write_jsonl_records`, `write_tsv_records`, and `create_text_writer` are directly used by the T9 output path.
- No scope drift in the reviewed T9 production files beyond the explicit legacy compatibility metadata.

`omo:programming` Rust direct pass:
- No new `unsafe` in the T9 diff.
- No new production `unwrap()` or `expect()`; added unwrap-style calls are confined to tests.
- T9 writer helpers use `io::Result<()>`, `?`, and contextual `map_err` in the existing `Result<(), String>` boundary.
- JSONL/TSV no longer use output-wide `String::new` / `String::from` aggregation; TSV still allocates one row string at a time via `to_tsv_row()`, which is bounded per row.
- `src/util.rs` measures 242 pure LOC, below the 250 defect threshold but in the warning band.
- `src/scan.rs` measures 1061 pure LOC, up from 897 at `HEAD`. This is a real programming-size defect, but it is an accepted T9 WATCH risk because T11 explicitly owns the split after behavior locks.

## cleanupAssessment
- `cleanup-receipt-final.txt` shows recorded T9 temp roots removed and no `target/debug/frametrace` processes.
- Fresh `find ${TMPDIR:-/tmp} -maxdepth 1 -name 'frametrace-t9-*' -print` returned no roots.
- Fresh `pgrep -fl 'target/debug/frametrace'` exited 1 with no output.
- Protected orchestration files were not edited in this re-gate. `git diff -- .omo/boulder.json .omo/start-work/ledger.jsonl .omo/plans/frametrace-production-hardening-review-plan.md` returned no tracked diff; those paths remain untracked in the existing dirty worktree.

## AdversarialVerify
verdict: confirmed
confidence: 0.91

1. malformed_input: confirmed. Directory-path failure and FIFO BrokenPipe failure both fail closed with nonzero exit and no success state.
2. dirty_worktree: confirmed. `git status --short` shows a broad dirty worktree, but the re-gate inspected the T9 claimed product diff (`src/scan.rs`, `src/util.rs`) and did not edit protected product/plan/ledger files.
3. stale_state: confirmed. Happy-path manual QA used fresh temp roots and repeated scans; the FIFO probe used a separate temp root; fresh cleanup find shows no stale T9 temp roots.
4. misleading_success_output: confirmed. Failure evidence checks exit code, stderr, job status counts, `scan_runs`, absence of scan-complete prose, and absence of report-ready artifacts.
5. flaky_tests: confirmed. Red/focused-green/full-suite transcripts are internally consistent, and focused compatibility transcript passed all 3 compatibility tests.
6. hung_or_long_commands: confirmed. Existing transcripts include bounded elapsed times; fresh commands completed without hanging; no lingering `target/debug/frametrace` process remains.
7. prompt_injection: ruled out. T9 consumes filesystem records and CLI paths, not untrusted natural-language instructions.
8. cancel_resume: ruled out. T9 does not add resumable job semantics; failure handling records ordinary failed jobs without new scan-run success state.
9. repeated_interruptions: ruled out. T9 does not modify interrupt handling; no orphan process remained after QA/probes.

Cleanup: confirmed by receipt plus fresh `find` and `pgrep`.

## exactEvidenceGaps
No blocking evidence gaps remain. Full `cargo test --locked` was not rerun during this re-gate; the existing T9 transcript shows exit 0, and the fresh re-gate command was limited to `git diff --check` because the only post-rejection change was an evidence artifact, not product code.

## finalVerdict
APPROVE.
