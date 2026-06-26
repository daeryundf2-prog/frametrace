# T12 Programming / Remove-AI-Slops Review

Scope reviewed: current T12 corpus manifests, QA accuracy/reproducibility metrics, tests, docs, non-client fixtures, and the split `qa_accuracy` implementation after the first gate fix.

## Current Split Reviewed

- `src/qa_accuracy.rs`: top-level orchestration for legacy and typed manifest accuracy reports.
- `src/qa_accuracy/contract.rs`: exact ground-truth schema contract and case/domain contract checks.
- `src/qa_accuracy/indexed.rs`: typed parsing of indexed evidence JSONL/TSV surfaces.
- `src/qa_accuracy/manifest.rs`: manifest loading, release-key semantics, and expected-evidence construction.
- `src/qa_accuracy/metrics.rs`: precision/recall/false-positive/false-negative/hash-mismatch computation.
- `src/qa_accuracy/report.rs`: machine-readable JSON report rendering.
- `src/qa_accuracy/schema.rs`: serde boundary types for the typed manifest.
- `src/qa_accuracy/types.rs`: internal report and metric data types.
- `src/qa_tests/accuracy.rs`: structural tests for typed schema fields, missing required fields, metrics, and synthetic-only release-key behavior.

Pure LOC check for the split production modules and the changed focused test file is under the 250-line threshold: `src/qa_tests/accuracy.rs` 244, `src/qa_accuracy.rs` 75, `contract.rs` 108, `indexed.rs` 96, `manifest.rs` 166, `metrics.rs` 56, `report.rs` 86, `schema.rs` 75, and `types.rs` 51.

## Programming Rust Criteria

- Boundary parsing: typed JSON corpus manifests parse through `serde` with `deny_unknown_fields`; legacy TSV compatibility remains isolated at the manifest boundary.
- Error handling: new production paths return `Result<_, String>` with context; no new `unwrap`/`expect` outside tests; no `unsafe` added.
- Exhaustiveness: owned enums `CorpusKind` and `DomainStatus` use explicit variants; release-key interpretation is isolated in a small helper.
- Allocation/control: report assembly uses explicit vectors and `serde_json` values for the CLI JSON artifact; no new dependency was added.
- Size and responsibility: the former oversized `src/qa_accuracy.rs` has been split by responsibility into focused modules; no `qa_accuracy` production module remains over 250 pure LOC, and the changed focused test file is now 244 pure LOC.

## Anti-Slop / Overfit / Test-Slop

- Tests assert parsed JSON fields and behavior, not prose strings or snapshots.
- Manual CLI QA exercises the real binary surface for accuracy and reproducibility, including a nonzero bad-manifest path.
- The bad manifest changes one expected hash and proves mismatch/false-negative metrics instead of relying on success text.
- Required-field tests mutate parsed JSON objects and remove fields such as `source_artifact_id`, `expected_hash`, `negative_controls`, and `notes`; they are not substring-only tests.
- The helper manifest in `src/qa_tests/accuracy.rs` now uses `external_references: []`, matching the committed synthetic corpus and avoiding unused placeholder-like test data.
- No tests were deleted, skipped, or weakened.
- No broad abstraction or new dependency was introduced. The module split follows existing Rust module boundaries and keeps parsing, indexing, metrics, and rendering separate.

## Fixture Privacy / Non-Client Handling

- Committed fixtures under `tests/fixtures/corpus/` contain lightweight synthetic/non-client source and expected artifact files plus JSONL/TSV evidence records.
- Manifest `source_sha256` and `ground_truth.expected_hash` values are computed from committed fixture bytes; the all-`f` placeholder external hash has been removed.
- No private/client evidence, media, raw images, E01 files, tokens, cookies, or PII were added.
- `corpus/manifest/synthetic-video-corpus.json` now has an empty `external_references` array. No large external corpus is claimed until a real hash and provenance record exist.

## Manifest Schema Parsing

- Typed manifest requires `schema_version`, `corpus_id`, `corpus_kind`, `release_keys`, `domains`, `cases`, and `external_references`.
- Supported domains must declare the exact ground-truth schema fields: `corpus_id`, `source_artifact_id`, `source_sha256`, `expected_artifact_type`, `expected_path_pattern`, `expected_hash`, `expected_timestamp_range`, `expected_state`, `negative_controls`, and `notes`.
- Supported domains must also declare non-empty expected-output schemas.
- Cases assigned to unsupported domains fail closed.
- Missing required fields fail at parse time; evidence: `manual-qa-missing-field.txt`.

## Synthetic-Only Release-Key Semantics

- `corpus_kind: synthetic` cannot claim `mixed_real_world_like` as pass/passed/supported/true.
- The committed synthetic manifest records `mixed_real_world_like: unsupported`.
- Accuracy report preserves unsupported domain/release-key status in machine-readable JSON.

## External Reference Semantics

- The current synthetic corpus does not include external references.
- The removed placeholder `ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff` hash is not release evidence.
- Future external references need a real non-placeholder SHA-256 and enforceable provenance fields before they can support release gating.

Verdict: PASS for current T12 scope.
