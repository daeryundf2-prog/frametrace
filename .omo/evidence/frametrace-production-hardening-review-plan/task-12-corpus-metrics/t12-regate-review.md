# T12 Re-Gate Review

verdict: needs-fix
recommendation: REJECT
confidence: 0.88

## Original Intent

T12 is meant to create real non-client validation corpus manifests and accuracy/reproducibility metrics for FrameTrace release gating. The user-visible outcome is a gateable corpus package: lightweight fixtures under `tests/fixtures/corpus/`, manifests under `corpus/manifest/`, hash-only external references only when they are real and provenance-backed, parsed QA reports for accuracy/reproducibility, and no unresolved programming/remove-ai-slops evidence gaps.

## Desired Outcome

T12 can be marked complete only if the plan checkbox remains unchecked during review, the line 58 corpus contract is preserved, all initial gate blockers are fixed, reports are verified by JSON parsing, cleanup is clean, and the evidence set supports the implementation without stale or unsupported review claims.

## User Outcome Review

The implementation fixes most product-level blockers from the initial gate. The T12 checkbox is still unchecked at `.omo/plans/frametrace-production-hardening-review-plan.md:183`, and line 58 still lists the standard contract fields: `corpus_id`, `source_artifact_id`, `source_sha256`, `expected_artifact_type`, `expected_path_pattern`, `expected_hash`, `expected_timestamp_range`, `expected_state`, `negative_controls`, and `notes`.

Fresh checks confirm the typed manifest includes those fields for supported domains and cases, the three manifest case source paths now exist as lightweight fixture files, fixture SHA-256 values match `source_sha256` and `expected_hash`, targeted tests pass, and CLI happy/failure reports parse structurally with the expected precision/recall/FP/FN/hash-mismatch and reproducibility threshold/diff metrics.

I cannot confirm T12 because two evidence/contract blockers remain:

1. `corpus/manifest/synthetic-video-corpus.json` still contains a hash-only external reference with SHA-256 `ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff`. That is a placeholder-looking value, not a verifiable external corpus hash. The object has only `corpus_id`, `description`, `sha256`, and `hash_only`; it has no enforceable provenance fields such as source artifact ID, storage/location reference, acquisition/generation note, owner, or date, and the parser does not validate placeholders or provenance. This does not satisfy the regate requirement for explicit hash-only external references with provenance.
2. The only programming/remove-ai-slops review artifact I found, `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/programming-remove-ai-slops-review.md`, is stale. It still says `src/qa_accuracy.rs` is above the 250 pure-LOC threshold and defers the split, while the actual code has since been split. The direct pass shows the size issue is fixed, but the required skill-perspective review report does not explicitly cover the fixed diff or the current overfit/slop criteria. Per the final gate rules, unsupported or stale report coverage is a blocker.

## Checked Artifact Paths

- `.omo/plans/frametrace-production-hardening-review-plan.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/t12-gate-review.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/t12-fix-doneclaim.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/adversarial-class-results.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/programming-remove-ai-slops-review.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/t12-fix-*`
- `corpus/manifest/synthetic-video-corpus.json`
- `docs/validation-corpus.md`
- `docs/recovery-prd.md`
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
- `src/qa_tests/reproducibility_performance.rs`
- `src/qa_repro.rs`
- `tests/fixtures/corpus/**`

## Direct Slop And Programming Pass

- Loaded and applied `omo:remove-ai-slops` and `omo:programming` guidance, including the Rust reference.
- Direct pure-LOC check passed: `src/qa_accuracy.rs` 75, `contract.rs` 108, `indexed.rs` 96, `manifest.rs` 166, `metrics.rs` 56, `report.rs` 86, `schema.rs` 75, `types.rs` 51.
- Production code removed the ad hoc JSON substring extractor and now parses JSONL records with typed serde structs.
- The split modules are focused by responsibility; I did not find deletion-only tests, tests that merely verify requested removal, or obvious implementation-mirroring tests for the fixed blockers.
- Remaining issue is evidence/report coverage: the recorded programming/remove-ai-slops review is stale and unsupported for the current split.

## Fresh Commands Run

- `git diff --check` - PASS.
- `jq` structural manifest contract check - PASS for supported-domain `ground_truth_schema`, case `ground_truth` keys, non-empty `negative_controls`, non-empty `notes`, and `mixed_real_world_like: unsupported`.
- `shasum -a 256` fixture check - PASS for `SYN-VID-001`, `SYN-VID-002`, and `SYN-DEL-001`; actual hashes match manifest `source_sha256` and `expected_hash`.
- `cargo test --locked qa_tests::accuracy -- --nocapture` - PASS, 5 tests.
- `cargo test --locked qa_tests::reproducibility_performance -- --nocapture` - PASS, 4 tests.
- Existing transcript inspected: `t12-fix-cargo-test-locked.txt` ends with `EXIT_CODE=0`.
- Existing transcript inspected: `t12-fix-cargo-clippy.txt` exits 0 with `-D warnings`.
- Existing transcript inspected: `t12-fix-cargo-build-locked.txt` exits 0.
- Manual happy CLI: `target/debug/frametrace qa accuracy tests/fixtures/corpus/synthetic-video-case-a corpus/manifest/synthetic-video-corpus.json --output-dir <tmp>` - PASS; parsed JSON showed `precision=1.0`, `recall=1.0`, `false_positives=0`, `false_negatives=0`, `hash_mismatch=0`, `mixed_real_world_like="unsupported"`, `expected_count=3`, and all expected records include the contract keys.
- Manual happy CLI: `target/debug/frametrace qa reproducibility tests/fixtures/corpus/synthetic-video-case-a tests/fixtures/corpus/synthetic-video-case-b --output-dir <tmp>` - PASS; parsed JSON showed `allowed_diff_thresholds.normalized_core_differences=0`, `diff_metrics.normalized_core_differences=0`, and matching normalized byte counts.
- Manual failure CLI: bad-hash manifest exits 1; parsed report showed `passed=false`, `false_negatives=1`, and `hash_mismatch=1`.
- Manual failure CLI: missing `source_artifact_id` manifest exits 1 with a typed missing-field error naming `source_artifact_id`.
- `jq -e '.external_references[] | select(.sha256 == "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")' corpus/manifest/synthetic-video-corpus.json` - FOUND placeholder-like external reference.
- Cleanup checks: no staged files; no `tests/fixtures/corpus/**/reports/qa*`; no matching `frametrace qa`/cargo temp processes; no tracked diff under `.omo/boulder.json`, `.omo/start-work/ledger.jsonl`, or `.omo/plans`.

## Blockers

1. Replace or remove the placeholder external reference. If T12 needs a large/hash-only external corpus entry, the manifest/schema/docs/tests must model a real hash-only reference with provenance and must reject placeholder hashes or missing provenance. If no external corpus is available for T12, mark that corpus unsupported instead of shipping a fake all-`f` SHA.
2. Update the programming/remove-ai-slops review evidence for the fixed diff. The report must explicitly cover the current `src/qa_accuracy.rs` split, all `src/qa_accuracy/*.rs` modules, test-overfit/slop checks, and the 250 pure-LOC result. The stale report that defers the old size issue cannot support approval.

## Evidence Gaps

- No current code-review/slop report supports the actual fixed T12 diff.
- No artifact proves the `large-non-client-video-reference` external hash is real or provenance-backed.
- No parser/test check rejects placeholder external hashes or missing external-reference provenance.

## Adversarial Classes

- `malformed_input`: pass for tested paths. Bad hash exits nonzero with parsed mismatch metrics; missing `source_artifact_id` exits nonzero with typed serde error.
- `dirty_worktree`: pass with caveat. The worktree is broadly dirty from active plan work; no staged files were present.
- `stale_state`: needs-fix. Fresh CLI reports were generated and parsed, but the programming/remove-ai-slops review artifact is stale.
- `misleading_success_output`: pass. I relied on exit codes and parsed JSON, not success prose.
- `flaky_tests`: pass. Targeted T12 tests passed fresh.
- `hung_or_long_commands`: pass. Fresh commands completed; no matching temp processes remained.
- `prompt_injection`: not applicable. Manifests are parsed as typed data and not executed as instructions.
- `cancel_resume`: not applicable. No cancel/resume flow changed.
- `repeated_interruptions`: not applicable. No interrupt-handling path changed.

## Cleanup Assessment

- No staged files: `git diff --cached --name-only` produced no output.
- No fixture QA leftovers: `find tests/fixtures/corpus -path '*/reports/qa*' -print` produced no output.
- No T12 temp processes: `pgrep -lf 'target/debug/frametrace|frametrace qa|cargo test|cargo clippy|cargo build'` produced no output.
- No tracked plan/boulder/ledger diff from my review command: `git diff --name-only -- .omo/boulder.json .omo/start-work/ledger.jsonl .omo/plans` produced no output.
- I did not edit product files, plan checkboxes, `.omo/boulder.json`, `.omo/start-work/ledger.jsonl`, stage, or commit. This markdown report is my only intended write.

## Final Recommendation

needs-fix
