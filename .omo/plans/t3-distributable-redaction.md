# FrameTrace T3 Distributable Path Redaction

## TL;DR
> Summary:      Implement default path redaction for all distributable outputs while preserving original case SQLite/audit provenance locally. Reports, review bundles, generated/static viewers, and default packages must show source IDs, relative artifact paths, and redacted labels unless an operator explicitly opts into full local paths.
> Deliverables:
> - Shared distributable redaction policy and metadata contract.
> - CLI opt-in flags for local full-path report/review/package generation.
> - Redacted report, review, viewer, and package outputs by default.
> - QA leakage gate plus failing-first/e2e proof under `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/`.
> Effort:       Medium
> Risk:         Medium - package currently copies raw DB/log/index files that contain full local paths.

## Scope
### Must have
- Add one shared redaction policy for distributable outputs, likely `src/redaction.rs`, exported from `src/lib.rs`.
- Default policy shape:
  - `mode = "redacted"` and `full_local_paths = false`.
  - Source evidence paths render as `source:<source_id>/<relative_path>` or `source:<file_id>/<relative_path>` when no registered source ID exists.
  - Case-contained derived artifacts render as case-relative paths such as `artifacts/frames/frame_0001.jpg`.
  - Full source paths and `file://` URLs are omitted or set to `null` in default distributable payloads.
  - Every report/review/package artifact emits metadata similar to `path_redaction: { schema_version: 1, mode, full_local_paths, operator, generated_unix, source_of_truth: "case SQLite/audit logs retain full local provenance" }`.
- Add explicit opt-in for local/operator full path display/export:
  - Recommended CLI shape: `--include-local-paths --operator <name>` on `make-report`, `make-review`, and `package-case`.
  - `--include-local-paths` without `--operator` fails with a clear error.
  - Opt-in mode records operator and mode in report/review/package metadata and QA output.
- Redact default report output from `src/report.rs`.
- Redact default review bundle/index payload from `src/review_bundle.rs` and `src/html_report.rs`.
- Redact generated evidence viewer output from `src/html_report.rs` and static GUI display fallback in `gui/evidence-viewer/app.js`.
- Redact default package outputs in `src/package.rs`.
- Preserve original internal provenance:
  - Do not mutate source case `db/case.db`.
  - Do not rewrite source case audit logs under `evidence/logs` or `artifacts/**/**-log.jsonl`.
  - Default package must not copy raw full-path DB/audit-log provenance into the distributable tree. It should package redacted shadow indexes/reports/review artifacts and an audit-chain summary; raw full-path provenance package export requires the opt-in flag.
- Respect dirty worktree guardrails: do not revert existing T1/T2 dirty files `Cargo.toml`, `Cargo.lock`, `src/qa*.rs`, `tests/cli_smoke.rs`, or `scripts/qa`.

### Must NOT have (guardrails, anti-slop, scope boundaries)
- Do not globally remove `source_path`, `file_url`, or full path storage from SQLite, scan internals, audit logs, or internal QA inputs.
- Do not add a new dependency unless the executor proves it is already unavoidable; local code can handle this policy.
- Do not make redaction a visual-only CSS/JS masking layer while leaking full paths in embedded JSON.
- Do not treat `relative_path` as safe if it is actually absolute; normalize through the shared policy.
- Do not make opt-in implicit through environment variables or debug builds.
- Do not delete or rewrite user/worktree changes outside the redaction implementation.

## Verification strategy
> Zero human intervention - all verification is agent-executed.
- Test decision: TDD + Rust unit/integration tests, Node syntax check for GUI JS, and bash manual QA scripts.
- QA policy: every task has agent-executed scenarios.
- Evidence: `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-<N>-<slug>.<ext>`

## Execution strategy
### Parallel execution waves
> Target 5-8 tasks per wave. <3 per wave (except final) = under-splitting.
> Extract shared dependencies as Wave-1 tasks to maximize parallelism.

Wave 1 (no dependencies):
- Task 1: Shared redaction policy and unit contract
- Task 2: CLI opt-in plumbing and metadata contract
- Task 3: Redaction e2e fixture helpers and failing-first baseline test

Wave 2 (after Wave 1):
- Task 4: depends [1, 2, 3] - report/review JSON payload redaction
- Task 5: depends [1, 2, 3] - generated/static viewer redaction
- Task 6: depends [1, 2, 3] - default package redacted output

Wave 3 (after Wave 2):
- Task 7: depends [4, 5, 6] - QA leakage gate and opt-in reporting
- Task 8: depends [4, 5, 6, 7] - end-to-end manual proof and regression suite

Critical path: Task 1 -> Task 4 -> Task 7 -> Task 8

### Dependency matrix
| Task | Depends on | Blocks | Can parallelize with |
|------|------------|--------|----------------------|
| 1    | none       | 4, 5, 6, 7, 8 | 2, 3 |
| 2    | none       | 4, 5, 6, 7, 8 | 1, 3 |
| 3    | none       | 4, 5, 6, 8 | 1, 2 |
| 4    | 1, 2, 3    | 7, 8 | 5, 6 |
| 5    | 1, 2, 3    | 7, 8 | 4, 6 |
| 6    | 1, 2, 3    | 7, 8 | 4, 5 |
| 7    | 4, 5, 6    | 8 | none |
| 8    | 4, 5, 6, 7 | final verification | none |

## Todos
> Implementation + Test = ONE task. Never separate.
> Every task MUST have: References + Acceptance Criteria + QA Scenarios + Commit.

- [ ] 1. Shared Redaction Policy And Unit Contract

  What to do: Add a shared distributable redaction module, likely `src/redaction.rs`, and export it from `src/lib.rs`. Implement the policy enum/config, metadata JSON builder, source path display helpers, case-relative artifact helper, and file URL handling. Unit tests must cover Unix paths, Windows drive paths, file URLs, case-contained artifacts, source evidence paths, pre-redacted IDs, and opt-in metadata.
  Must NOT do: Do not change scan storage, SQLite schema, or audit log write paths in this task.

  Parallelization: Can parallel: YES | Wave 1 | Blocks: [4, 5, 6, 7, 8] | Blocked by: []

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `src/util.rs` - follow existing `json_escape`, `path_to_file_url`, and file helper style.
  - Pattern:  `src/model.rs:148` - current `VideoRecord::to_json` emits `source_path` and `file_url`; policy must avoid changing this internal serializer unless explicitly used for distributable shadows.
  - Pattern:  `src/model.rs:215` - `ScanResult::to_json` is internal scan output and currently emits `source_path`.
  - API/Type: `src/case_db/inventory_types.rs:12` - `InventoryRow` has `source_id`, `source_label`, `relative_path`, and `full_path`.
  - API/Type: `src/case_db/inventory_query.rs:92` - mapper currently sets `source_id` and `source_label` to `source_path`; policy must not assume these are already safe.
  - API/Type: `src/case_db/evidence.rs:78` - stable source ID shape is `src_<16 hex chars>` when a registered source is available.
  - Test:     `src/review_bundle.rs:172` - local module unit test style using temp dirs and string assertions.
  - Repo policy: `docs/security-review.md:11` - full path exposure in reports/viewer is pending security work.
  - Repo policy: `docs/security-review.md:28` - report privacy/redaction mode is remaining security work.

  Acceptance criteria (agent-executable only):
  - [ ] `cargo test redaction -- --nocapture` passes.
  - [ ] Unit assertions prove default metadata contains `"mode":"redacted"` and does not contain the temp root or `file://`.
  - [ ] Unit assertions prove opt-in metadata contains `"mode":"full-local-paths"`, `"full_local_paths":true`, and the operator value.

  QA scenarios (MANDATORY - task incomplete without these):
  > Name the exact tool AND its exact invocation - not "verify it works".
  ```
  Scenario: default policy redacts full local source paths
    Tool:     bash
    Steps:    mkdir -p .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction && cargo test redaction_default_policy_redacts_source_and_file_url -- --nocapture | tee .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-01-policy-default.txt
    Expected: command exits 0; evidence contains the test name and no assertion failure.
    Evidence: .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-01-policy-default.txt

  Scenario: opt-in policy records operator and permits full paths
    Tool:     bash
    Steps:    cargo test redaction_opt_in_records_operator -- --nocapture | tee .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-01-policy-optin.txt
    Expected: command exits 0; evidence contains `"full-local-paths"` or the passing test name.
    Evidence: .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-01-policy-optin.txt
  ```

  Commit: YES | Message: `feat(redaction): add distributable path policy` | Files: [`src/redaction.rs`, `src/lib.rs`]

- [ ] 2. CLI Opt-In Plumbing And Metadata Contract

  What to do: Add `--include-local-paths` and `--operator <name>` to `make-report`, `make-review`, and `package-case`; thread a redaction policy object through `src/cli/mod.rs` into `src/cli/handlers.rs`. Reject opt-in without operator. Default commands must use redacted mode. Preserve existing command names and output paths.
  Must NOT do: Do not add flags to unrelated export/proxy/thumbnail commands in this task.

  Parallelization: Can parallel: YES | Wave 1 | Blocks: [4, 5, 6, 7, 8] | Blocked by: []

  References:
  - Pattern:  `src/cli/commands.rs:89` - `MakeReview` currently only accepts `case_dir`.
  - Pattern:  `src/cli/commands.rs:93` - `MakeReport` currently only accepts `case_dir`.
  - Pattern:  `src/cli/commands.rs:95` - `PackageCase` currently accepts `case_dir` and optional `--output`.
  - Pattern:  `src/cli/mod.rs:123` - `make-review` dispatch currently passes only `case_dir`.
  - Pattern:  `src/cli/mod.rs:128` - `make-report` dispatch currently passes only `case_dir`.
  - Pattern:  `src/cli/mod.rs:129` - package dispatch builds `PackageOptions`.
  - API/Type: `src/cli/handlers.rs:256` - `make_review` owns manifest/index loading and viewer generation.
  - API/Type: `src/cli/handlers.rs:307` - `make_report` owns report generation.
  - API/Type: `src/cli/handlers.rs:350` - `package_case` owns package command output.
  - Test:     `tests/cli_smoke.rs:35` - CLI help smoke pattern.
  - Test:     `tests/cli_output_policy.rs:37` - failure assertion helper pattern.

  Acceptance criteria:
  - [ ] `cargo test --test cli_smoke help_command_succeeds -- --nocapture` passes with new help text.
  - [ ] A new CLI test proves `make-report <case> --include-local-paths` without `--operator` fails with the exact message `--include-local-paths requires --operator`.
  - [ ] `cargo run -- make-report --help` output contains `--include-local-paths` and `--operator`.

  QA scenarios:
  ```
  Scenario: default CLI uses redacted policy
    Tool:     bash
    Steps:    cargo run -- make-report --help | tee .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-02-help.txt
    Expected: command exits 0; evidence contains `--include-local-paths` and `--operator`.
    Evidence: .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-02-help.txt

  Scenario: opt-in without operator fails closed
    Tool:     bash
    Steps:    cargo test --test cli_redaction include_local_paths_requires_operator -- --nocapture | tee .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-02-optin-error.txt
    Expected: command exits 0; test asserts command failure and exact error text.
    Evidence: .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-02-optin-error.txt
  ```

  Commit: YES | Message: `feat(cli): require operator for local path opt-in` | Files: [`src/cli/commands.rs`, `src/cli/mod.rs`, `src/cli/handlers.rs`, `tests/cli_smoke.rs`, `tests/cli_redaction.rs`]

- [ ] 3. Redaction Fixture Helpers And Failing-First Baseline Test

  What to do: Add a focused integration test file, recommended `tests/cli_redaction.rs`, with helpers that create a temp root containing user/client-like names and non-ASCII text. Preserve the existing baseline evidence as failing-first proof and encode the expected final behavior as tests. The initial test should fail before Tasks 4-6 are implemented and pass after them.
  Must NOT do: Do not edit or delete existing evidence files under `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/`.

  Parallelization: Can parallel: YES | Wave 1 | Blocks: [4, 5, 6, 8] | Blocked by: []

  References:
  - Evidence: `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/baseline-leak-before-fix.md:1` - current report/package leak proof.
  - Evidence: `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/baseline-review-leak-before-fix.md:1` - current review/viewer leak proof.
  - Pattern:  `tests/cli_review.rs:10` - temp directory helper pattern.
  - Pattern:  `tests/cli_review.rs:18` - command runner pattern.
  - Pattern:  `tests/cli_output_policy.rs:37` - failure assertion pattern.
  - Pattern:  `tests/cli_default_output_policy.rs:163` - seeded indexed case helper style.
  - Test:     `tests/cli_review.rs:78` - review generation from SQLite inventory.

  Acceptance criteria:
  - [ ] `cargo test --test cli_redaction default_distributable_outputs_do_not_leak_temp_root -- --nocapture` passes after Tasks 4-6.
  - [ ] Test fixture root includes literal substrings `FrameTrace Client ACME`, `Examiner Shin`, and `유출`.
  - [ ] Test asserts default report/review/package text artifacts do not contain the temp root and do not contain `file://`.
  - [ ] Test asserts source case SQLite/audit files are not rewritten by comparing pre/post presence of full path in local provenance files.

  QA scenarios:
  ```
  Scenario: failing-first fixture captures current leak then passes after implementation
    Tool:     bash
    Steps:    cargo test --test cli_redaction default_distributable_outputs_do_not_leak_temp_root -- --nocapture | tee .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-03-default-e2e.txt
    Expected: final implementation exits 0; evidence contains the test name and no leaked temp root assertion.
    Evidence: .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-03-default-e2e.txt

  Scenario: local provenance remains untouched
    Tool:     bash
    Steps:    cargo test --test cli_redaction internal_sqlite_and_audit_provenance_stays_full_path -- --nocapture | tee .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-03-provenance.txt
    Expected: command exits 0; test asserts local `db/case.db` or seeded audit log still contains the full source path while distributable artifacts do not.
    Evidence: .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-03-provenance.txt
  ```

  Commit: YES | Message: `test(redaction): cover distributable path leaks` | Files: [`tests/cli_redaction.rs`]

- [ ] 4. Report And Review JSON Payload Redaction

  What to do: Use the shared policy when constructing report/review payloads. For review, replace `row.full_path` and `file_url` in SQLite rows and legacy JSONL rows before embedding. For report, sanitize embedded `scan`, export/proxy/thumbnail/frame/carve/filesystem/validation log arrays before `render_case_report` displays them. Add metadata to the embedded JSON and preserve opt-in full path behavior.
  Must NOT do: Do not sanitize original `db/video_index.json`, `db/videos.jsonl`, `db/video_paths.tsv`, SQLite, or audit logs in the case directory.

  Parallelization: Can parallel: YES | Wave 2 | Blocks: [7, 8] | Blocked by: [1, 2, 3]

  References:
  - Leak:     `src/review_bundle.rs:129` - `sqlite_row_json` currently creates `file_url` from `row.full_path`.
  - Leak:     `src/review_bundle.rs:132` - review JSON currently emits `"source_path"` and `"file_url"`.
  - Leak:     `src/review_bundle.rs:138` - review JSON inserts `row.full_path`.
  - Leak:     `src/html_report.rs:248` - review subtitle displays `scan.source_path`.
  - Leak:     `src/html_report.rs:322` - review table creates source link from `video.file_url`.
  - Leak:     `src/report.rs:263` - report processing table displays `scan.source_path`.
  - Leak:     `src/report.rs:332` - report clip export table displays `item.source_path`.
  - Leak:     `src/report.rs:333` - report clip export table displays `item.output_path`.
  - Leak:     `src/report.rs:352` - report derived artifact table displays `source_path`.
  - Leak:     `src/report.rs:353` - report derived artifact table displays `output_path`.
  - Leak:     `src/report.rs:371` - report carved artifact table displays `output_path`.
  - Leak:     `src/report.rs:385` - report validation table displays `target_path`.
  - Leak:     `src/report.rs:403` - report filesystem table displays `image_path`.
  - Leak:     `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/baseline-leak-before-fix.md:20` - current report embeds full source path and file URL.
  - Test:     `tests/media_contract.rs:8` - report provenance contract should be updated to expect source IDs and case-relative artifact paths by default.
  - Test:     `tests/cli_review.rs:35` - bounded review generation test to update for safe embedded paths.

  Acceptance criteria:
  - [ ] `cargo test --test media_contract report_discloses_derived_provenance_and_validation_failures -- --nocapture` passes with redacted path assertions.
  - [ ] `cargo test --test cli_review -- --nocapture` passes.
  - [ ] Default `reports/case-report.html`, `review/index.html`, and `review/evidence-viewer.html` contain `path_redaction` metadata and no temp root or `file://`.
  - [ ] Opt-in report/review output contains full path and opt-in metadata.

  QA scenarios:
  ```
  Scenario: default report and review payloads are redacted
    Tool:     bash
    Steps:    cargo test --test cli_redaction report_and_review_default_payloads_are_redacted -- --nocapture | tee .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-04-report-review-default.txt
    Expected: command exits 0; test asserts no temp root and no `file://` in generated report/review HTML.
    Evidence: .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-04-report-review-default.txt

  Scenario: operator opt-in report/review discloses full local paths with metadata
    Tool:     bash
    Steps:    cargo test --test cli_redaction report_and_review_opt_in_records_operator_and_full_paths -- --nocapture | tee .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-04-report-review-optin.txt
    Expected: command exits 0; test asserts full temp root appears only when `--include-local-paths --operator qa-local` is used and metadata records `qa-local`.
    Evidence: .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-04-report-review-optin.txt
  ```

  Commit: YES | Message: `feat(report): redact distributable report and review paths` | Files: [`src/review_bundle.rs`, `src/report.rs`, `src/html_report.rs`, `src/cli/handlers.rs`, `tests/media_contract.rs`, `tests/cli_review.rs`, `tests/cli_redaction.rs`]

- [ ] 5. Generated And Static Viewer Redaction

  What to do: Update generated evidence viewer records to use `display_path`, `artifact_path`, and redacted file URL policy. In default mode, do not create manual-open links to local files. Update `gui/evidence-viewer/app.js` so static/prototype display prefers redacted display labels and does not normalize future real payloads back to full paths in list, search, sort, or inspector views.
  Must NOT do: Do not remove validation matching by artifact ID/hash; path matching can remain as a fallback only for opt-in/local mode.

  Parallelization: Can parallel: YES | Wave 2 | Blocks: [7, 8] | Blocked by: [1, 2, 3]

  References:
  - Leak:     `src/html_report.rs:526` - generated viewer validates using `video.source_path`.
  - Leak:     `src/html_report.rs:531` - generated viewer record path is `video.source_path`.
  - Leak:     `src/html_report.rs:558` - derived record path uses full output path.
  - Leak:     `src/html_report.rs:581` - carved record path uses full output path.
  - Leak:     `src/html_report.rs:602` - filesystem record path uses full output path.
  - Leak:     `src/html_report.rs:647` - generated viewer creates file URLs from paths.
  - Leak:     `src/html_report.rs:736` - generated viewer list displays `record.path`.
  - Leak:     `src/html_report.rs:762` - generated viewer inspector displays `record.path`.
  - Leak:     `src/html_report.rs:782` - generated viewer case line displays `scan.source_path`.
  - Leak:     `gui/evidence-viewer/app.js:6` - static seed data includes local drive path.
  - Leak:     `gui/evidence-viewer/app.js:196` - generated mock records create path-like source labels.
  - Leak:     `gui/evidence-viewer/app.js:756` - static viewer searches `record.path`.
  - Leak:     `gui/evidence-viewer/app.js:987` - static viewer row displays `record.path`.
  - Leak:     `gui/evidence-viewer/app.js:1117` - static inspector displays `record.path`.
  - Test:     `src/html_report.rs:913` - generated viewer no-auto-load-media test should be extended to assert no file URL in redacted mode.

  Acceptance criteria:
  - [ ] `cargo test html_report::tests -- --nocapture` passes.
  - [ ] `node --check gui/evidence-viewer/app.js` passes.
  - [ ] Default generated viewer does not contain the temp root, `file://`, `E:/`, `D:/`, or `C:/Cases` unless those strings are in explicitly demo-only comments removed from runtime data.
  - [ ] Viewer still matches validation by artifact ID and hash.

  QA scenarios:
  ```
  Scenario: generated evidence viewer displays redacted labels only
    Tool:     bash
    Steps:    cargo test html_report::tests::evidence_viewer_redacts_default_paths -- --nocapture | tee .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-05-generated-viewer.txt
    Expected: command exits 0; test asserts no local path/file URL and preserved artifact IDs.
    Evidence: .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-05-generated-viewer.txt

  Scenario: static GUI viewer syntax and path labels are safe
    Tool:     bash
    Steps:    node --check gui/evidence-viewer/app.js | tee .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-05-static-viewer-node.txt && ! grep -nE '([A-Z]:/|file://|/Users/|/tmp/FrameTrace Client)' gui/evidence-viewer/app.js > .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-05-static-viewer-grep.txt
    Expected: node exits 0; grep exits 1 and evidence grep file is empty.
    Evidence: .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-05-static-viewer-grep.txt
  ```

  Commit: YES | Message: `feat(viewer): display redacted paths by default` | Files: [`src/html_report.rs`, `gui/evidence-viewer/app.js`]

- [ ] 6. Default Package Redacted Output

  What to do: Update `src/package.rs` so default `package-case` creates distributable-safe contents. It must copy redacted report/review artifacts, redacted shadow index files, and package metadata. It must not copy raw `db/case.db` or raw audit logs into default distributable packages because those intentionally retain internal full-path provenance. Add opt-in `--include-local-paths --operator <name>` behavior to include raw provenance files and record this in `package-manifest.json`.
  Must NOT do: Do not mutate the source case database or source audit logs; package redaction must happen in the output tree only.

  Parallelization: Can parallel: YES | Wave 2 | Blocks: [7, 8] | Blocked by: [1, 2, 3]

  References:
  - Leak:     `src/package.rs:58` - required files are copied directly from the case.
  - Leak:     `src/package.rs:130` - required package files include `db/case.db`, `db/video_index.json`, `db/videos.jsonl`, and `db/video_paths.tsv`.
  - Leak:     `src/package.rs:165` - recursive package dirs include `evidence/logs` and artifact logs.
  - Leak:     `src/package.rs:250` - `copy_package_file` copies files byte-for-byte.
  - Leak:     `src/package.rs:310` - package manifest currently has no redaction metadata.
  - Evidence: `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/baseline-leak-before-fix.md:22` - default package currently leaks full paths in artifact logs.
  - Evidence: `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/baseline-leak-before-fix.md:23` - default package currently leaks full paths in `db/videos.jsonl`.
  - Evidence: `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/baseline-leak-before-fix.md:25` - default package currently leaks full paths in `db/video_paths.tsv`.
  - Test:     `src/package.rs:354` - package unit test checks package contents and manifest.
  - Test:     `tests/cli_default_output_policy.rs:146` - package output symlink guard must continue passing.

  Acceptance criteria:
  - [ ] `cargo test package -- --nocapture` passes.
  - [ ] `cargo test --test cli_default_output_policy package_case_rejects_symlinked_default_reports_directory_without_writing_outside -- --nocapture` passes.
  - [ ] Default package manifest contains redaction metadata and lists excluded raw provenance files, or redacted shadow replacements, without leaking the temp root.
  - [ ] Opt-in package manifest records full-local-path mode and operator, and package may include raw DB/audit provenance.

  QA scenarios:
  ```
  Scenario: default package has no full local paths
    Tool:     bash
    Steps:    cargo test --test cli_redaction package_default_outputs_are_redacted -- --nocapture | tee .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-06-package-default.txt
    Expected: command exits 0; test greps package text artifacts and asserts no temp root, no `file://`, redaction metadata present.
    Evidence: .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-06-package-default.txt

  Scenario: opt-in package records operator and raw provenance inclusion
    Tool:     bash
    Steps:    cargo test --test cli_redaction package_opt_in_records_operator_and_includes_raw_provenance -- --nocapture | tee .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-06-package-optin.txt
    Expected: command exits 0; test asserts full temp root appears only with opt-in and package manifest records operator.
    Evidence: .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-06-package-optin.txt
  ```

  Commit: YES | Message: `feat(package): build redacted distributable packages` | Files: [`src/package.rs`, `src/cli/handlers.rs`, `tests/cli_redaction.rs`]

- [ ] 7. QA Leakage Gate And Opt-In Reporting

  What to do: Extend report-defense/privacy QA so generated report/review/package artifacts fail if they contain local path tokens without opt-in metadata. Record redaction mode, operator, and checked artifacts in QA JSON/markdown. The gate must pass redacted default outputs and explicitly disclose opt-in mode rather than silently passing leaked local paths.
  Must NOT do: Do not make QA inspect raw internal SQLite/audit logs as leaks when they are still in the source case directory; QA should target distributable outputs.

  Parallelization: Can parallel: NO | Wave 3 | Blocks: [8] | Blocked by: [4, 5, 6]

  References:
  - Pattern:  `src/qa_report_defense.rs:11` - report defensibility QA entry point.
  - Pattern:  `src/qa_report_defense.rs:100` - current report claim scan loops over report/viewer HTML.
  - Pattern:  `src/qa_tests.rs:74` - QA failure test style for report-defense checks.
  - Policy:   `docs/security-review.md:31` - release-time privacy leakage QA is pending.
  - Policy:   `docs/FORENSIC_HARDENING_PLAN.md:279` - security phase includes report/export private path leakage review.
  - Policy:   `docs/FORENSIC_HARDENING_PLAN.md:305` - risk notes that HTML reports may expose local paths unnecessarily.

  Acceptance criteria:
  - [ ] `cargo test qa_report_defense -- --nocapture` passes.
  - [ ] New QA test fails a report containing `/tmp/FrameTrace Client ACME` without redaction metadata.
  - [ ] New QA test allows opt-in full paths only when metadata contains `mode:"full-local-paths"` and non-empty operator.
  - [ ] QA output includes checked artifact list and redaction mode.

  QA scenarios:
  ```
  Scenario: QA rejects unredacted distributable output
    Tool:     bash
    Steps:    cargo test qa_report_defense_rejects_unredacted_local_paths -- --nocapture | tee .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-07-qa-rejects-leak.txt
    Expected: command exits 0; test asserts QA error contains `privacy leakage` or `unredacted local path`.
    Evidence: .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-07-qa-rejects-leak.txt

  Scenario: QA records explicit opt-in
    Tool:     bash
    Steps:    cargo test qa_report_defense_records_full_path_opt_in -- --nocapture | tee .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-07-qa-optin.txt
    Expected: command exits 0; generated checklist/JSON includes `full-local-paths` and operator.
    Evidence: .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-07-qa-optin.txt
  ```

  Commit: YES | Message: `feat(qa): gate distributable path leakage` | Files: [`src/qa_report_defense.rs`, `src/qa_tests.rs`, `tests/cli_redaction.rs`]

- [ ] 8. End-To-End Manual Proof And Regression Suite

  What to do: Run the full focused and broad verification suite, including manual grep under a temp path containing user/client-like names. Capture all evidence under the requested evidence directory. Fix any regression discovered by these commands before final verification.
  Must NOT do: Do not declare completion from grep-only evidence; grep complements tests and QA gates.

  Parallelization: Can parallel: NO | Wave 3 | Blocks: [final verification] | Blocked by: [4, 5, 6, 7]

  References:
  - Evidence: `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/baseline-leak-before-fix.md:18` - existing manual grep shape for default leak proof.
  - Evidence: `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/baseline-review-leak-before-fix.md:9` - existing review grep shape.
  - Pattern:  `tests/cli_lifecycle.rs:73` - lifecycle generation order includes review/report.
  - Pattern:  `docs/WINDOWS_VALIDATION.md:26` - documented report/review/package command order.
  - Test:     `tests/cli_redaction.rs` - new redaction e2e tests from this plan.

  Acceptance criteria:
  - [ ] `cargo fmt -- --check` passes.
  - [ ] `cargo clippy --all-targets -- -D warnings` passes.
  - [ ] `cargo test --test cli_redaction -- --nocapture` passes.
  - [ ] `cargo test --test cli_review -- --nocapture` passes.
  - [ ] `cargo test --test media_contract -- --nocapture` passes.
  - [ ] `cargo test` passes.
  - [ ] `node --check gui/evidence-viewer/app.js` passes.
  - [ ] `git diff --check` passes.
  - [ ] Manual default grep exits 1 and evidence file is empty for report/review/default package outputs.
  - [ ] Manual opt-in grep exits 0 and metadata grep proves operator is recorded.

  QA scenarios:
  ```
  Scenario: default distributable outputs under sensitive temp root have no path leaks
    Tool:     bash
    Steps:    EVID=.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction; mkdir -p "$EVID"; ROOT="$(mktemp -d '/tmp/FrameTrace Client ACME Redaction 유출.XXXXXX')"; CASE="$ROOT/Examiner Shin/Case Alpha"; SRC="$ROOT/Client ACME Source/Camera 01"; mkdir -p "$SRC"; printf '\0\0\0\030ftypmp42payload' > "$SRC/parking lot clip.mp4"; cargo run -- init-case "$CASE" --title "ACME Redaction QA" --operator "Examiner Shin"; cargo run -- scan-folder "$CASE" "$SRC" --no-ffprobe; cargo run -- make-report "$CASE"; cargo run -- make-review "$CASE"; cargo run -- package-case "$CASE" --output "$ROOT/package-default"; ! grep -R -n -F "$ROOT" "$CASE/reports" "$CASE/review" "$ROOT/package-default" --exclude='case.db' > "$EVID/task-08-manual-default-grep.txt"; printf 'root=%s\ncase=%s\n' "$ROOT" "$CASE" > "$EVID/task-08-manual-default-root.txt"
    Expected: all commands exit 0 except grep exits 1 through `!`; `task-08-manual-default-grep.txt` is empty; report/review/package metadata contains redacted mode.
    Evidence: .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-08-manual-default-grep.txt

  Scenario: explicit operator opt-in exports full paths and records metadata
    Tool:     bash
    Steps:    EVID=.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction; ROOT="$(mktemp -d '/tmp/FrameTrace Client ACME OptIn 유출.XXXXXX')"; CASE="$ROOT/Examiner Shin/Case OptIn"; SRC="$ROOT/Client ACME Source/Camera 02"; mkdir -p "$SRC"; printf '\0\0\0\030ftypmp42payload' > "$SRC/review clip.mp4"; cargo run -- init-case "$CASE" --title "ACME OptIn QA" --operator "Examiner Shin"; cargo run -- scan-folder "$CASE" "$SRC" --no-ffprobe; cargo run -- make-report "$CASE" --include-local-paths --operator "qa-local"; cargo run -- make-review "$CASE" --include-local-paths --operator "qa-local"; cargo run -- package-case "$CASE" --include-local-paths --operator "qa-local" --output "$ROOT/package-optin"; grep -R -n -F "$ROOT" "$CASE/reports" "$CASE/review" "$ROOT/package-optin" > "$EVID/task-08-manual-optin-grep.txt"; grep -R -n -E 'full-local-paths|qa-local' "$CASE/reports" "$CASE/review" "$ROOT/package-optin/package-manifest.json" > "$EVID/task-08-manual-optin-metadata.txt"
    Expected: all commands exit 0; full path grep exits 0; metadata grep exits 0 and includes `full-local-paths` plus `qa-local`.
    Evidence: .omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/task-08-manual-optin-metadata.txt
  ```

  Commit: YES | Message: `test(redaction): verify end-to-end distributable policy` | Files: [`tests/cli_redaction.rs`, `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/*`]

## Final verification wave (MANDATORY - after all implementation tasks)
> Runs in PARALLEL. ALL must APPROVE. Surface results to the caller and wait for an explicit "okay" before declaring complete.
- [ ] F1. Plan compliance audit - every task done, every acceptance criterion met.
- [ ] F2. Code quality review - `cargo fmt -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `node --check gui/evidence-viewer/app.js`, and `git diff --check` are clean.
- [ ] F3. Real manual QA - Task 8 default and opt-in scenarios executed with evidence captured.
- [ ] F4. Scope fidelity - no changes remove internal SQLite/audit provenance; no unrelated dirty T1/T2 files are reverted.

## Commit strategy
- One logical change per commit. Use Conventional Commit subjects plus the workspace Lore trailers in commit bodies.
- Atomic: every commit builds and passes its task-level tests on its own.
- No "WIP" commits on the final branch.
- Do not stage unrelated dirty files unless the task intentionally edits them.
- Reference the plan file path in the final commit footer: `Plan: .omo/plans/t3-distributable-redaction.md`.

## Success criteria
- Default report, review, generated/static viewer, and package outputs do not disclose full local source paths, temp roots, user/client names embedded in parent directories, or `file://` URLs.
- Explicit local full-path output requires `--include-local-paths --operator <name>` and records the operator in report/review/package/QA metadata.
- Original case SQLite and audit logs retain internal full-path provenance and are not rewritten by redaction.
- Focused tests, full Rust tests, JS syntax check, clippy, fmt, git diff check, and manual grep proof all pass with captured evidence.
