# T9 Stream Compat Outputs Notepad

Tier: HEAVY - scan compatibility output generation is a CLI data-output behavior with failure semantics and release compatibility guarantees.

Skills:
- omo:programming - mandatory for Rust edits; loaded Rust reference before code changes.
- TDD discipline - explicitly required by task; using red/green proof before production edits.

Success criteria:
- JSONL and TSV compatibility outputs are written row-by-row without building full output strings in memory.
- Repeated scan produces no duplicate/missing compatibility rows, verified against SQLite counts.
- Compatibility JSON behavior remains explicit and release-safe, with tests/documentation around continued generation.
- Mid-stream/write-path failure fails closed: nonzero exit, no false success/report-ready state.
- Evidence saved under this task directory; no orchestration state or unrelated dirty changes edited.

Manual QA scenarios:
- Happy path command: build CLI, create temp tree with several thousand files, run `target/debug/frametrace scan-folder <case> <source> --no-ffprobe` twice; PASS iff SQLite count == JSONL line count == TSV data-row count and commands exit 0.
- Failure path command: replace `<case>/db/videos.jsonl` with a directory, run scan again; PASS iff command exits nonzero and no report-ready marker/status is produced.

## Final Self-Review
- Single responsibility: T9 changes are limited to scan compatibility output writing and shared writer creation.
- Boundary purity: no new untyped external input boundary added.
- Escape hatches: no new unwrap/expect outside tests; no unsafe.
- Tests: RED/GREEN compatibility tests plus full cargo test and real CLI QA captured.
- File size: src/scan.rs is oversized pre-existing/T11-owned; no broad split attempted under T9 scope.
- Cleanup: final receipt proves temp roots removed and no frametrace process remained.
