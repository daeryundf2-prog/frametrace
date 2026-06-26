# T10 Programming / Remove-AI-Slops Review

Scope reviewed: `src/performance_qa.rs` T10 changes and T10 evidence artifacts under `.omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/`.

## omo:programming Rust Criteria

Evidence reviewed:
- `fix-focused-performance-tests.txt` -> focused Rust tests pass with parsed JSON assertions.
- `fix-cargo-fmt-check.txt` -> rustfmt gate.
- `fix-cargo-clippy-locked-all-targets-all-features.txt` -> clippy `-D warnings` gate.
- `fix-cargo-test-locked.txt` -> full Rust test suite.

Findings:
- Boundary parsing: new tests parse generated reports through `serde_json::Value` instead of substring-only checks for T10 contract fields.
- Error handling: production code returns `Result<_, String>` and propagates I/O/SQLite/report failures; no production `unwrap`/`expect` was introduced.
- Unsafe: no `unsafe` introduced.
- Borrowing/allocation: compatibility exports stream through SQLite rows into `BufWriter`; they no longer collect all inventory rows or page with OFFSET for the export path.
- Exhaustive match: no new owned enum wildcard discrimination introduced.
- Dependencies: no new dependencies added; existing `serde_json` is used in tests.

Conclusion: Rust criteria are satisfied for the scoped T10 fix, subject to the deliberate oversized-file disposition below.

## omo:remove-ai-slops / Overfit / Test-Slop Review

Categories checked:
- Obvious comments/debug leftovers: no debug prints or commented-out code were added.
- Over-defensive code: full-json-load denial is an explicit release contract check, not a speculative guard; malformed row count remains boundary validation.
- Excessive complexity: T10 added multiple helpers in one existing QA module. This is real complexity but scoped to T10 evidence generation and is marked for T11 split rather than widened here.
- Needless abstraction: helper structs (`LargeCaseSurvivalEvidence`, `CompatibilityExportEvidence`, `CompatibilityRow`) model report sections and streaming export rows used by multiple local functions; they are not pass-through wrappers.
- Boundary violations: performance QA calls existing case DB/report/review/workstation APIs; it does not duplicate those subsystems' internals except for a read-only compatibility export cursor used to avoid the previous OFFSET performance bottleneck.
- Dead code: no unused T10 helper remains after focused tests and clippy.
- Duplication: JSON/TSV rendering is local to compatibility exports and mirrors the legacy compatibility shape intentionally. No duplicate collect/temporary-row accumulation remains.
- Performance equivalence: the export path was changed from paged inventory materialization to a direct ordered SQLite cursor. Observable compatibility row counts are locked by tests and 100k evidence.
- Missing tests: added parsed JSON assertions for RSS thresholds, large-case survival latency, compatibility row counts, full-json-load denial, and 1M fail-closed semantics.
- Test slop/overfit: tests assert typed JSON values and gate semantics, not success prose or field-name substrings. The 1M release-blocker test uses a fixture to avoid requiring a million-row unit test while preserving the actual threshold logic.

Conclusion: no T10 slop remains that should be fixed before handing back to root. The only deferred item is module size/splitting, assigned to T11 by the active plan scope.

## Oversized `src/performance_qa.rs` Disposition

Measured artifact: `fix-performance_qa-pure-loc.txt` records 934 pure LOC after parsed-test additions; the file remains over the 250 pure LOC guideline.

Disposition:
- This is acknowledged as an architectural smell.
- T10 was explicitly instructed to avoid T11 refactor/splitting: "Avoid T11 refactor/splitting; T11 owns module decomposition after T10 is green."
- Splitting now would broaden scope across QA/report/export helpers while T1-T9 are dirty and root owns orchestration state.
- Behavior is locked by focused tests plus full suite verification before T11 decomposition.

Deferred to T11:
- Extract performance report schema/rendering.
- Extract large-case survival evidence generation.
- Extract compatibility export evidence writer.
- Keep `qa performance` behavior unchanged during the split.

## Threshold Semantics

Corrected plan targets in `src/performance_qa.rs`:
- 100k max RSS target: `1610612736` bytes (1.5 GiB).
- 1M max RSS target: `3758096384` bytes (3.5 GiB).

Evidence:
- `fix-focused-performance-tests.txt` includes `performance_report_uses_plan_memory_targets_for_100k_and_1m_profiles` and `performance_json_fixture_records_plan_memory_targets_for_1m`.
- `fix-manual-qa-100k-transcript.txt` proves the 100k output directory was deleted before rerun.
- `performance-report-100k.json` now reports `max_rss_target_bytes: 1610612736`.
- `corrected-threshold-receipt-1m.json` records the old 1M report target, corrected 3.5 GiB target, measured RSS, and preserves the query-latency release blocker without faking a pass.

Release-blocker semantics:
- 100k profile passes under corrected RSS target.
- Existing measured 1M profile remains fail-closed because query latency exceeded target (`2142 ms > 2000 ms`), independent of memory target correction.

## Evidence-Backed Conclusion

T10 fix blockers addressed:
1. This review artifact explicitly covers Rust programming and anti-slop criteria.
2. RSS targets are corrected in code and covered by parsed JSON/constant tests.
3. New T10 tests parse JSON and assert values/semantics rather than field-name substrings.
4. Stale-state evidence is closed by corrected 100k transcript with pre-run deletion and corrected 1M threshold receipt.

Result: T10 remains `DONE_WITH_RELEASE_BLOCKER`, not release-green, because 1M query latency is still a measured release blocker. No plan checkboxes or orchestration ledgers were edited.
