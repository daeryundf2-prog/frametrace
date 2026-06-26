# T10 Gate Review

recommendation: REJECT

## originalIntent

Independently gate-review T10 of the FrameTrace production hardening plan from the user's perspective, staying read-only except for this artifact. T10 is intended to prove large-case survival with a 100k local pass and a 1M profile that records pass/fail metrics for memory, throughput, and query latency without silently converting a 1M failure into success.

## desiredOutcome

The user-visible outcome should be a defensible decision on whether T10 can close as evidence-complete. Completion requires:

- Plan T10 and verification strategy respected.
- 100k profile passes local verification.
- 1M profile exists with visible pass/fail metrics and release-blocker semantics when thresholds are exceeded.
- Source and tests do not create false confidence, scope drift, or unreviewed slop.
- Evidence artifacts support the worker's claims without relying on untrusted prose.

## userOutcomeReview

The 100k evidence is strong enough on the measured run: `performance-report-100k.json` parses, has `passed: true`, records 100000 rows, has bounded inventory timings, writes 100000 JSONL and TSV compatibility rows, denies full JSON load, and stays far below the plan's 1.5 GiB RSS ceiling by measured RSS.

The 1M evidence preserves fail-closed semantics: `performance-report-1m.json` parses, has `passed: false`, records 1000000 rows, writes 1000000 JSONL and TSV compatibility rows, denies full JSON load, and records the query-latency failure `large_case_survival.max_query_ms=2142` against `query_latency_target_ms=2000`. This was not silently converted to a pass.

However, I cannot approve T10 because the artifact set is missing the required independent code-review/slop-coverage report, and the production performance QA reports memory targets that are looser than the plan's stated thresholds. Those gaps can create false release confidence even though the measured RSS happens to pass the stricter limits.

## blockers

1. Missing code-review / anti-slop coverage artifact.

   Exact failure: no T10 evidence file contains a code-review report showing `omo:programming` and `omo:remove-ai-slops` perspective coverage, overfit/slop test review, or support for the oversized-module deferral. The only hit for skill/review language is the worker notepad saying review-work was considered but unavailable.

   Reproduction:

   ```bash
   find .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance -maxdepth 1 -type f -iname '*review*' -o -iname '*slop*' -o -iname '*code*'
   rg -n -i "slop|overfit|programming|skill|code review|review-work|remove-ai|tautolog|mirror|coverage" .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance
   ```

   Observed: `find` returned no review/slop/code-review artifact. `rg` only found `notepad.md:3`.

   Minimal fix guidance: add a T10 review artifact under this evidence directory that explicitly covers the programming and remove-ai-slops criteria, including overfit/slop tests, oversized module handling, threshold semantics, and whether any slop is intentionally deferred to T11. The report must be evidence-backed, not just a doneclaim statement.

2. Performance QA encodes memory gates looser than the plan.

   Exact failure: the plan's verification strategy says 100k max RSS must be <= 1.5 GiB and the 1M release-candidate profile must be <= 3.5 GiB. Current production code uses 2.5 GiB for 100k and 4 GiB for 1M:

   - [src/performance_qa.rs](/Users/shinyoohag/Desktop/frametrace/src/performance_qa.rs:491)
   - `performance-report-100k.json.max_rss_target_bytes = 2684354560`
   - `performance-report-1m.json.max_rss_target_bytes = 4294967296`

   Reproduction:

   ```bash
   jq '{rows, passed, max_rss_bytes, max_rss_target_bytes}' \
     .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/performance-report-100k.json \
     .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/performance-report-1m.json
   ```

   Expected targets from the plan are `1610612736` bytes for 1.5 GiB at 100k and `3758096384` bytes for 3.5 GiB at 1M. The measured RSS values pass those stricter limits, but the shipped gate would allow future regressions between the plan limit and the looser code limit.

   Minimal fix guidance: update `max_rss_target_for_rows` to encode the plan thresholds, add parsed JSON assertions for those target bytes, rerun the 100k profile, and update the 1M evidence or add a corrected threshold receipt that preserves the query-latency release blocker.

3. Test adequacy and slop-review gap.

   Exact failure: the new T10 tests at [src/performance_qa.rs](/Users/shinyoohag/Desktop/frametrace/src/performance_qa.rs:786) mostly assert JSON substrings and file existence. They do not assert the plan memory thresholds, parsed `large_case_survival` values, fail-closed status, or that full JSON denial is represented structurally rather than by incidental text. This is not enough to offset the missing slop/code-review artifact.

   Minimal fix guidance: add narrow parsed JSON assertions for target bytes, survival timings, export row counts, and `full_json_load_allowed == false`. Avoid tests that only mirror field names.

## releaseBlockerAssessment

The 1M failure is handled correctly as a release blocker, not as a pass. Evidence:

- `doneclaim.json.status` is `DONE_WITH_RELEASE_BLOCKER`.
- `doneclaim.json.checkbox_claimed` is `false`.
- `manual-qa-1m-transcript.txt` exits `EXIT_CODE=1`.
- `performance-report-1m.json.passed` is `false`.
- Failure reason is query latency: `large_case_survival.max_query_ms=2142` exceeds `query_latency_target_ms=2000`.

In principle, T10 can close as evidence-complete with a fail-closed 1M release blocker because the plan only requires the 1M profile to exist with pass/fail metrics before release candidate. This particular submission still needs fixes for the evidence/report gaps above before I would approve closure.

## cleanupAssessment

Cleanup evidence is mostly acceptable:

- `cleanup-receipt.txt` preserves `qa-performance-100k` and `qa-performance-1m`.
- `cleanup-receipt.txt` removes `qa-performance-invalid`.
- Process scan in the receipt found no matching `frametrace qa performance` processes.
- Fresh SQLite checks confirmed exact row counts:
  - 100k DB: `100000|bench_00000000|bench_00099999`
  - 1M DB: `1000000|bench_00000000|bench_00999999`

Evidence gap: `manual-qa-100k-transcript.txt` and `manual-qa-1m-transcript.txt` do not show the claimed pre-run directory deletion. The stale-state risk is mitigated by exact DB counts and matching report hashes, but the deletion claim itself is not independently proven by the transcripts.

## checkedArtifactPaths

- `.omo/plans/frametrace-production-hardening-review-plan.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/doneclaim.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/performance-report-100k.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/performance-report-1m.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/qa-performance-100k/performance-report.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/qa-performance-1m/performance-report.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/manual-qa-100k-transcript.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/manual-qa-1m-transcript.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/manual-qa-full-json-load-denial.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/manual-qa-malformed-input-row-count.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/cargo-test-locked.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/cargo-clippy-locked-all-targets-all-features.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/cargo-fmt-check.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/git-diff-check.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/git-status-short.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/cleanup-receipt.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/notepad.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/performance_qa-pure-loc.txt`
- `src/performance_qa.rs`
- `src/case_db/metrics.rs`

## commandsRun

```bash
sed -n '45,60p;167,173p;232,258p' .omo/plans/frametrace-production-hardening-review-plan.md
jq . .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/doneclaim.json
jq '{passed, rows, elapsed_ms, rows_per_minute, rows_per_minute_target, max_query_ms, query_latency_target_ms, max_rss_bytes, max_rss_target_bytes, large_case_survival}' .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/performance-report-100k.json
jq '{passed, rows, elapsed_ms, rows_per_minute, rows_per_minute_target, max_query_ms, query_latency_target_ms, max_rss_bytes, max_rss_target_bytes, large_case_survival}' .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/performance-report-1m.json
tail -n 80 .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/manual-qa-100k-transcript.txt
tail -n 120 .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/manual-qa-1m-transcript.txt
cat .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/manual-qa-full-json-load-denial.txt
cat .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/manual-qa-malformed-input-row-count.txt
cat .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/cleanup-receipt.txt
cat .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/performance_qa-pure-loc.txt
tail -n 80 .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/cargo-test-locked.txt
tail -n 80 .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/cargo-clippy-locked-all-targets-all-features.txt
cat .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/cargo-fmt-check.txt
cat .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/git-diff-check.txt
cat .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/git-status-short.txt
git diff --check
jq -e '.passed == true and .rows == 100000 and (.max_rss_bytes <= 1610612736) and (.large_case_survival.max_query_ms <= .large_case_survival.query_latency_target_ms) and (.large_case_survival.compatibility_exports.jsonl_rows == 100000) and (.large_case_survival.compatibility_exports.tsv_rows == 100000) and (.large_case_survival.full_json_load_denial.full_json_load_allowed == false)' .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/performance-report-100k.json
jq -e '.passed == false and .rows == 1000000 and (.max_rss_bytes <= 3758096384) and (.large_case_survival.max_query_ms > .large_case_survival.query_latency_target_ms) and (.large_case_survival.compatibility_exports.jsonl_rows == 1000000) and (.large_case_survival.compatibility_exports.tsv_rows == 1000000) and (.large_case_survival.full_json_load_denial.full_json_load_allowed == false)' .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/performance-report-1m.json
shasum -a 256 .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/performance-report-100k.json .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/qa-performance-100k/performance-report.json .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/performance-report-1m.json .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/qa-performance-1m/performance-report.json
rg -n "max_rss_target_for_rows|GIB|PERFORMANCE_QUERY|full_json_load|source_profile_json|contains|performance_report_records|performance_report_preserves|unwrap\(|expect\(|unsafe|#\[allow" src/performance_qa.rs
rg -n -i "slop|overfit|programming|skill|code review|review-work|remove-ai|tautolog|mirror|coverage" .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance
nl -ba src/performance_qa.rs | sed -n '1,120p;390,500p;760,835p'
jq '{rows, passed, max_rss_bytes, max_rss_target_bytes, large_case_survival: {max_query_ms: .large_case_survival.max_query_ms, query_latency_target_ms: .large_case_survival.query_latency_target_ms, jsonl_rows: .large_case_survival.compatibility_exports.jsonl_rows, tsv_rows: .large_case_survival.compatibility_exports.tsv_rows, jsonl_rows_per_minute: .large_case_survival.compatibility_exports.jsonl_rows_per_minute, tsv_rows_per_minute: .large_case_survival.compatibility_exports.tsv_rows_per_minute, full_json_load_allowed: .large_case_survival.full_json_load_denial.full_json_load_allowed}}' .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/performance-report-100k.json .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/performance-report-1m.json
wc -c .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/performance-report-100k.json .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/performance-report-1m.json .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/manual-qa-full-json-load-denial.txt
rg -n "fn benchmark_case_db|benchmark_case_db|bench_" src/case_db -S
nl -ba src/case_db/metrics.rs | sed -n '1,140p'
sqlite3 .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/qa-performance-100k/db/case.db 'select count(*), min(id), max(id) from videos;'
sqlite3 .omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/qa-performance-1m/db/case.db 'select count(*), min(id), max(id) from videos;'
```

## directAntiSlopAndProgrammingPass

- Loaded and applied `omo:programming` and `omo:remove-ai-slops` criteria.
- `src/performance_qa.rs` is 774 pure LOC after T10, confirmed by `performance_qa-pure-loc.txt`. The plan says T11 owns module decomposition, so I did not treat this as a standalone functional failure for T10, but it must have explicit review support before approval. That support is currently absent.
- Production code contains ad hoc string checks for JSON evidence in [src/performance_qa.rs](/Users/shinyoohag/Desktop/frametrace/src/performance_qa.rs:468), and substring-based parser detection in [src/performance_qa.rs](/Users/shinyoohag/Desktop/frametrace/src/performance_qa.rs:416). These are not the main blockers, but they should be covered by the missing code-review/slop report or replaced with structured parsing.
- New tests are useful smoke coverage but are too field-name oriented to prove the plan threshold contract.

## exactEvidenceGaps

- No code-review/slop artifact exists under the T10 evidence directory.
- No artifact explicitly maps the plan memory thresholds to the implementation's `max_rss_target_bytes`.
- Manual transcripts do not prove the claimed pre-run deletion of `qa-performance-100k` and `qa-performance-1m`.
- No parsed-test evidence asserts the corrected 1.5 GiB / 3.5 GiB target bytes.

## adversarialVerify

```json
{
  "verdict": "needs-fix",
  "confidence": 0.88,
  "classes": {
    "malformed_input": {
      "result": "confirmed_fail_closed",
      "evidence": "manual-qa-malformed-input-row-count.txt exits 1 for --rows 0 with benchmark row count must be greater than 0"
    },
    "dirty_worktree": {
      "result": "risk_recorded",
      "evidence": "git-status-short.txt and fresh git status show a broad dirty worktree beyond T10; T10 code claim is src/performance_qa.rs plus evidence, but isolation is not clean"
    },
    "stale_state": {
      "result": "partially_ruled_out",
      "evidence": "report hashes match run-local reports and SQLite row counts are exact for 100k and 1M; transcripts do not show the claimed pre-run directory deletion"
    },
    "misleading_success_output": {
      "result": "ruled_out_for_1m",
      "evidence": "1M report passed=false, transcript EXIT_CODE=1, doneclaim status DONE_WITH_RELEASE_BLOCKER"
    },
    "flaky_tests": {
      "result": "not_fully_ruled_out",
      "evidence": "full cargo test transcript exits 0, but no repeat-run evidence and tests are weak on threshold semantics"
    },
    "hung_or_long_commands": {
      "result": "ruled_out_currently",
      "evidence": "1M transcript completed in 92.43s, cleanup process scan found no remaining matching processes, fresh sqlite3 count completed"
    },
    "prompt_injection": {
      "result": "low_risk",
      "evidence": "T10 performance QA uses synthetic benchmark data; no untrusted natural-language instructions are executed"
    },
    "cancel_resume": {
      "result": "low_risk",
      "evidence": "No resumable workflow state was edited during this gate; doneclaim keeps checkbox_claimed=false"
    },
    "repeated_interruptions": {
      "result": "low_risk",
      "evidence": "No active profile processes remain; 1M failure is represented by completed artifacts"
    }
  }
}
```

## finalVerdict

needs-fix

T10 has credible 100k pass evidence and credible 1M fail-closed release-blocker evidence. It should not be approved until the missing code-review/slop artifact exists and the performance QA memory target contract matches the plan.
