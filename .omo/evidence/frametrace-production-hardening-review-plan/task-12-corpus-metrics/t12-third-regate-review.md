# T12 Third Re-Gate Review

verdict: confirmed
recommendation: APPROVE
confidence: 0.91

## originalIntent

T12 is intended to make FrameTrace's validation corpus and QA metrics gateable: committed non-client fixtures, typed corpus manifests with source hashes and ground truth, parsed accuracy/reproducibility metrics, fail-closed malformed-manifest behavior, and no fake real-world corpus claim.

## desiredOutcome

T12 can be marked complete when the T12 plan checkbox remains unchecked during review, the plan line 58 corpus schema is represented in current manifests/docs/tests, `mixed_real_world_like` remains unsupported for synthetic-only evidence, current fixture hashes are real and committed, accuracy/reproducibility reports parse structurally, malformed/bad-hash cases fail closed, and the programming/remove-ai-slops evidence supports the current diff without oversized modules or overfit tests.

## userOutcomeReview

Confirmed. The current product files satisfy the requested user-visible outcome:

- `.omo/plans/frametrace-production-hardening-review-plan.md:183` still has `- [ ] T12. Create real validation corpus manifests and accuracy/reproducibility metrics`.
- `.omo/plans/frametrace-production-hardening-review-plan.md:58` still lists the required ground-truth schema fields.
- `corpus/manifest/synthetic-video-corpus.json` has `external_references: []`, `release_keys.mixed_real_world_like: "unsupported"`, supported-domain `ground_truth_schema` arrays with every plan-required field, and every case `ground_truth` object has those fields.
- No all-`f` placeholder external hash remains in the current manifest/docs/source/test/fixture scope checked by `rg`.
- `docs/validation-corpus.md` states that no hash-only external corpus is claimed until a real external corpus hash and provenance record exist.
- The three current manifest case files exist in `tests/fixtures/corpus/`, and their disk SHA-256 values match both `source_sha256` and `ground_truth.expected_hash`.
- Fresh accuracy CLI output parsed as `passed=true`, `precision=1.0`, `recall=1.0`, `false_positives=0`, `false_negatives=0`, `hash_mismatch=0`, `expected_count=3`, `mixed_real_world_like="unsupported"`, and `external_references=[]`.
- Fresh reproducibility CLI output parsed as `passed=true` with `allowed_diff_thresholds.normalized_core_differences=0` and observed `diff_metrics.normalized_core_differences=0`.
- Fresh bad-hash CLI output exited nonzero and parsed as `passed=false`, `false_negatives=1`, and `hash_mismatch=1`.
- Fresh missing-field CLI output exited nonzero with a typed manifest error naming the accepted ground-truth fields.

## checkedArtifactPaths

- `.omo/plans/frametrace-production-hardening-review-plan.md`
- `.omo/start-work/ledger.jsonl`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/t12-gate-review.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/t12-regate-review.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/t12-second-fix-doneclaim.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/t12-third-fix-doneclaim.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/programming-remove-ai-slops-review.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/t12-fix-cargo-test-locked.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/t12-fix-cargo-clippy.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/t12-second-fix-structural-check.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/t12-third-fix-pure-loc.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/t12-third-fix-focused-tests.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/t12-third-fix-cleanup-receipt.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/notepad.md`
- `corpus/manifest/synthetic-video-corpus.json`
- `docs/validation-corpus.md`
- `docs/recovery-test-spec.md`
- `src/qa_accuracy.rs`
- `src/qa_accuracy/contract.rs`
- `src/qa_accuracy/indexed.rs`
- `src/qa_accuracy/manifest.rs`
- `src/qa_accuracy/metrics.rs`
- `src/qa_accuracy/report.rs`
- `src/qa_accuracy/schema.rs`
- `src/qa_accuracy/types.rs`
- `src/qa_tests/accuracy.rs`
- `tests/fixtures/corpus/**`

## freshVerification

- `git diff --check`: PASS.
- `cargo fmt --all -- --check`: PASS.
- `cargo test --locked qa_tests::accuracy -- --nocapture`: PASS, 5 tests.
- Structural `jq` check for `external_references == []`, `mixed_real_world_like == "unsupported"`, supported-domain schema fields, and case `ground_truth` keys: PASS.
- `rg` placeholder/schema scan over current manifest/docs/source/test/fixture scope: no all-`f` placeholder hash; current docs/source mention only valid unsupported/external-reference semantics.
- Fixture SHA loop: PASS for `SYN-VID-001`, `SYN-VID-002`, and `SYN-DEL-001`.
- Pure LOC check: `src/qa_tests/accuracy.rs` 244; `src/qa_accuracy.rs` 75; `contract.rs` 108; `indexed.rs` 96; `manifest.rs` 166; `metrics.rs` 56; `report.rs` 86; `schema.rs` 75; `types.rs` 51.
- Fresh CLI accuracy happy path: PASS with parsed metrics listed above.
- Fresh CLI reproducibility happy path: PASS with parsed threshold/diff values listed above.
- Fresh CLI bad-hash path: expected FAIL, exit 1 with parsed mismatch metrics.
- Fresh CLI missing-field path: expected FAIL, exit 1 with typed manifest error.
- Existing T12 fix full test transcript `t12-fix-cargo-test-locked.txt`: inspected; ends with `EXIT_CODE=0`.
- Existing T12 fix clippy transcript `t12-fix-cargo-clippy.txt`: inspected; `cargo clippy --locked --all-targets --all-features -- -D warnings` ends with `EXIT_CODE=0`.
- Cleanup checks: no staged files; no `tests/fixtures/corpus/**/reports/qa*`; no matching cargo/frametrace QA processes; no tracked diff under `.omo/boulder.json`, `.omo/start-work/ledger.jsonl`, or `.omo/plans`.

## directProgrammingAndSlopPass

Loaded and applied `omo:programming` plus Rust reference and `omo:remove-ai-slops`.

- Oversized module blocker is closed. The changed test file is 244 pure LOC, and every current `src/qa_accuracy` production module is below 250 pure LOC.
- The current `programming-remove-ai-slops-review.md` explicitly covers `src/qa_tests/accuracy.rs`, the split production module LOC, and overfit/slop checks.
- Direct review found no deletion-only tests, no tests that merely verify a requested removal, no tautological success-prose assertions, and no implementation-mirroring snapshot tests for the T12 requirements.
- Tests mutate parsed JSON and remove required fields; malformed schema coverage is not substring-only.
- Production parsing uses serde structs at the manifest and indexed-evidence boundaries; the earlier ad hoc JSON substring parser is gone.
- No new dependency, broad abstraction, public API drift, or unresolved SIZE_OK escape was found in the T12 scope.

## blockers

[]

## adversarialClasses

- malformed_input: confirmed. Bad-hash and missing-field paths fail closed; typed schema and contract checks cover the plan fields.
- dirty_worktree: confirmed with scope caveat. The active plan worktree is broadly dirty/untracked, but no staged files exist and this gate did not edit product files, plan state, `.omo/boulder.json`, or `.omo/start-work/ledger.jsonl`.
- stale_state: confirmed with caveats. Current files, fresh CLI runs, fresh structural checks, and current slop review prove closure. Caveat: `t12-second-regate-review.md` is missing from disk although the ledger records the second-regate rejection; older superseded manual QA JSON under `manual-qa/t12-fix-happy-accuracy-report.json` still contains the pre-fix placeholder external reference and was not used as current proof.
- misleading_success_output: confirmed. Verdict relies on parsed JSON, exit codes, source inspection, LOC counts, fixture hashes, and transcripts, not prose claims.
- flaky_tests: confirmed. Focused `qa_tests::accuracy` passed fresh and in the third-fix transcript.
- hung_or_long_commands: confirmed. Bounded commands completed; no matching lingering cargo/frametrace QA processes remain.
- prompt_injection: not applicable. Corpus manifests are parsed as data and are not used as natural-language instructions.
- cancel_resume: not applicable. No cancel/resume behavior changed.
- repeated_interruptions: not applicable. No interrupt-handling path changed.

## cleanup

Clean for gate purposes:

- No staged files.
- No fixture `reports/qa` leftovers.
- No matching cargo/frametrace QA processes.
- No tracked diff under `.omo/boulder.json`, `.omo/start-work/ledger.jsonl`, or `.omo/plans`.
- This report is the only file intentionally written by the third re-gate reviewer.

## exactEvidenceGaps

- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/t12-second-regate-review.md` is missing from disk. The rejection is preserved in `.omo/start-work/ledger.jsonl`, and the current gate independently rechecked the two recorded blockers.
- Older superseded manual QA artifact `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/manual-qa/t12-fix-happy-accuracy-report.json` still contains the pre-second-fix placeholder external reference. It is stale relative to current product files and fresh CLI output; it should not be used as current T12 proof.
- No real external/mixed-real-world corpus is claimed. This is acceptable for T12 because the current synthetic manifest explicitly has `external_references: []` and `mixed_real_world_like: "unsupported"`.

## residualRisks

- T12 validates the committed synthetic/non-client corpus and metrics; it does not satisfy the future mixed-real-world release key.
- The workspace has many unrelated active-plan dirty/untracked files. This gate scoped review to T12 current files and evidence and did not attempt to classify all non-T12 work.

## finalRecommendation

confirmed
