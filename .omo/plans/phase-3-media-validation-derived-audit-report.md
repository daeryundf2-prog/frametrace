# Phase 3 Media Validation, Derived Artifacts, Audit Chain, And Report Defensibility

## TL;DR
> Summary:      Harden FrameTrace's media validation and derived-artifact chain of custody without a WinUI surface or SQLite migration. The source of truth remains chained JSONL logs; reports and viewers must disclose validation, derived provenance, failures, skipped/unsupported states, and audit-chain status.
> Deliverables:
> - Validation command provenance contract with operator, method, tool/version, command args, source artifact ID/hash, target hash, timestamp, and audit chain.
> - Consistent derived-artifact contract for clip export, proxy, thumbnail, and frame capture.
> - Explicit no-overwrite and source/output alias guards for derived outputs.
> - Report, evidence viewer, and report-defense QA disclosure of validation state, provenance, failures, skipped/unsupported records, and chain verification.
> - Focused failing-first Rust and CLI tests plus real-surface QA evidence.
> Effort:       Large
> Risk:         High - multiple existing Rust modules exceed the preferred size, the branch is dirty, and current dirty tests already reference missing Phase 3 contract functions.

## Scope
### Must have
- Preserve existing uncommitted GUI/inventory/media changes; do not revert or normalize unrelated dirty files.
- Keep JSONL audit logs as the source of truth for Phase 3; do not introduce a large SQLite migration.
- Use failing-first tests before production edits for each behavior change.
- Resolve the current dirty validation contract gap in `src/validation.rs:285` where tests reference `validation_log_body_json`, `ValidationOptions.operator`, and new provenance fields not implemented at `src/validation.rs:10`.
- Define one naming rule:
  - `artifact_id`: ID for the output or validated artifact record.
  - `source_artifact_id`: ID for the input evidence/artifact being validated or used as source.
  - `source_artifact_sha256`: actual SHA-256 of the input/source file when the file is readable.
  - `source_index_sha256`: optional hash recorded by the index or prior log; may be `null`.
  - IDs use `<kind>-<selector-or-file-stem>-<first12sha256>` after sanitizing with the existing filename rules in `src/video_export.rs:267`.
- Add optional `--operator <name>` to validation and media mutation CLI commands; if omitted, resolve from `case.json.operator`, then `USER`/`USERNAME`; fail if no non-empty operator can be resolved.
- Keep inventory's SQLite `ffprobe_ok` state unchanged in this phase (`src/case_db/inventory_query.rs:196`); report/viewer confirmation overlays must use validation-log hash match plus verified chain.
- Treat missing source index hashes as non-blocking: log `source_index_sha256:null` and `source_hash_status:"computed"` if the actual file hash was computed.
- Add frame capture as a CLI/engine derived photo artifact, distinct from thumbnail:
  - Thumbnail remains review-sized JPEG under `artifacts/thumbnails`.
  - Frame capture writes a report-defensible still image under `artifacts/frames`, logs `kind:"frame-capture"` and exact `time_seconds`.
- Add chain status collection outside renderers; renderers receive both JSONL bodies and chain-status JSON rather than opening files internally.
- Reports/viewers must tolerate older v1 validation and v2 derived records for display, but Phase 3 QA must require new records to carry the new fields.

### Must NOT have (guardrails, anti-slop, scope boundaries)
- No WinUI work.
- No large SQLite migration or inventory schema rewrite.
- No new runtime dependency such as `serde`/`serde_json` unless the executor gets explicit approval in a later implementation turn.
- No weakening, deleting, skipping, or reverting existing failing tests.
- No broad refactor unrelated to validation, derived artifacts, audit chain, report/viewer disclosure, or report-defense QA.
- No "manual examiner tested it" acceptance criteria; all verification is agent-executed and evidence-backed.
- No report language claiming legal admissibility or court readiness; preserve disallowed-claim scanning in `src/qa_report_defense.rs:7`.

## Verification strategy
> Zero human intervention - all verification is agent-executed.
- Test decision: TDD + Rust unit/integration tests (`cargo test`), CLI real-surface scenarios, and browser-driven HTML checks.
- QA policy: every task has agent-executed scenarios.
- Evidence: `.omo/evidence/task-<N>-phase-3-media-validation-derived-audit-report.<ext>`

## Execution strategy
### Parallel execution waves
> Target 5-8 tasks per wave. <3 per wave (except final) = under-splitting.
> Extract shared dependencies as Wave-1 tasks to maximize parallelism.

Wave 1 (no dependencies):
- Task 1: Shared media audit contract and current red validation seam
- Task 2: Output alias/no-clobber guard and failed-output cleanup
- Task 3: CLI operator plumbing for existing durable media commands
- Task 4: Validation provenance resolver and hash-matched promotion rules
- Task 5: Audit-chain status collector and renderer input contract
- Task 6: CLI/media integration test harness fixtures

Wave 2 (after Wave 1):
- Task 7: Validation command schema v2 and chained validation log contract depends [1, 3, 4, 5, 6]
- Task 8: Export/proxy/thumbnail derived-artifact schema v3 depends [1, 2, 3, 5, 6]
- Task 9: Frame capture derived artifact command depends [1, 2, 3, 5, 6]
- Task 10: Report provenance and audit-chain disclosure depends [5, 7, 8, 9]
- Task 11: Evidence viewer/review provenance disclosure depends [4, 5, 7, 8, 9]

Wave 3 (after Wave 2):
- Task 12: Report-defense QA, reproducibility, and old-log tolerance depends [7, 8, 9, 10, 11]
- Task 13: End-to-end CLI and browser QA contract depends [7, 8, 9, 10, 11, 12]
- Task 14: User-facing docs/help examples depends [7, 8, 9, 10, 11, 12, 13]

Critical path: Task 1 -> Task 7 -> Task 10 -> Task 12 -> Task 13 -> Task 14

### Dependency matrix
| Task | Depends on | Blocks | Can parallelize with |
|------|------------|--------|----------------------|
| 1    | none       | 7, 8, 9 | 2, 3, 4, 5, 6 |
| 2    | none       | 8, 9 | 1, 3, 4, 5, 6 |
| 3    | none       | 7, 8, 9 | 1, 2, 4, 5, 6 |
| 4    | none       | 7, 11 | 1, 2, 3, 5, 6 |
| 5    | none       | 7, 8, 9, 10, 11 | 1, 2, 3, 4, 6 |
| 6    | none       | 7, 8, 9, 13 | 1, 2, 3, 4, 5 |
| 7    | 1, 3, 4, 5, 6 | 10, 11, 12, 13, 14 | 8, 9 |
| 8    | 1, 2, 3, 5, 6 | 10, 11, 12, 13, 14 | 7, 9 |
| 9    | 1, 2, 3, 5, 6 | 10, 11, 12, 13, 14 | 7, 8 |
| 10   | 5, 7, 8, 9 | 12, 13, 14 | 11 |
| 11   | 4, 5, 7, 8, 9 | 12, 13, 14 | 10 |
| 12   | 7, 8, 9, 10, 11 | 13, 14 | none |
| 13   | 7, 8, 9, 10, 11, 12 | 14 | none |
| 14   | 7, 8, 9, 10, 11, 12, 13 | final verification | none |

## Todos
> Implementation + Test = ONE task. Never separate.
> Every task MUST have: References + Acceptance Criteria + QA Scenarios + Commit.

- [ ] 1. Shared media audit contract and current red validation seam

  What to do: First capture the existing failing contract from the dirty tree. Then add a small shared Rust module, likely `src/media_audit.rs`, and register it in `src/lib.rs`. Keep it focused on operator resolution, artifact ID generation, source hash status, JSON string helpers, command-args helpers, and log-body field naming. Implement the missing `ValidationOptions.operator` and `validation_log_body_json` seam so the current dirty test in `src/validation.rs:285` compiles and passes. If adding this logic pushes `src/validation.rs` further over the preferred size, move reusable builders into `src/media_audit.rs` instead of adding more private helpers to `src/validation.rs`.
  Must NOT do: Do not add `serde`/`serde_json`. Do not modify unrelated dirty GUI/inventory files. Do not change CLI behavior yet except what is needed to compile the new option field.

  Parallelization: Can parallel: YES | Wave 1 | Blocks: [7, 8, 9] | Blocked by: []

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `src/validation.rs:285` - existing dirty failing contract test for operator/method/tool/source artifact fields.
  - Pattern:  `src/validation.rs:127` - current validation log body is hand-built and missing the Phase 3 schema.
  - Pattern:  `src/audit.rs:131` - existing JSON string-array helper for command arguments.
  - Pattern:  `src/util.rs:76` - existing JSON escaping style to reuse.
  - Pattern:  `src/video_export.rs:267` - existing filename sanitization rule to mirror for IDs.
  - API/Type: `src/validation.rs:10` - `ValidationOptions` currently only carries `ffprobe_bin`.
  - Test:     `src/validation.rs:285` - must become green without deleting the test.
  - External: `https://ffmpeg.org/ffprobe.html#json` - primary reference for JSON ffprobe output.

  Acceptance criteria (agent-executable only):
  - [ ] Before production edits, capture the current red test output with `cargo test validation_log_body_records_forensic_promotion_contract -- --exact 2>&1 | tee .omo/evidence/task-1-phase-3-media-validation-derived-audit-report-red.txt`.
  - [ ] After implementation, `cargo test validation_log_body_records_forensic_promotion_contract -- --exact` exits 0.
  - [ ] `cargo test media_audit --lib` exits 0 and covers ID generation, operator fallback failure, command-args JSON, and missing indexed-source hash behavior.
  - [ ] `rg -n "serde|serde_json" Cargo.toml src` shows no new serde dependency or imports.

  QA scenarios (MANDATORY - task incomplete without these):
  > Name the exact tool AND its exact invocation - not "verify it works". Browser use: use Chrome to drive the page; if Chrome is not available, download and use agent-browser (https://github.com/vercel-labs/agent-browser). Computer use: OS-level GUI automation for a non-browser desktop app.
  ```
  Scenario: validation log body contract emits Phase 3 fields
    Tool:     bash
    Steps:    cargo test validation_log_body_records_forensic_promotion_contract -- --exact 2>&1 | tee .omo/evidence/task-1-phase-3-media-validation-derived-audit-report.txt
    Expected: exit 0 and output includes "validation_log_body_records_forensic_promotion_contract ... ok"
    Evidence: .omo/evidence/task-1-phase-3-media-validation-derived-audit-report.txt

  Scenario: missing operator is rejected by helper
    Tool:     bash
    Steps:    cargo test media_audit_rejects_missing_operator --lib -- --exact 2>&1 | tee .omo/evidence/task-1-phase-3-media-validation-derived-audit-report-error.txt
    Expected: exit 0 and assertion checks the exact message "operator is required"
    Evidence: .omo/evidence/task-1-phase-3-media-validation-derived-audit-report-error.txt
  ```

  Commit: YES | Message: `test(media-audit): define provenance contract helpers` | Files: [src/media_audit.rs, src/lib.rs, src/validation.rs]

- [ ] 2. Output alias/no-clobber guard and failed-output cleanup

  What to do: Add one reusable guard, likely `reject_source_output_path`, beside `require_case_output_path` in `src/tool_policy.rs`. It must reject canonical or lexically equivalent source/output paths before running ffmpeg, including symlink/case-root aliases where possible. Apply it to clip export, proxy, thumbnail, and future frame capture. Keep the existing explicit-output exists checks and ffmpeg `-n`. Add temp-output or cleanup behavior so a failed ffmpeg run leaves no partial output at the requested final path; if temp+rename is too invasive, delete the final output on failure and test that policy.
  Must NOT do: Do not loosen case-root containment from `src/tool_policy.rs:78`. Do not rely only on ffmpeg `-n` for source/output protection.

  Parallelization: Can parallel: YES | Wave 1 | Blocks: [8, 9] | Blocked by: []

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `src/tool_policy.rs:78` - existing case-root output policy to extend.
  - Pattern:  `src/tool_policy.rs:180` - existing output-policy tests to mirror.
  - Pattern:  `src/artifacts.rs:56` - proxy explicit-output exists guard.
  - Pattern:  `src/artifacts.rs:112` - thumbnail explicit-output exists guard.
  - Pattern:  `src/video_export.rs:54` - export explicit-output exists guard.
  - Pattern:  `src/artifacts.rs:165` - proxy ffmpeg args already include `-n`.
  - Pattern:  `src/video_export.rs:113` - export ffmpeg args already include `-n`.
  - External: `https://ffmpeg.org/ffmpeg.html#n` - `ffmpeg -n` no-overwrite behavior.
  - External: `https://ffmpeg.org/ffmpeg.html#y` - `ffmpeg -y` overwrite behavior to guard against.

  Acceptance criteria (agent-executable only):
  - [ ] `cargo test reject_source_output_path --lib` exits 0.
  - [ ] `cargo test output_policy --lib` exits 0.
  - [ ] `cargo test derived_output_cleanup --lib` exits 0 and proves failed ffmpeg output is not left at the final path.
  - [ ] `rg -n "\"-y\"|overwrite" src/artifacts.rs src/video_export.rs src/tool_policy.rs` finds no ffmpeg overwrite option in media mutation commands.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: export rejects source/output alias before ffmpeg
    Tool:     bash
    Steps:    cargo test export_rejects_source_output_alias --lib -- --exact 2>&1 | tee .omo/evidence/task-2-phase-3-media-validation-derived-audit-report.txt
    Expected: exit 0 and assertion checks "output path must differ from source evidence path"
    Evidence: .omo/evidence/task-2-phase-3-media-validation-derived-audit-report.txt

  Scenario: preexisting output remains protected
    Tool:     bash
    Steps:    cargo test derived_outputs_refuse_existing_explicit_output --lib -- --exact 2>&1 | tee .omo/evidence/task-2-phase-3-media-validation-derived-audit-report-error.txt
    Expected: exit 0 and assertion checks "output already exists"
    Evidence: .omo/evidence/task-2-phase-3-media-validation-derived-audit-report-error.txt
  ```

  Commit: YES | Message: `fix(media): reject source output aliases` | Files: [src/tool_policy.rs, src/artifacts.rs, src/video_export.rs]

- [ ] 3. CLI operator plumbing for existing durable media commands

  What to do: Add optional `--operator <operator>` to `validate-artifact`, `export-video`, `make-proxy`, and `make-thumbnail` in `src/cli/commands.rs`, thread it through `src/cli/mod.rs`, `src/cli/media_cmd.rs`, and `src/cli/handlers.rs`. Resolution order is explicit flag, `case.json.operator`, then `USER`/`USERNAME`; if all are missing or blank, return an error. Keep current command names and existing examples working when case/env operator exists.
  Must NOT do: Do not add operator fields to unrelated inventory commands already handled in `src/cli/inventory_cmd.rs`. Do not require a GUI operator source.

  Parallelization: Can parallel: YES | Wave 1 | Blocks: [7, 8, 9] | Blocked by: []

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `src/cli/commands.rs:101` - `ExportVideo` command definition.
  - Pattern:  `src/cli/commands.rs:114` - `MakeProxy` command definition.
  - Pattern:  `src/cli/commands.rs:123` - `MakeThumbnail` command definition.
  - Pattern:  `src/cli/commands.rs:172` - `ValidateArtifact` command definition.
  - Pattern:  `src/cli/mod.rs:133` - export dispatch.
  - Pattern:  `src/cli/mod.rs:150` - proxy dispatch.
  - Pattern:  `src/cli/mod.rs:156` - thumbnail dispatch.
  - Pattern:  `src/cli/mod.rs:213` - validation dispatch.
  - Pattern:  `src/cli/handlers.rs:737` - existing env-based default operator helper.
  - API/Type: `src/model.rs:13` - case manifest stores optional operator.
  - Test:     `tests/cli_smoke.rs:18` - CLI test helper pattern.

  Acceptance criteria (agent-executable only):
  - [ ] `cargo test cli_media_operator --test cli_media_contract` exits 0.
  - [ ] `cargo run -- --help` includes unchanged command names.
  - [ ] `cargo run -- validate-artifact --help` includes `--operator`.
  - [ ] `cargo run -- export-video --help`, `cargo run -- make-proxy --help`, and `cargo run -- make-thumbnail --help` each include `--operator`.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: validate help discloses operator option
    Tool:     bash
    Steps:    cargo run -- validate-artifact --help 2>&1 | tee .omo/evidence/task-3-phase-3-media-validation-derived-audit-report.txt
    Expected: exit 0 and output contains "--operator <OPERATOR>"
    Evidence: .omo/evidence/task-3-phase-3-media-validation-derived-audit-report.txt

  Scenario: missing operator fails when no case/env fallback is available
    Tool:     bash
    Steps:    env -u USER -u USERNAME cargo test cli_media_missing_operator_fails --test cli_media_contract -- --exact 2>&1 | tee .omo/evidence/task-3-phase-3-media-validation-derived-audit-report-error.txt
    Expected: exit 0 and assertion checks "operator is required"
    Evidence: .omo/evidence/task-3-phase-3-media-validation-derived-audit-report-error.txt
  ```

  Commit: YES | Message: `feat(cli): add operator plumbing for media mutations` | Files: [src/cli/commands.rs, src/cli/mod.rs, src/cli/media_cmd.rs, src/cli/handlers.rs, tests/cli_media_contract.rs]

- [ ] 4. Validation provenance resolver and hash-matched promotion rules

  What to do: Replace path-only validation target resolution with a provenance-aware resolver that returns `SourceArtifactRef { source_artifact_id, kind, selector, path, source_index_sha256 }`. It must support direct paths, indexed video IDs, carved IDs, export/proxy/thumbnail/frame output artifact IDs, filesystem recovered inodes, and output paths. Add a display/promotion helper used by reports/viewers: `ffprobe-video-stream-confirmed` only counts as confirmed when the validation log chain is verified and `target_sha256` equals the selected record's current/source hash. Inventory state remains unchanged in this phase.
  Must NOT do: Do not mutate SQLite inventory validation state. Do not mark a record confirmed from path match alone.

  Parallelization: Can parallel: YES | Wave 1 | Blocks: [7, 11] | Blocked by: []

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `src/validation.rs:56` - current direct/index/log selector resolution.
  - Pattern:  `src/validation.rs:68` - logs currently searched for artifact outputs.
  - Pattern:  `src/validation.rs:90` - current log matching keys.
  - Pattern:  `src/video_export.rs:166` - indexed video selector resolver.
  - Pattern:  `src/html_report.rs:495` - viewer currently maps validation by path only.
  - Pattern:  `src/case_db/inventory_query.rs:196` - inventory validation state remains SQLite `ffprobe_ok`.
  - Test:     `src/validation.rs:250` - current log selector parsing test to expand.
  - External: `https://ffmpeg.org/ffprobe.html#show_streams` - stream-level validation basis.

  Acceptance criteria (agent-executable only):
  - [ ] `cargo test resolves_validation_target_with_provenance --lib` exits 0.
  - [ ] `cargo test validation_confirmation_requires_hash_match --lib` exits 0.
  - [ ] `cargo test validation_confirmation_requires_verified_chain --lib` exits 0.
  - [ ] `rg -n "validationsByPath|normalizePath\\(item.target_path\\)" src/html_report.rs` returns no path-only confirmation mapping after Task 11 lands.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: carved selector resolves with source artifact provenance
    Tool:     bash
    Steps:    cargo test resolves_carved_selector_with_source_artifact_id --lib -- --exact 2>&1 | tee .omo/evidence/task-4-phase-3-media-validation-derived-audit-report.txt
    Expected: exit 0 and assertion checks source_artifact_id starts with "carved-carve_000001-"
    Evidence: .omo/evidence/task-4-phase-3-media-validation-derived-audit-report.txt

  Scenario: stale validation hash does not promote
    Tool:     bash
    Steps:    cargo test stale_validation_hash_stays_candidate --lib -- --exact 2>&1 | tee .omo/evidence/task-4-phase-3-media-validation-derived-audit-report-error.txt
    Expected: exit 0 and assertion checks status remains "candidate-unvalidated"
    Evidence: .omo/evidence/task-4-phase-3-media-validation-derived-audit-report-error.txt
  ```

  Commit: YES | Message: `fix(validation): require provenance hash match for confirmation` | Files: [src/validation.rs, src/media_audit.rs, src/html_report.rs]

- [ ] 5. Audit-chain status collector and renderer input contract

  What to do: Add a small audit status collector that takes case-relative log paths and returns JSON-friendly statuses: `missing`, `empty`, `verified`, `tampered`, or `legacy-unverified`, with `entries`, `last_entry_sha256`, and `error`. Use existing `audit::verify_chained_jsonl` at `src/audit.rs:69`. Do not make renderers open files; update handler-layer wiring so report/viewer render functions receive log strings plus chain-status JSON. Document the single-writer assumption for `append_chained_jsonl` or implement atomic append/replace if needed, but do not build a locking subsystem unless tests prove it necessary.
  Must NOT do: Do not make `report.rs` or `html_report.rs` read from disk. Do not fail report generation solely because an optional log is missing; disclose missing status.

  Parallelization: Can parallel: YES | Wave 1 | Blocks: [7, 8, 9, 10, 11] | Blocked by: []

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `src/audit.rs:37` - current chained JSONL append implementation.
  - Pattern:  `src/audit.rs:69` - current chain verification implementation.
  - Pattern:  `src/cli/handlers.rs:286` - report handler reads logs before rendering.
  - Pattern:  `src/cli/handlers.rs:253` - review handler reads logs before rendering.
  - Pattern:  `src/report.rs:3` - report renderer currently receives JSONL strings only.
  - Pattern:  `src/html_report.rs:342` - evidence viewer currently receives carve/filesystem/validation logs only.
  - Test:     `src/audit.rs:204` - chain append/verify tests to mirror.

  Acceptance criteria (agent-executable only):
  - [ ] `cargo test audit_status_reports_verified_missing_and_tampered_logs --lib` exits 0.
  - [ ] `cargo test report_renderer_does_not_open_log_paths --lib` exits 0 or equivalent static assertion exists.
  - [ ] `cargo test make_report_includes_audit_chain_status --test cli_media_contract` exits 0 after Task 10.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: verified validation log reports chain status
    Tool:     bash
    Steps:    cargo test audit_status_reports_verified_validation_log --lib -- --exact 2>&1 | tee .omo/evidence/task-5-phase-3-media-validation-derived-audit-report.txt
    Expected: exit 0 and assertion checks "verified" plus entries > 0
    Evidence: .omo/evidence/task-5-phase-3-media-validation-derived-audit-report.txt

  Scenario: tampered validation log is disclosed not ignored
    Tool:     bash
    Steps:    cargo test audit_status_reports_tampered_validation_log --lib -- --exact 2>&1 | tee .omo/evidence/task-5-phase-3-media-validation-derived-audit-report-error.txt
    Expected: exit 0 and assertion checks "tampered" plus an entry hash mismatch error
    Evidence: .omo/evidence/task-5-phase-3-media-validation-derived-audit-report-error.txt
  ```

  Commit: YES | Message: `feat(audit): expose log chain status for reports` | Files: [src/audit.rs, src/cli/handlers.rs, src/report.rs, src/html_report.rs]

- [ ] 6. CLI/media integration test harness fixtures

  What to do: Add `tests/cli_media_contract.rs` with shared helpers based on `tests/cli_smoke.rs:6` and `tests/cli_inventory.rs:10`. Generate tiny media fixtures with `ffmpeg` when available; otherwise use deterministic fake files for error-path tests and skip only the ffmpeg-dependent happy path with an explicit message captured in evidence. Helpers must create temp case/media dirs, run `init-case --operator`, `scan-folder --hash --no-ffprobe`, read JSONL logs, and run `verify-audit`.
  Must NOT do: Do not weaken tests to pass without checking fields. Do not require external sample media checked into the repo.

  Parallelization: Can parallel: YES | Wave 1 | Blocks: [7, 8, 9, 13] | Blocked by: []

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `tests/cli_smoke.rs:6` - binary path helper.
  - Pattern:  `tests/cli_smoke.rs:10` - unique temp directory helper.
  - Pattern:  `tests/cli_inventory.rs:18` - CLI runner helper.
  - Pattern:  `docs/WINDOWS_IMPLEMENTATION_HANDOFF.md:96` - ffmpeg testsrc fixture command.
  - Pattern:  `Cargo.toml:1` - crate name and current dependency minimalism.
  - External: `https://ffmpeg.org/ffmpeg.html#version` - ffmpeg version availability reference.

  Acceptance criteria (agent-executable only):
  - [ ] `cargo test cli_media_fixture_bootstrap --test cli_media_contract -- --exact` exits 0.
  - [ ] `cargo test cli_media_missing_ffmpeg_is_explicit --test cli_media_contract -- --exact` exits 0.
  - [ ] Test helpers write command transcripts to `.omo/evidence/` when invoked by QA scenarios.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: media test fixture bootstraps a case and scanned video
    Tool:     bash
    Steps:    cargo test cli_media_fixture_bootstrap --test cli_media_contract -- --exact 2>&1 | tee .omo/evidence/task-6-phase-3-media-validation-derived-audit-report.txt
    Expected: exit 0 and output includes "cli_media_fixture_bootstrap ... ok"
    Evidence: .omo/evidence/task-6-phase-3-media-validation-derived-audit-report.txt

  Scenario: ffmpeg absence path is explicit
    Tool:     bash
    Steps:    cargo test cli_media_missing_ffmpeg_is_explicit --test cli_media_contract -- --exact 2>&1 | tee .omo/evidence/task-6-phase-3-media-validation-derived-audit-report-error.txt
    Expected: exit 0 and assertion checks "ffmpeg unavailable" or generated skip evidence, not silent pass
    Evidence: .omo/evidence/task-6-phase-3-media-validation-derived-audit-report-error.txt
  ```

  Commit: YES | Message: `test(cli): add media contract harness` | Files: [tests/cli_media_contract.rs]

- [ ] 7. Validation command schema v2 and chained validation log contract

  What to do: Wire the validation command through the new operator and provenance resolver. `validate-artifact` must append `schema_version:2` validation records with `event`, `operator`, `method`, `tool`, `tool_version`, `command`, `command_args`, `validated_unix`, `selector`, `artifact_id`, `source_artifact_id`, `source_artifact_sha256`, `source_index_sha256`, `source_hash_status`, `target_path`, `target_sha256`, `validation_artifact_path`, media fields, `validation_status`, `validation_note`, and chain fields added by `append_chained_jsonl`. Use `method:"ffprobe-container-video-stream"` only for video-stream confirmation; use distinct methods for parse failure and no-video-stream failure. Ensure stdout remains useful and includes operator/status/hash.
  Must NOT do: Do not promote by editing carve/export/proxy logs. Do not claim examiner playback review occurred.

  Parallelization: Can parallel: YES | Wave 2 | Blocks: [10, 11, 12, 13, 14] | Blocked by: [1, 3, 4, 5, 6]

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `src/validation.rs:33` - current validation engine sequence.
  - Pattern:  `src/validation.rs:108` - current validation status classifier.
  - Pattern:  `src/validation.rs:127` - current append function to replace/extend.
  - Pattern:  `src/ffprobe.rs:29` - exact ffprobe args currently executed.
  - Pattern:  `src/cli/handlers.rs:541` - handler job lifecycle and stdout.
  - Pattern:  `src/cli/media_cmd.rs:86` - CLI adapter.
  - Test:     `src/validation.rs:285` - existing Phase 3 body contract.
  - Test:     `src/audit.rs:204` - chain verification pattern.
  - External: `https://ffmpeg.org/ffprobe.html#show_format` - format/container info reference.
  - External: `https://ffmpeg.org/ffprobe.html#show_streams` - stream info reference.

  Acceptance criteria (agent-executable only):
  - [ ] `cargo test validation_log_body_records_forensic_promotion_contract -- --exact` exits 0.
  - [ ] `cargo test validate_artifact_writes_schema_v2_chained_log --test cli_media_contract -- --exact` exits 0.
  - [ ] `cargo test validate_artifact_records_failure_method --test cli_media_contract -- --exact` exits 0.
  - [ ] `cargo run -- verify-audit <case>/evidence/logs/validation-log.jsonl` exits 0 for the CLI fixture.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: validation promotes only through schema v2 chained log
    Tool:     bash
    Steps:    cargo test validate_artifact_writes_schema_v2_chained_log --test cli_media_contract -- --exact 2>&1 | tee .omo/evidence/task-7-phase-3-media-validation-derived-audit-report.txt
    Expected: exit 0 and test asserts operator, method, tool_version, command_args, source_artifact_id, source_artifact_sha256, target_sha256, previous_entry_sha256, and entry_sha256
    Evidence: .omo/evidence/task-7-phase-3-media-validation-derived-audit-report.txt

  Scenario: unsupported/non-media validation records failure not confirmation
    Tool:     bash
    Steps:    cargo test validate_artifact_records_failure_method --test cli_media_contract -- --exact 2>&1 | tee .omo/evidence/task-7-phase-3-media-validation-derived-audit-report-error.txt
    Expected: exit 0 and test asserts validation_status "validation-failed" and method is not "ffprobe-container-video-stream"
    Evidence: .omo/evidence/task-7-phase-3-media-validation-derived-audit-report-error.txt
  ```

  Commit: YES | Message: `feat(validation): record forensic validation provenance` | Files: [src/validation.rs, src/cli/handlers.rs, src/cli/media_cmd.rs, tests/cli_media_contract.rs]

- [ ] 8. Export/proxy/thumbnail derived-artifact schema v3

  What to do: Upgrade export, proxy, and thumbnail log bodies to a consistent schema while preserving old display compatibility. New records use `schema_version:3`, `operator`, `artifact_id`, `source_artifact_id`, `source_artifact_sha256`, `source_index_sha256`, `source_hash_status`, `source_path`, `output_path`, `output_sha256`, `kind`, `method`, `tool:"ffmpeg"`, `tool_version`, `command`, `command_args`, timestamp, and operation-specific fields such as format/start/duration/max_width/time. Use the guard from Task 2. Existing log readers must continue to display old `source_index_sha256` records.
  Must NOT do: Do not rename existing command names. Do not overwrite existing outputs or source evidence.

  Parallelization: Can parallel: YES | Wave 2 | Blocks: [10, 11, 12, 13, 14] | Blocked by: [1, 2, 3, 5, 6]

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `src/video_export.rs:47` - export flow.
  - Pattern:  `src/video_export.rs:208` - current export log body.
  - Pattern:  `src/artifacts.rs:46` - proxy flow.
  - Pattern:  `src/artifacts.rs:102` - thumbnail flow.
  - Pattern:  `src/artifacts.rs:219` - current proxy/thumbnail log body.
  - Pattern:  `src/audit.rs:113` - existing indexed source hash lookup.
  - Test:     `src/video_export.rs:302` - export args test pattern.
  - Test:     `src/artifacts.rs:263` - artifact args test pattern.
  - External: `https://ffmpeg.org/ffmpeg.html#n` - no-overwrite reference.

  Acceptance criteria (agent-executable only):
  - [ ] `cargo test export_log_body_records_derived_artifact_contract --lib -- --exact` exits 0.
  - [ ] `cargo test derived_artifact_log_body_records_proxy_and_thumbnail_contract --lib -- --exact` exits 0.
  - [ ] `cargo test export_proxy_thumbnail_cli_logs_schema_v3 --test cli_media_contract -- --exact` exits 0.
  - [ ] `cargo run -- verify-audit <case>/artifacts/clips/export-log.jsonl`, `.../proxy-log.jsonl`, and `.../thumbnail-log.jsonl` exit 0 in the fixture case.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: export/proxy/thumbnail logs include derived provenance
    Tool:     bash
    Steps:    cargo test export_proxy_thumbnail_cli_logs_schema_v3 --test cli_media_contract -- --exact 2>&1 | tee .omo/evidence/task-8-phase-3-media-validation-derived-audit-report.txt
    Expected: exit 0 and test asserts operator, artifact_id, source_artifact_id, source_artifact_sha256, output_sha256, method, tool_version, command_args, and chain fields for all three logs
    Evidence: .omo/evidence/task-8-phase-3-media-validation-derived-audit-report.txt

  Scenario: explicit existing output is rejected for all derived commands
    Tool:     bash
    Steps:    cargo test export_proxy_thumbnail_refuse_existing_output --test cli_media_contract -- --exact 2>&1 | tee .omo/evidence/task-8-phase-3-media-validation-derived-audit-report-error.txt
    Expected: exit 0 and test asserts "output already exists" for export, proxy, and thumbnail
    Evidence: .omo/evidence/task-8-phase-3-media-validation-derived-audit-report-error.txt
  ```

  Commit: YES | Message: `feat(media): unify derived artifact audit logs` | Files: [src/video_export.rs, src/artifacts.rs, src/media_audit.rs, tests/cli_media_contract.rs]

- [ ] 9. Frame capture derived artifact command

  What to do: Add a new CLI command `capture-frame <case_dir> <selector> --time <seconds> [--output <path>] [--operator <operator>]`. Implement it as a derived photo artifact under `artifacts/frames/` with `frame-log.jsonl`, using ffmpeg single-frame extraction. Default output should be unique, case-contained, and likely `.png`; exact ffmpeg args must be logged. Add `artifacts/frames` to case layout. Make validation selector resolution and report/viewer log readers aware of frame outputs, but do not build GUI controls.
  Must NOT do: Do not treat frame capture as thumbnail. Do not add WinUI or browser GUI mutation code. Do not write outside the case directory.

  Parallelization: Can parallel: YES | Wave 2 | Blocks: [10, 11, 12, 13, 14] | Blocked by: [1, 2, 3, 5, 6]

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `src/artifacts.rs:102` - thumbnail single-frame ffmpeg flow to adapt without conflating semantics.
  - Pattern:  `src/artifacts.rs:191` - thumbnail ffmpeg arg shape.
  - Pattern:  `src/util.rs:13` - case layout directories to extend.
  - Pattern:  `src/cli/commands.rs:123` - similar media command definition.
  - Pattern:  `src/cli/mod.rs:156` - similar dispatch.
  - Pattern:  `docs/EVIDENCE_VIEWER_GUI.md:171` - frame capture is required as a derived artifact.
  - Pattern:  `docs/GUI_LARGE_INVENTORY_EXECUTION_PLAN.md:202` - frame captures are derived artifacts.
  - External: `https://ffmpeg.org/ffmpeg.html#version` - ffmpeg version capture reference.

  Acceptance criteria (agent-executable only):
  - [ ] `cargo test capture_frame_args_and_default_path --lib -- --exact` exits 0.
  - [ ] `cargo test capture_frame_cli_writes_schema_v3_log --test cli_media_contract -- --exact` exits 0.
  - [ ] `cargo run -- capture-frame --help` includes `--time`, `--operator`, and `--output`.
  - [ ] `cargo run -- verify-audit <case>/artifacts/frames/frame-log.jsonl` exits 0 in the fixture case.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: frame capture writes a chained derived photo artifact log
    Tool:     bash
    Steps:    cargo test capture_frame_cli_writes_schema_v3_log --test cli_media_contract -- --exact 2>&1 | tee .omo/evidence/task-9-phase-3-media-validation-derived-audit-report.txt
    Expected: exit 0 and test asserts kind "frame-capture", time_seconds, output_sha256, source_artifact_id, command_args, operator, and chain fields
    Evidence: .omo/evidence/task-9-phase-3-media-validation-derived-audit-report.txt

  Scenario: frame capture rejects negative or non-finite time
    Tool:     bash
    Steps:    cargo test capture_frame_rejects_invalid_time --lib -- --exact 2>&1 | tee .omo/evidence/task-9-phase-3-media-validation-derived-audit-report-error.txt
    Expected: exit 0 and assertion checks "--time must be a non-negative finite number"
    Evidence: .omo/evidence/task-9-phase-3-media-validation-derived-audit-report-error.txt
  ```

  Commit: YES | Message: `feat(media): add audited frame capture` | Files: [src/artifacts.rs, src/util.rs, src/cli/commands.rs, src/cli/mod.rs, src/cli/media_cmd.rs, src/cli/handlers.rs, tests/cli_media_contract.rs]

- [ ] 10. Report provenance and audit-chain disclosure

  What to do: Extend report inputs and handler wiring to include frame logs and audit-chain status JSON for all relevant logs. Update `render_case_report` to disclose source/derived relationships, operator, method, tool/version, command args, source and output hashes, validation failures, skipped/missing/unsupported/unverifiable states, chain status, and old-record compatibility. Add direct report renderer tests rather than relying only on file-exists smoke tests.
  Must NOT do: Do not make legal/court-ready claims. Do not hide missing/tampered chains.

  Parallelization: Can parallel: YES | Wave 2 | Blocks: [12, 13, 14] | Blocked by: [5, 7, 8, 9]

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `src/report.rs:3` - report input struct to extend.
  - Pattern:  `src/report.rs:17` - JSONL to array conversion.
  - Pattern:  `src/report.rs:140` - clip exports section.
  - Pattern:  `src/report.rs:143` - review artifacts section.
  - Pattern:  `src/report.rs:149` - validation section.
  - Pattern:  `src/report.rs:274` - export table rendering.
  - Pattern:  `src/report.rs:290` - derived artifact table rendering.
  - Pattern:  `src/report.rs:323` - validation result table rendering.
  - Pattern:  `src/cli/handlers.rs:286` - report handler log wiring.
  - Pattern:  `docs/FORENSIC_HARDENING_PLAN.md:718` - report must include versions/options/relationships/failures/chain status.
  - Test:     `tests/cli_smoke.rs:80` - smoke currently checks generation only.

  Acceptance criteria (agent-executable only):
  - [ ] `cargo test report_discloses_validation_and_derived_provenance --lib -- --exact` exits 0.
  - [ ] `cargo test report_discloses_audit_chain_statuses --lib -- --exact` exits 0.
  - [ ] `cargo test make_report_contains_phase3_disclosures --test cli_media_contract -- --exact` exits 0.
  - [ ] `rg -ni "court-ready|court-grade|court-proven|legal-grade" src/report.rs src/html_report.rs` returns no matches.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: generated case report discloses derived provenance and chain status
    Tool:     bash
    Steps:    cargo test make_report_contains_phase3_disclosures --test cli_media_contract -- --exact 2>&1 | tee .omo/evidence/task-10-phase-3-media-validation-derived-audit-report.txt
    Expected: exit 0 and test asserts report HTML contains source_artifact_id, source_artifact_sha256, output_sha256, command_args, operator, verified chain status, validation-failed, and frame-capture
    Evidence: .omo/evidence/task-10-phase-3-media-validation-derived-audit-report.txt

  Scenario: tampered chain is visibly disclosed in report
    Tool:     bash
    Steps:    cargo test make_report_discloses_tampered_audit_chain --test cli_media_contract -- --exact 2>&1 | tee .omo/evidence/task-10-phase-3-media-validation-derived-audit-report-error.txt
    Expected: exit 0 and test asserts report HTML contains "tampered" and the audit mismatch message
    Evidence: .omo/evidence/task-10-phase-3-media-validation-derived-audit-report-error.txt
  ```

  Commit: YES | Message: `feat(report): disclose media provenance and chain status` | Files: [src/report.rs, src/cli/handlers.rs, tests/cli_media_contract.rs]

- [ ] 11. Evidence viewer/review provenance disclosure

  What to do: Extend `render_evidence_viewer_html` and handler wiring to receive export/proxy/thumbnail/frame logs plus chain-status JSON. Add derived artifact rows to the evidence viewer with `kind:"derived"`, source IDs/hashes, output path/hash, method, operator, and chain status. Validation overlays must use hash-matched, chain-verified validation records from Task 4, not path-only `validationsByPath`. Keep generated review HTML bounded; disclose truncation and paging contract as currently done.
  Must NOT do: Do not modify the separate `gui/evidence-viewer/*` prototype unless a failing test proves the generated viewer depends on it. Do not load 100k/1M rows into generated HTML.

  Parallelization: Can parallel: YES | Wave 2 | Blocks: [12, 13, 14] | Blocked by: [4, 5, 7, 8, 9]

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `src/html_report.rs:342` - evidence viewer input signature currently lacks derived/frame logs.
  - Pattern:  `src/html_report.rs:489` - record construction currently includes videos, carve, and filesystem only.
  - Pattern:  `src/html_report.rs:495` - current path-only validation map to replace.
  - Pattern:  `src/html_report.rs:668` - validation detail attachment.
  - Pattern:  `src/html_report.rs:675` - truncation notice must stay.
  - Pattern:  `src/cli/handlers.rs:253` - make-review handler wiring.
  - Test:     `src/html_report.rs:716` - existing evidence viewer tests.
  - Test:     `tests/cli_inventory.rs:155` - CLI bounded review test.

  Acceptance criteria (agent-executable only):
  - [ ] `cargo test evidence_viewer_includes_derived_artifact_records --lib -- --exact` exits 0.
  - [ ] `cargo test evidence_viewer_requires_hash_matched_validation --lib -- --exact` exits 0.
  - [ ] `cargo test make_review_contains_phase3_provenance --test cli_media_contract -- --exact` exits 0.
  - [ ] `cargo test make_review_embeds_bounded_inventory_subset --test cli_inventory -- --exact` still exits 0.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: generated evidence viewer exposes derived provenance rows
    Tool:     bash
    Steps:    cargo test make_review_contains_phase3_provenance --test cli_media_contract -- --exact 2>&1 | tee .omo/evidence/task-11-phase-3-media-validation-derived-audit-report.txt
    Expected: exit 0 and test asserts evidence-viewer HTML contains frame-capture, proxy, thumbnail, export-video, source_artifact_id, output_sha256, operator, and verified chain status
    Evidence: .omo/evidence/task-11-phase-3-media-validation-derived-audit-report.txt

  Scenario: stale validation hash remains candidate in viewer
    Tool:     bash
    Steps:    cargo test evidence_viewer_requires_hash_matched_validation --lib -- --exact 2>&1 | tee .omo/evidence/task-11-phase-3-media-validation-derived-audit-report-error.txt
    Expected: exit 0 and generated HTML data keeps the stale record as candidate-unvalidated
    Evidence: .omo/evidence/task-11-phase-3-media-validation-derived-audit-report-error.txt
  ```

  Commit: YES | Message: `feat(viewer): show derived provenance with hash-matched validation` | Files: [src/html_report.rs, src/cli/handlers.rs, tests/cli_media_contract.rs]

- [ ] 12. Report-defense QA, reproducibility, and old-log tolerance

  What to do: Extend `qa report-defense` to verify required logs and chain status for validation, export, proxy, thumbnail, frame, carve, and filesystem logs when present. The checklist must flag missing required Phase 3 fields on new records, tampered chains, unsupported/skipped/unverifiable items, and active jobs. Keep old v1/v2 display compatibility but require new records to meet schema contracts. Update reproducibility normalization and fixtures so new fields are normalized only where time/path variability requires it.
  Must NOT do: Do not require legal/operator human signoff as an automated pass criterion. Do not make report-defense fail because optional logs are absent in cases that never ran the associated command; disclose optional absence.

  Parallelization: Can parallel: NO | Wave 3 | Blocks: [13, 14] | Blocked by: [7, 8, 9, 10, 11]

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `src/qa_report_defense.rs:10` - current report-defense checks.
  - Pattern:  `src/qa_report_defense.rs:71` - disallowed-claim scan.
  - Pattern:  `src/qa_repro.rs:40` - reproducibility core includes validation/carve/tsk logs.
  - Pattern:  `src/qa_test_fixtures.rs:38` - old minimal validation-log fixture shape.
  - Pattern:  `src/qa_tests.rs:76` - report-defense rejection test pattern.
  - Pattern:  `docs/FORENSIC_HARDENING_PLAN.md:728` - reports must include failed/skipped/missing/partial/unsupported/unverifiable items.
  - Pattern:  `docs/recovery-prd.md:38` - report-defense is an acceptance criterion.

  Acceptance criteria (agent-executable only):
  - [ ] `cargo test report_defense_verifies_phase3_media_audit_contract --lib -- --exact` exits 0.
  - [ ] `cargo test report_defense_rejects_tampered_media_audit_log --lib -- --exact` exits 0.
  - [ ] `cargo test reproducibility_normalizes_phase3_media_logs --lib -- --exact` exits 0.
  - [ ] `cargo run -- qa report-defense <fixture-case>` exits 0 after running validation/export/proxy/thumbnail/capture-frame/make-report/make-review in Task 13 fixture.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: report-defense passes with complete Phase 3 logs
    Tool:     bash
    Steps:    cargo test report_defense_verifies_phase3_media_audit_contract --lib -- --exact 2>&1 | tee .omo/evidence/task-12-phase-3-media-validation-derived-audit-report.txt
    Expected: exit 0 and checklist contains PASS rows for validation, export, proxy, thumbnail, frame, report, viewer, and chain status
    Evidence: .omo/evidence/task-12-phase-3-media-validation-derived-audit-report.txt

  Scenario: report-defense rejects tampered media log
    Tool:     bash
    Steps:    cargo test report_defense_rejects_tampered_media_audit_log --lib -- --exact 2>&1 | tee .omo/evidence/task-12-phase-3-media-validation-derived-audit-report-error.txt
    Expected: exit 0 and assertion checks "audit chain" plus "tampered"
    Evidence: .omo/evidence/task-12-phase-3-media-validation-derived-audit-report-error.txt
  ```

  Commit: YES | Message: `feat(qa): verify media audit provenance in report defense` | Files: [src/qa_report_defense.rs, src/qa_repro.rs, src/qa_test_fixtures.rs, src/qa_tests.rs]

- [ ] 13. End-to-end CLI and browser QA contract

  What to do: Add and run a single CLI integration scenario that creates a case, generates a real tiny video fixture with ffmpeg if available, scans with hash, validates it, exports a clip, makes a proxy, makes a thumbnail, captures a frame, validates at least one derived output, makes review/report, verifies every log with `verify-audit`, runs `qa report-defense`, and parses report/viewer HTML. Then drive the generated `review/evidence-viewer.html` and `reports/case-report.html` through real Chrome using Playwright; if Chrome is unavailable, download and use agent-browser.
  Must NOT do: Do not stop at `cargo test`. Do not skip browser proof for generated HTML disclosure.

  Parallelization: Can parallel: NO | Wave 3 | Blocks: [14] | Blocked by: [7, 8, 9, 10, 11, 12]

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `tests/cli_smoke.rs:60` - case lifecycle smoke structure.
  - Pattern:  `README.md:101` - current media command examples.
  - Pattern:  `README.md:137` - existing audit-chain/hash claim.
  - Pattern:  `src/cli/handlers.rs:590` - `verify-audit` command output.
  - Pattern:  `src/html_report.rs:650` - viewer media stage.
  - Pattern:  `src/report.rs:323` - validation report table.
  - External: `https://ffmpeg.org/ffmpeg.html#version` - ffmpeg version capture.

  Acceptance criteria (agent-executable only):
  - [ ] `cargo test phase3_media_chain_cli_lifecycle --test cli_media_contract -- --exact` exits 0.
  - [ ] `cargo fmt -- --check` exits 0.
  - [ ] `cargo check` exits 0.
  - [ ] `cargo test` exits 0.
  - [ ] Real Chrome QA writes screenshots and action logs under `.omo/evidence/`.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: full CLI lifecycle proves validation and derived audit chain
    Tool:     bash
    Steps:    cargo test phase3_media_chain_cli_lifecycle --test cli_media_contract -- --exact 2>&1 | tee .omo/evidence/task-13-phase-3-media-validation-derived-audit-report.txt
    Expected: exit 0 and test asserts logs for validation/export/proxy/thumbnail/frame all verify, report-defense passes, report/viewer files contain Phase 3 fields
    Evidence: .omo/evidence/task-13-phase-3-media-validation-derived-audit-report.txt

  Scenario: generated report and viewer render Phase 3 fields in real Chrome
    Tool:     playwright(real Chrome)
    Steps:    npm_config_yes=true npx -p playwright node - <<'NODE' | tee .omo/evidence/task-13-phase-3-media-validation-derived-audit-report-browser.log
              const { chromium } = require('playwright');
              const fs = require('fs');
              const path = require('path');
              const caseDir = fs.readFileSync('.omo/evidence/task-13-case-dir.txt', 'utf8').trim();
              const browser = await chromium.launch({ channel: 'chrome', headless: true }).catch(() => chromium.launch({ headless: true }));
              const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
              await page.goto('file://' + path.resolve(caseDir, 'review/evidence-viewer.html'));
              await page.screenshot({ path: '.omo/evidence/task-13-phase-3-media-validation-derived-audit-report-viewer.png', fullPage: true });
              const viewer = await page.locator('body').innerText();
              if (!viewer.includes('frame-capture') || !viewer.includes('ffprobe-video-stream-confirmed')) throw new Error('viewer missing Phase 3 fields');
              await page.goto('file://' + path.resolve(caseDir, 'reports/case-report.html'));
              await page.screenshot({ path: '.omo/evidence/task-13-phase-3-media-validation-derived-audit-report-report.png', fullPage: true });
              const report = await page.locator('body').innerText();
              if (!report.includes('source_artifact') || !report.includes('audit') || !report.includes('validation-failed')) throw new Error('report missing Phase 3 disclosures');
              await browser.close();
              console.log('PASS browser Phase 3 disclosure QA');
              NODE
    Expected: exit 0, browser log contains "PASS browser Phase 3 disclosure QA", and two PNG screenshots exist
    Evidence: .omo/evidence/task-13-phase-3-media-validation-derived-audit-report-browser.log
  ```

  Commit: YES | Message: `test(cli): cover phase 3 media audit lifecycle` | Files: [tests/cli_media_contract.rs]

- [ ] 14. User-facing docs/help examples

  What to do: Update README and Windows usage/handoff docs for the new `--operator` media mutation flag, `capture-frame`, new JSONL log fields at a high level, no-overwrite/source-output alias behavior, validation failure semantics, and report/viewer disclosure. Keep docs concise and aligned with executable commands from Task 13. Verify generated CLI help and docs examples do not mention WinUI or court-ready claims.
  Must NOT do: Do not expand scope into GUI prototype docs except to clarify that durable frame capture is engine/CLI-only in this phase.

  Parallelization: Can parallel: NO | Wave 3 | Blocks: final verification | Blocked by: [7, 8, 9, 10, 11, 12, 13]

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `README.md:65` - quickstart command list.
  - Pattern:  `README.md:101` - current media commands.
  - Pattern:  `README.md:126` - artifact output locations.
  - Pattern:  `README.md:137` - current audit-chain/hash statement.
  - Pattern:  `docs/WINDOWS_USAGE.md:132` - Windows export examples.
  - Pattern:  `docs/WINDOWS_USAGE.md:139` - Windows proxy/thumbnail examples.
  - Pattern:  `docs/WINDOWS_USAGE.md:164` - validation examples.
  - Pattern:  `docs/EVIDENCE_VIEWER_GUI.md:236` - validation command must record tool/method/operator/timestamp/source/audit.

  Acceptance criteria (agent-executable only):
  - [ ] `cargo run -- capture-frame --help` exits 0 and help matches documented command shape.
  - [ ] `rg -n "capture-frame|--operator|frame-log.jsonl|source_artifact_id" README.md docs/WINDOWS_USAGE.md docs/WINDOWS_IMPLEMENTATION_HANDOFF.md` finds expected docs.
  - [ ] `rg -ni "court-ready|court-grade|court-proven|legal-grade|WinUI" README.md docs/WINDOWS_USAGE.md docs/WINDOWS_IMPLEMENTATION_HANDOFF.md` returns no new prohibited claims for Phase 3 docs.
  - [ ] `cargo test` still exits 0 after docs edits.

  QA scenarios (MANDATORY - task incomplete without these):
  ```
  Scenario: CLI help and docs agree on capture-frame/operator
    Tool:     bash
    Steps:    { cargo run -- capture-frame --help; rg -n "capture-frame|--operator|frame-log.jsonl|source_artifact_id" README.md docs/WINDOWS_USAGE.md docs/WINDOWS_IMPLEMENTATION_HANDOFF.md; } 2>&1 | tee .omo/evidence/task-14-phase-3-media-validation-derived-audit-report.txt
    Expected: exit 0 and output contains capture-frame help plus docs references
    Evidence: .omo/evidence/task-14-phase-3-media-validation-derived-audit-report.txt

  Scenario: docs avoid prohibited legal and WinUI scope claims
    Tool:     bash
    Steps:    ! rg -ni "court-ready|court-grade|court-proven|legal-grade|WinUI" README.md docs/WINDOWS_USAGE.md docs/WINDOWS_IMPLEMENTATION_HANDOFF.md 2>&1 | tee .omo/evidence/task-14-phase-3-media-validation-derived-audit-report-error.txt
    Expected: exit 0 because no prohibited terms are found in updated Phase 3 docs
    Evidence: .omo/evidence/task-14-phase-3-media-validation-derived-audit-report-error.txt
  ```

  Commit: YES | Message: `docs(media): document phase 3 audit workflow` | Files: [README.md, docs/WINDOWS_USAGE.md, docs/WINDOWS_IMPLEMENTATION_HANDOFF.md]

## Final verification wave (MANDATORY - after all implementation tasks)
> Runs in PARALLEL. ALL must APPROVE. Surface results to the caller and wait for an explicit "okay" before declaring complete.
- [ ] F1. Plan compliance audit - every task done, every acceptance criterion met
- [ ] F2. Code quality review - diagnostics clean, idioms match, no dead code
- [ ] F3. Real manual QA - every QA scenario executed with evidence captured
- [ ] F4. Scope fidelity - nothing extra shipped beyond Must-Have, nothing Must-NOT-Have introduced

## Commit strategy
- One logical change per commit. Conventional Commits (`<type>(<scope>): <subject>` body + footer).
- Atomic: every commit builds and passes tests on its own. Because the branch starts with dirty failing Phase 3 contract tests, Task 1 is the first commit and must restore a buildable test baseline before later commits.
- No "WIP" / "fix typo squash later" commits on the final branch - clean up before merge.
- Follow the workspace Lore protocol in every commit body: include useful `Constraint:`, `Rejected:`, `Confidence:`, `Scope-risk:`, `Directive:`, `Tested:`, and `Not-tested:` trailers.
- Reference the plan file path in the final commit footer: `Plan: .omo/plans/phase-3-media-validation-derived-audit-report.md`.

## Success criteria
- All Must-Have items are implemented without reverting unrelated dirty GUI/inventory work.
- New validation records prove promotion through the engine validation command with operator, method, tool/version, command args, timestamp, source artifact ID/hash, target hash, and chain fields.
- Export, proxy, thumbnail, and frame capture are all derived artifacts with source/output provenance and no source/output overwrite path.
- Report and evidence viewer disclose validation states, failures, skipped/unsupported/unverifiable items, derived provenance, and audit-chain status.
- `cargo fmt -- --check`, `cargo check`, `cargo test`, CLI lifecycle QA, report-defense QA, and real Chrome report/viewer QA pass with evidence under `.omo/evidence/`.
- F1-F4 approve, and commit history is clean with the final plan footer.
