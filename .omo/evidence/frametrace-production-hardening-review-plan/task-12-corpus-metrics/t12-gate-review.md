# T12 Gate Review

verdict: needs-fix
recommendation: REJECT
confidence: 0.91

## Original Intent

T12 was intended to create real non-client validation corpus manifests plus accuracy and reproducibility metrics. The plan requires source hashes, ground truth, expected outputs, explicit precision/recall/false-positive/false-negative/hash-mismatch metrics, reproducibility threshold metrics, and fail-closed handling for malformed or bad manifests. It also explicitly says a synthetic-only corpus cannot satisfy the `mixed_real_world_like` release key.

## Desired Outcome

The user-visible outcome should be a gateable T12 corpus package: committed lightweight non-client fixtures and manifests that satisfy the plan's corpus fixture contract, CLI QA commands that produce parsed machine-readable evidence, and no unresolved programming/remove-ai-slops blockers in the T12 diff.

## User Outcome Review

The CLI behavior mostly works for the narrow synthetic fixture: fresh `qa accuracy` reports precision `1.0`, recall `1.0`, false positives `0`, false negatives `0`, and hash mismatches `0`; fresh `qa reproducibility` reports `normalized_core_differences: 0` with threshold `0`; a copied bad-hash manifest exits nonzero with `false_negatives: 1` and `hash_mismatch: 1`; and a missing `source_sha256` manifest fails closed.

However, I cannot confirm T12 because the shipped manifest/docs/tests do not satisfy the plan's stated ground-truth schema contract, the T12 implementation leaves a direct programming/remove-ai-slops size defect in `src/qa_accuracy.rs`, and the committed corpus paths for carved/recovered media are metadata-only placeholders rather than real fixture files or clearly modeled hash-only external references.

## Checked Artifact Paths

- `.omo/plans/frametrace-production-hardening-review-plan.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/doneclaim.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/adversarial-class-results.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/programming-remove-ai-slops-review.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/cleanup-receipt.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/manual-qa/*.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/manual-qa-*.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/final-cargo-*.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/git-diff-check.txt`
- `corpus/manifest/synthetic-video-corpus.json`
- `docs/validation-corpus.md`
- `docs/recovery-prd.md`
- `docs/recovery-test-spec.md`
- `src/qa_accuracy.rs`
- `src/qa_repro.rs`
- `src/cli/commands.rs`
- `src/qa_tests/accuracy.rs`
- `src/qa_tests/reproducibility_performance.rs`
- `tests/fixtures/corpus/synthetic-video-case-a/**`
- `tests/fixtures/corpus/synthetic-video-case-b/**`

## Plan Check

- T12 checkbox is still unchecked in `.omo/plans/frametrace-production-hardening-review-plan.md`.
- T12 acceptance criteria inspected at plan lines 183-188.
- The broader plan corpus fixture contract at line 58 requires ground-truth schema fields: `corpus_id`, `source_artifact_id`, `source_sha256`, `expected_artifact_type`, `expected_path_pattern`, `expected_hash`, `expected_timestamp_range`, `expected_state`, `negative_controls`, and `notes`.

## Blockers

1. Manifest schema does not satisfy the plan contract.
   - Structured check against `corpus/manifest/synthetic-video-corpus.json` showed supported domains only declare `source_path`, `source_sha256`, `expected_status`, and for deleted recovery `inode`.
   - Missing required contract fields for both supported domains: `corpus_id`, `source_artifact_id`, `expected_artifact_type`, `expected_path_pattern`, `expected_hash`, `expected_timestamp_range`, `expected_state`, `negative_controls`, and `notes`.
   - The docs and tests also encode the narrower schema, so the test suite would not catch this plan mismatch.

2. `src/qa_accuracy.rs` violates the required programming/remove-ai-slops size gate.
   - Fresh pure LOC count: `540`.
   - T12 added roughly `360` lines to this file.
   - `omo:programming` and `omo:remove-ai-slops` both treat files over 250 pure LOC as a defect unless a narrow documented exception exists. No `SIZE_OK` rationale exists in the file.
   - The worker's own `programming-remove-ai-slops-review.md` acknowledges the oversized file and defers the split. Direct gate review cannot approve with unresolved slop.

3. Corpus source paths are not backed by real committed fixture files.
   - `SYN-VID-001` source path is `/non-client/synthetic/video-a.mp4`, outside the repo.
   - `SYN-VID-002` source path `tests/fixtures/corpus/synthetic-video-case-a/artifacts/carved/carve_000001.mp4` does not exist.
   - `SYN-DEL-001` source path `tests/fixtures/corpus/synthetic-video-case-a/artifacts/recovered/inode_42.mp4` does not exist.
   - Their hashes are placeholder values present in synthetic JSONL records. That is enough for a metadata-only smoke fixture, but not enough to confirm the requested real validation corpus/source-hash readiness unless these entries are modeled as external hash-only references with provenance.

4. Anti-slop direct pass found implementation/test overfit risk.
   - `src/qa_tests/accuracy.rs` asserts the same narrowed manifest schema implemented in `src/qa_accuracy.rs`, not the plan line 58 contract.
   - Production code still uses ad hoc JSONL field extraction via `extract_json_string` for case evidence records rather than parsing evidence JSONL into typed records at the boundary. This is not the primary rejection reason, but it increases maintenance risk in the already oversized module.

## Evidence Gaps

- No evidence file proves the manifest satisfies the plan line 58 ground-truth schema fields.
- No evidence file proves the relative media artifact `source_path` entries exist as fixture files or are governed as external hash-only references.
- No accepted cleanup/slop plan resolves the oversized `src/qa_accuracy.rs` file within T12.
- No test would fail if the plan-required fields `source_artifact_id`, `expected_hash`, `negative_controls`, or `notes` remain absent.

## Fresh Commands Run

- `git diff --check`: PASS.
- `cargo test --locked accuracy_report_accepts_typed_corpus_manifest_and_records_metric_shape`: PASS.
- `cargo test --locked accuracy_report_rejects_synthetic_only_mixed_real_world_release_key`: PASS.
- `cargo test --locked reproducibility_report_records_diff_threshold_metrics`: PASS.
- `cargo build --locked`: PASS.
- `target/debug/frametrace qa accuracy tests/fixtures/corpus/synthetic-video-case-a corpus/manifest/synthetic-video-corpus.json --output-dir /tmp/.../accuracy`: PASS; parsed JSON showed `precision: 1.0`, `recall: 1.0`, `false_positives: 0`, `false_negatives: 0`, `hash_mismatch: 0`, `mixed_real_world_like: unsupported`.
- `target/debug/frametrace qa reproducibility tests/fixtures/corpus/synthetic-video-case-a tests/fixtures/corpus/synthetic-video-case-b --output-dir /tmp/.../repro`: PASS; parsed JSON showed threshold `0` and `normalized_core_differences: 0`.
- `target/debug/frametrace qa accuracy tests/fixtures/corpus/synthetic-video-case-a .omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/bad-manifest-case/bad-hash-manifest.json --output-dir /tmp/.../bad`: expected FAIL; exit `1`, parsed JSON showed `false_negatives: 1`, `hash_mismatch: 1`.
- `target/debug/frametrace qa accuracy tests/fixtures/corpus/synthetic-video-case-a .omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/bad-manifest-case/missing-field-manifest.json --output-dir /tmp/.../missing`: expected FAIL; exit `1`, error reported missing `source_sha256`.
- `jq` structured manifest contract check: FAIL against plan-required fields, as listed above.
- `awk` pure LOC count for `src/qa_accuracy.rs`: `540`.
- `find tests/fixtures/corpus/... -path '*/reports/qa*' -print`: no fixture QA report leftovers.
- `git diff --cached --name-only`: no staged files.
- `pgrep -lf 'target/debug/frametrace|frametrace qa|cargo test|cargo clippy|cargo build'`: no leftover T12 processes.

## Adversarial Classes

- `malformed_input`: partial pass. Bad-hash and missing-field manifests fail closed, but plan-schema mismatch is not tested and evidence JSONL parsing remains ad hoc.
- `dirty_worktree`: pass with caveat. Broad dirty worktree exists from active plan work; no staged files found.
- `stale_state`: partial pass. Fresh CLI outputs were generated in `/tmp` and parsed; however doneclaim evidence cannot prove plan-schema compliance.
- `misleading_success_output`: fail. CLI success prose is not the issue; the worker's PASS prose overstates completion despite schema and size blockers.
- `flaky_tests`: pass. Targeted T12 tests passed fresh.
- `hung_or_long_commands`: pass. Fresh commands completed quickly; no leftover matching processes.
- `prompt_injection`: not applicable for natural-language instruction execution; manifests are parsed data.
- `cancel_resume`: not applicable; no resumable flow changed.
- `repeated_interruptions`: not applicable; no interrupt-handling path changed.

## Cleanup Assessment

- No staged files.
- No fixture `reports/qa` directories remain under `tests/fixtures/corpus/synthetic-video-case-a` or `tests/fixtures/corpus/synthetic-video-case-b`.
- No matching `frametrace` or cargo verification processes remain.
- I did not edit product files, plan checkboxes, `.omo/boulder.json`, or `.omo/start-work/ledger.jsonl`.
- This gate artifact is the only remaining file from my writes; an initial misplaced copy in the session's original cwd was removed.

## Required Fixes

1. Update the manifest schema, docs, parser, tests, and fixtures to satisfy the plan line 58 ground-truth contract, or update the plan through the proper planning process before claiming T12.
2. Resolve `src/qa_accuracy.rs` size/slop before gate approval, preferably by splitting typed manifest parsing, indexed evidence parsing, metric computation, and report rendering into focused modules under 250 pure LOC each.
3. Either add real lightweight non-client fixture files with verifiable hashes or move missing media paths to explicit hash-only external references with provenance, then make the manifest semantics and tests reflect that distinction.
4. Add tests that parse JSON and fail if required corpus contract fields are missing.
