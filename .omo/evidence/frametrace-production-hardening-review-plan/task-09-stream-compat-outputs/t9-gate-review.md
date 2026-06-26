recommendation: REJECT

# T9 Gate Review: Stream Compatibility Outputs

## originalIntent
T9 asked FrameTrace to stop constructing the JSONL and TSV compatibility outputs as full in-memory strings during scan output generation. The expected user-visible behavior is that repeated scans still produce complete, non-duplicated compatibility outputs, while legacy `db/video_index.json` behavior is explicit and release-safe.

## desiredOutcome
- `db/videos.jsonl` and `db/video_paths.tsv` are written through streaming writer APIs row-by-row.
- Repeated scan does not duplicate or drop rows across SQLite, JSONL, and TSV.
- Legacy compatibility JSON remains explicit, documented by output metadata/tests, and is not silently removed.
- Compatibility-output write failures fail closed: nonzero exit, no misleading success output, failed job recorded, and no new successful scan run/report-ready state.
- Evidence includes tests, manual QA, cleanup, and a T9-specific code-review/slop report covering `omo:remove-ai-slops` and `omo:programming` criteria.

## userOutcomeReview
The shipped code appears to satisfy the core product behavior:
- `src/scan.rs:259` and `src/scan.rs:265` now call `write_case_stream` for JSONL and TSV.
- `src/scan.rs:300` streams JSONL record lines with `Write::write_all` per record.
- `src/scan.rs:308` streams the TSV header and then one row at a time.
- `src/util.rs:46` adds `create_text_writer`, preserving parent creation and symlink-leaf rejection before `File::create(...).map(BufWriter::new)`.
- `src/scan.rs:649` declares `compatibility_json.mode = legacy_full_index`, `release_policy = compatibility-only`, and streaming alternatives.

Fresh focused verification passed, and I added a small independent FIFO probe for a real streamed TSV write failure. However, final approval is blocked because the artifact set lacks the required T9-specific code-review report with explicit `remove-ai-slops` overfit/slop coverage and `programming` coverage. The only T9 evidence mentioning those criteria is `notepad.md`; that is a worker self-note, not a code-review report.

## blockers
1. Missing required T9 code-review/slop report.
   - Evidence search: `rg -n "code review|review report|slop|overfit|tautolog|implementation-mirroring|programming|remove-ai|false confidence|scope drift|coverage" .omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs`
   - Result: only `notepad.md:6` mentions `omo:programming`; no T9 code-review report exists in the T9 evidence directory.
   - Why this blocks: the final-gate instructions require confirming that the code-review report explicitly covers the same skill-perspective checks and overfit/slop criteria. Absent report coverage is a mandatory rejection condition.
   - Minimal fix guidance: add a T9-specific code-review artifact under this evidence directory, from an independent review pass, explicitly covering `omo:remove-ai-slops` categories including deletion-only tests, tautological tests, implementation-mirroring tests, unnecessary extraction/parsing/normalization, false-confidence tests, scope drift, and `omo:programming` Rust criteria. It must justify carrying `src/scan.rs` until T11 or identify a required split.

2. Programming size-gate disposition is not supported by a review report.
   - Direct measurement: `src/scan.rs` is 1061 pure LOC; `src/util.rs` is 242 pure LOC.
   - T11 explicitly owns splitting `src/scan.rs`, so I am not treating the size alone as a product-code blocker for T9. But the required T9 code-review report is absent and therefore does not support the T11 deferral.
   - Minimal fix guidance: the code-review artifact should state why T9 may carry the pre-existing oversized `src/scan.rs` under the explicit T11 plan, or require moving the new compatibility tests/helpers into an existing or new responsibility split if the reviewer judges it necessary.

## checkedArtifactPaths
- `.omo/plans/frametrace-production-hardening-review-plan.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/doneclaim.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/red-compatibility-streaming-tests.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/focused-compatibility-green.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/cargo-test-locked.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/cargo-fmt-check-rerun.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/cargo-clippy.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/cargo-build-locked.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/git-diff-check.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/manual-qa-happy-path.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/manual-qa-failure-path-rerun.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/cleanup-receipt-final.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/notepad.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/t9-gate-midstream-fifo.txt`
- `src/scan.rs`
- `src/util.rs`

## commandsRun
- `cat /Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/remove-ai-slops/SKILL.md`
- `cat /Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/programming/SKILL.md`
- `cat /Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/programming/references/rust/README.md`
- `git status --short`
- `rg -n "T9|task-09|Stream JSONL|JSONL|TSV" .omo/plans/frametrace-production-hardening-review-plan.md`
- `rg --files .omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs`
- `git diff -- src/scan.rs src/util.rs`
- `cat` on T9 doneclaim, test transcripts, manual QA transcripts, cleanup receipt, and notepad
- `nl -ba src/scan.rs | sed -n '240,330p'`
- `nl -ba src/scan.rs | sed -n '600,670p'`
- `nl -ba src/scan.rs | sed -n '1000,1160p'`
- `nl -ba src/util.rs | sed -n '1,80p'`
- `rg -n "String::new\\(\\)|String::from\\(|videos\\.jsonl|video_paths\\.tsv|write_jsonl_records|write_tsv_records|compatibility_json|create_text_writer" src/scan.rs src/util.rs`
- `cargo test --locked compatibility -- --nocapture`
- `git diff --check`
- FIFO mid-stream failure probe recorded at `t9-gate-midstream-fifo.txt`
- `awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\\/\\/|#|--)/' src/scan.rs | wc -l`
- `awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\\/\\/|#|--)/' src/util.rs | wc -l`
- `find ${TMPDIR:-/tmp} -maxdepth 1 -name 'frametrace-t9-*' -print`
- `pgrep -fl 'target/debug/frametrace'`

## verificationResults
- Fresh `cargo test --locked compatibility -- --nocapture`: PASS, 3 compatibility tests passed.
- Fresh `git diff --check`: PASS, no output.
- Worker `cargo test --locked`: PASS transcript, 151 lib tests plus integration/doc tests passed; I did not rerun the full suite because the user prioritized bounded verification, the worker transcript is current in this evidence directory, and fresh focused tests plus direct source/manual probes covered T9-specific behavior.
- Worker `cargo clippy --locked --all-targets --all-features -- -D warnings`: PASS transcript.
- Worker `cargo fmt --all -- --check`: PASS transcript.
- Worker `cargo build --locked`: PASS transcript.
- Worker happy manual QA: PASS transcript; 3500 fixture files, 2 scans, SQLite=3500, JSONL=3500, TSV=3500, scan_runs=2.
- Worker failure manual QA: PASS for unwritable/read-failing `db/videos.jsonl` directory; nonzero exit, failed job recorded, scan_runs remained 1, no success prose, no report-ready artifacts.
- Fresh gate FIFO failure probe: PASS; streamed TSV write hit `Broken pipe`, command exited 1, no scan complete prose, job counts were complete=1 and failed=1, scan_runs remained 1, temp root removed.

## directSlopAndProgrammingPass
`omo:remove-ai-slops` direct pass:
- No deletion-only tests found.
- No tests that merely verify a requested removal found.
- No tautological tests found that only assert a value exists.
- Some tests use string containment against JSON text, but they assert the user-visible compatibility contract (`legacy_full_index`, streaming alternatives, duplicate/missing row counts), not private implementation constants alone.
- `FailingWriter` is a narrow fake for writer error propagation; it is not an unnecessary production abstraction.
- Production extraction is limited to `write_case_stream`, `write_jsonl_records`, `write_tsv_records`, and `create_text_writer`, all directly used by the changed output paths.

`omo:programming` direct pass:
- No new `unsafe`.
- No new production `unwrap()`/`expect()` found in T9 production diff; new unwraps are test-only.
- Error propagation uses `io::Result`, `?`, and contextual `map_err`.
- `src/util.rs` is 242 pure LOC, within the warning band but under the 250 defect ceiling.
- `src/scan.rs` is 1061 pure LOC and already explicitly listed for T11 module split. This needs formal review-disposition support, which is absent.

## cleanupAssessment
- Worker cleanup receipt says all recorded T9 temp roots were removed and no `target/debug/frametrace` processes remained.
- Fresh cleanup checks found no `${TMPDIR}/frametrace-t9-*` roots.
- Fresh `pgrep -fl 'target/debug/frametrace'` returned exit 1 with no output.
- Fresh FIFO probe removed its temp root (`cleanup_root_exists_after: false`).

## exactEvidenceGaps
- No T9-specific code-review report exists in the T9 evidence directory.
- No executor review artifact explicitly covers `remove-ai-slops` overfit/slop criteria for the T9 diff.
- No executor review artifact explicitly covers `programming` criteria and the `src/scan.rs` oversized-file disposition for T9.

## AdversarialVerify
```json
{
  "verdict": "needs-fix",
  "confidence": 0.92,
  "classes": {
    "malformed_input": {
      "status": "confirmed",
      "evidence": "Worker directory-path failure and fresh FIFO BrokenPipe failure both failed closed without success prose."
    },
    "dirty_worktree": {
      "status": "confirmed_with_blocking_evidence_gap",
      "evidence": "Worktree is broadly dirty from the plan; T9 product diff inspected as src/scan.rs and src/util.rs. Required T9 code-review artifact is absent."
    },
    "stale_state": {
      "status": "confirmed",
      "evidence": "Worker happy QA used fresh temp root and repeated scan; counts matched SQLite/JSONL/TSV and scan_runs=2. Fresh FIFO probe used a separate temp root."
    },
    "misleading_success_output": {
      "status": "confirmed",
      "evidence": "Verification used exit codes, parsed counts, SQLite state, and absence of scan-complete prose on failures."
    },
    "flaky_tests": {
      "status": "confirmed",
      "evidence": "Focused compatibility tests passed fresh; worker full test transcript passed."
    },
    "hung_or_long_commands": {
      "status": "confirmed",
      "evidence": "Fresh focused test completed quickly; FIFO probe used subprocess timeouts and cleaned up."
    },
    "prompt_injection": {
      "status": "ruled_out",
      "evidence": "T9 path consumes filesystem records/CLI args, not untrusted natural-language instructions."
    },
    "cancel_resume": {
      "status": "ruled_out",
      "evidence": "T9 does not introduce resumable job semantics; failure path records ordinary failed jobs."
    },
    "repeated_interruptions": {
      "status": "ruled_out",
      "evidence": "T9 does not change interrupt handling."
    }
  }
}
```

## finalVerdict
REJECT. Product behavior is strongly supported, but final-gate approval is blocked by missing required T9 code-review/slop coverage.
