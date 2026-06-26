# T11 Module Size Report

## Scope
Measured pure LOC for the T11 scoped files plus `src/performance_qa.rs`/new submodules using:

```bash
awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\/\/|#|--)/' <file> | wc -l
```

Baseline and post reports are saved as `baseline-module-loc.txt` and `post-module-loc.txt`.

## Splits Completed

- `src/performance_qa.rs` (934 pure LOC at baseline) was split into `src/performance_qa/mod.rs`, `compatibility.rs`, `survival.rs`, `render.rs`, and `tests.rs`.
- `src/qa_tests.rs` (538 pure LOC at baseline) was split into `src/qa_tests/mod.rs`, `accuracy.rs`, `helpers.rs`, `release_privacy.rs`, `report_defense.rs`, `report_defense_audit.rs`, and `reproducibility_performance.rs`.

All new split files are <=250 pure LOC. Largest new files: `src/performance_qa/render.rs` at 230, `src/performance_qa/tests.rs` at 208, and `src/performance_qa/compatibility.rs` at 191.

## Remaining Oversized Files

These files remain over 250 pure LOC and are explicitly deferred rather than hidden:

- `src/scan.rs` 1061 LOC — `SIZE_OK deferred for T11`: scan indexing, compatibility-output merging, and hand-written legacy JSON extraction are tightly covered by compatibility tests. Splitting this safely requires a dedicated scan-index/output/parser refactor because the file owns public compatibility contracts and JSON/TSV drift risk.
- `src/html_report.rs` 901 LOC — `SIZE_OK deferred for T11`: single large HTML/JS string renderer. A safe split needs snapshot/DOM contract pinning around generated report/viewer payloads; this T11 avoided editing renderer text to prevent output drift.
- `src/cli/handlers.rs` 838 LOC — `SIZE_OK deferred for T11`: public command handler surface with many pre-existing T1-T10 changes. Existing `cli/*_cmd.rs` modules already carry some routing split; deeper extraction risks command-output drift and shared dirty conflicts.
- `src/qa_report_defense.rs` 800 LOC — `SIZE_OK deferred for T11`: report-defense and privacy logic are behavior-sensitive and heavily gate release semantics. Tests were split first; production logic split should follow a dedicated internal API map.
- `src/report.rs` 411 LOC — `SIZE_OK deferred for T11`: single generated report renderer. Same output-drift risk as `html_report.rs`; behavior snapshots stayed clean by not editing renderer content.
- `tests/cli_lifecycle.rs` 309 LOC — `SIZE_OK deferred for T11`: end-to-end lifecycle scenario intentionally stays in one narrative test file to preserve fixture flow.
- `src/qa_release.rs` 287 LOC — `SIZE_OK deferred for T11`: near-threshold release gate orchestration; no safe responsibility split was needed for this T11 after QA tests were separated.
- `tests/cli_windows_prereq.rs` 264 LOC — `SIZE_OK deferred for T11`: near-threshold Windows prereq integration tests; split is low value without changing behavior coverage.

`src/artifacts.rs` is 220 pure LOC and below the ceiling after prior T1-T10 changes.

## Behavior Evidence

- Before and after focused cargo behavior snapshots are saved in `baseline-cargo-test-*.txt` and `post-cargo-test-*.txt`.
- Manual CLI smoke contracts are saved in `baseline-cli-smoke-contract.json` and `post-cli-smoke-contract.json`.
- `behavior-snapshot-diff.txt` records no normalized CLI smoke contract drift.
