# frametrace-production-hardening-review-plan - Work Plan

## TL;DR (For humans)
**What you'll get:** A hardened execution path that turns FrameTrace from a strong Rust/SQLite forensic prototype into a release-gated Windows workstation candidate without hiding privacy, audit, provenance, large-case, Windows, or corpus gaps.

**Why this approach:** The current review says the core is credible but only partially ready. This plan fixes the high-risk evidence and readiness contracts before GUI or release work, so the product cannot look done while still leaking paths, trusting missing logs, or loading huge cases unsafely.

**What it will NOT do:** It will not claim production/GA/legal readiness early. It will not write to original evidence or let derived outputs land in source paths. It will not build WinUI as a second source of truth or load massive inventory JSON in the browser.

**Effort:** XL
**Risk:** High - the work spans forensic correctness, report defensibility, large-case performance, Windows validation, and a new WinUI shell.
**Decisions to sanity-check:** redaction is default, WinUI waits until engine/Windows gates pass with PASS rather than BLOCKED, full-path disclosure is opt-in only, and release remains fail-closed until real corpus/Windows evidence exists.

Your next move: approve implementation from this plan, or request a separate high-accuracy plan review before execution. Full execution detail follows below.

---

> TL;DR (machine): XL/high-risk staged hardening plan: close privacy/audit/provenance/large-case blockers first, then corpus + Windows + WinUI + installer + release gates.

## Scope
### Must have
- Fix false readiness risk before new feature work: release/report-defense gates must require typed artifacts, not broad text.
- Redact local workstation/source paths from distributable report, viewer, review bundle, and package outputs by default.
- Add explicit opt-in for full-path local/operator mode, with QA evidence and report wording.
- Treat missing required audit chains as blockers when reports, validation claims, derived artifacts, exports, carving, recovery, or package output depend on them.
- Make validation target resolution typed, case-scoped, audit-chain aware, and resistant to poisoned/stale JSONL records.
- Route every ffmpeg-derived operation through the external-tool policy resolver and record tool path/version/args/provenance.
- Remove default full-load report/compatibility paths for large cases; use SQLite bounded queries and streaming artifacts.
- Refactor oversized modules only after behavior is pinned by tests.
- Create and run corpus accuracy/reproducibility validation with source hashes and ground truth.
- Validate Rust engine on Windows before WinUI work.
- Build WinUI 3 shell as a client of Rust engine, SQLite, and audit logs only.
- Add installer/package validation and final release gate artifacts.

### Must NOT have (guardrails, anti-slop, scope boundaries)
- Must not edit original evidence or write derived outputs to source evidence paths.
- Must not claim legal readiness, guaranteed admissibility, GA readiness, or production readiness until release gates pass with artifacts.
- Must not make browser/WinUI load 100k/1M inventory rows as one full JSON payload.
- Must not add a GUI-side durable state store that competes with Rust/SQLite/audit.
- Must not start WinUI implementation before security/privacy/audit/provenance/large-case/corpus/Windows engine gates are PASS. A BLOCKED Windows/corpus gate halts WinUI and release work; it does not authorize a degraded WinUI lane.
- Must not delete schema fields, tables, or outputs without evidence that no current report, test, CLI, or compatibility consumer depends on them.
- Must not weaken existing tests or release blockers to get a green result.
- Must not use manual human QA as pass evidence without machine-readable artifacts.
- Must not treat user approval, operator review, or a human sign-off as verification evidence unless it is represented by a typed artifact and all required machine checks already passed.

## Verification strategy
> Zero human intervention - all verification is agent-executed.
- Test decision: tests-after, with failing regression first inside each todo when current unsafe behavior is known.
- Rust verification: `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, focused `cargo test --locked <module/test>`, and full `cargo test --locked`.
- JavaScript/HTML verification: `node --check gui/evidence-viewer/app.js`, browser/Playwright proof for generated viewer changes when UI behavior changes.
- Performance verification: 100k baseline every large-case change; 1M target profile before release candidate.
- Performance pass/fail thresholds: 100k inventory/report case max RSS <= 1.5 GiB, default inventory query P95 <= 250 ms, search/facet P95 <= 500 ms, report generation <= 120 seconds, compatibility streaming throughput >= 10k rows/sec on the validation host. 1M profile is a release-candidate gate with max RSS <= 3.5 GiB, no browser full JSON load, completion success rate 100%, and every exceeded threshold recorded as a release blocker.
- Windows verification: PowerShell validation script on Windows 10/11 x64 with MSVC, dotnet, ffmpeg/ffprobe, libewf, and Sleuth Kit discovery.
- Windows execution environment: preferred runner is GitHub Actions `windows-latest` plus a local Windows 11 x64 VM for installer/WinUI manual-surface checks. If neither runner is available, T13-T17 are BLOCKED with a `missing-windows-runner` evidence receipt.
- Evidence root: `.omo/evidence/frametrace-production-hardening-review-plan/`.
- Every todo writes a command transcript or JSON receipt under `.omo/evidence/frametrace-production-hardening-review-plan/task-XX-*`.
- Release decision artifact: `reports/qa/release-decision.json` is the single release decision source. Required PASS keys: `typed_review_manifest`, `privacy_review`, `report_defense`, `audit_chain_required_logs`, `tool_policy`, `large_case_100k`, `large_case_1m_profile`, `accuracy`, `reproducibility`, `windows_prerequisites`, `winui_build_test`, `winui_manual_surface`, `installer_package`, `supply_chain_manifest`, `support_incident_docs`, `legal_wording_lint`, `known_limitations`. Any missing, FAIL, BLOCKED, PARTIAL, or UNSUPPORTED key makes the decision `NO_GO`.
- Corpus fixture contract: committed synthetic/non-client fixtures live under `tests/fixtures/corpus/`; large or binary-heavy non-client corpora are referenced by hash in `corpus/manifest/*.json` and stored outside git. Ground-truth schema fields are `corpus_id`, `source_artifact_id`, `source_sha256`, `expected_artifact_type`, `expected_path_pattern`, `expected_hash`, `expected_timestamp_range`, `expected_state`, `negative_controls`, and `notes`.
- Windows dependency policy: default to discovery with approved path/hash/version receipts for ffmpeg/ffprobe, libewf, Sleuth Kit, and dotnet. Bundling is allowed only when license, checksum, SBOM entry, and update policy are committed. Authenticode signing is required for GA only when a signing certificate is available through a secure CI secret; absent certificate records `signing-blocked` and prevents GA but not local lab validation.

## Execution strategy
### Parallel execution waves
> Target 5-8 todos per wave. Fewer than 3 (except the final) means you under-split.
- Wave 0 baseline and gate integrity: T1-T2.
- Wave 1 security/evidence contract hardening: T3-T7. T3-T7 may run in parallel after T1-T2 if write scopes are separated.
- Wave 2 large-case and maintainability: T8-T11. T11 starts only after T8-T10 behavior locks are green.
- Wave 3 corpus and cross-platform readiness: T12-T14. T13 starts after T12 PASS so Windows validation runs against the hardened corpus/release contract.
- Wave 4 WinUI and packaging: T15-T16. Both depend on T1-T14.
- Wave 5 release readiness: T17. Depends on everything.

### Dependency matrix
| Todo | Depends on | Blocks | Can parallelize with |
| --- | --- | --- | --- |
| T1 baseline snapshot and blocker fixtures | none | all todos | none |
| T2 typed readiness/review-gate evidence | T1 | T17 | T3-T7 after shared fixture agreement |
| T3 default path redaction | T1-T2 | T17 | T4, T5, T6, T7 |
| T4 audit-chain missing blocker | T1-T2 | T17 | T3, T5, T6, T7 |
| T5 provenance-safe validation target | T1-T2 | T17 | T3, T4, T6, T7 |
| T6 ffmpeg external-tool policy | T1-T2 | T15-T17 | T3, T4, T5, T7 |
| T7 privacy/report-defense QA surfaces | T3-T6 | T17 | none |
| T8 bounded report generation | T1-T7 | T10, T17 | T9 |
| T9 streaming compatibility exports | T1-T7 | T10, T17 | T8 |
| T10 large-case performance proof | T8-T9 | T15-T17 | none |
| T11 oversized module split | T1-T10 | T15-T17 | none |
| T12 validation corpus and metrics | T1-T7 | T13-T17 | none |
| T13 Windows engine validation | T1-T12 | T14-T17 | none |
| T14 Windows dependency/package preflight | T13 | T16-T17 | none |
| T15 WinUI 3 production shell | T1-T14 | T16-T17 | none |
| T16 installer/package distribution | T15 | T17 | none |
| T17 full release readiness run | T1-T16 | release decision | none |

## Todos
> Implementation + Test = ONE todo. Never separate.
<!-- APPEND TASK BATCHES BELOW THIS LINE WITH edit/apply_patch - never rewrite the headers above. -->
- [x] T1. Create a clean baseline snapshot and regression fixture set
  What to do / Must NOT do: Capture current HEAD, dirty worktree classification, current pass/fail commands, and minimal cases that reproduce each review blocker. Add or confirm a lightweight verification helper at `scripts/qa/verify-plan-evidence.py` that checks todo evidence directories and required receipt names. Do not modify product behavior in this todo.
  Parallelization: Wave 0 | Blocked by: none | Blocks: all todos
  References (executor has NO interview context - be exhaustive): `.omo/ulw-loop/frame-review-progress-20260624/evidence/code-review-scan.md`; `.omo/ulw-loop/frame-review-progress-20260624/evidence/progress-readiness-audit.md`; `.omo/ulw-loop/frame-review-progress-20260624/evidence/remaining-steps.md`; `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md`; `docs/MVP_STATUS.md`
  Acceptance criteria (agent-executable): write `.omo/evidence/frametrace-production-hardening-review-plan/task-01-baseline/baseline.md` containing HEAD, status, current command results, blocker fixture list, untouched unrelated dirty paths, and output from `python3 scripts/qa/verify-plan-evidence.py --self-test` if the helper is created.
  QA scenarios (name the exact tool + invocation): happy: run `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo test --locked`, `node --check gui/evidence-viewer/app.js`, `git diff --check`; failure: run empty-case `target/debug/frametrace qa release <tmp-case> --output-dir <tmp-case>/reports/qa-review-audit` and verify it fails closed. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/task-01-baseline/`.
  Commit: Y if the helper script or fixture code is added, otherwise N | `chore(qa): add plan evidence verification helper`

- [x] T2. Harden typed release/readiness review evidence before product fixes
  What to do / Must NOT do: Replace broad textual review-gate acceptance with typed review manifests and artifact-backed values. Keep `qa release` fail-closed when review, privacy, corpus, Windows, WinUI, or performance evidence is missing. Do not let `complete`, `done`, or `x` pass unless mapped from a typed PASS artifact with path, tool, timestamp, reviewer/operator, and cleanup status.
  Parallelization: Wave 0 | Blocked by: T1 | Blocks: T17
  References (executor has NO interview context - be exhaustive): `src/qa_release.rs`; `src/qa_release_gates.rs`; `tests/cli_smoke.rs`; `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md`; `docs/WINUI3_SHELL_CONTRACT.md`; `.omo/ulw-loop/frame-review-progress-20260624/evidence/progress-readiness-audit.md`
  Acceptance criteria (agent-executable): tests reject broad text-only manifests and accept only typed gate entries with artifact paths; release output lists exact blocker keys for missing or malformed artifacts.
  QA scenarios (name the exact tool + invocation): happy: `cargo test --locked cli_smoke release_gate -- --nocapture`; failure: generate malformed review manifest with `status:"done"` and verify `qa release` reports a malformed review gate blocker. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/`.
  Commit: Y | `fix(release-gates): require typed artifact-backed review evidence`

- [x] T3. Add default distributable path redaction across report, review bundle, and viewer
  What to do / Must NOT do: Introduce a shared redaction policy for distributable outputs. Replace full local source paths and file URLs with source IDs, case-relative artifact paths, and redacted labels by default. Add explicit local/operator opt-in for full path display/export and record that opt-in in QA/report metadata. Do not remove internal provenance from SQLite/audit logs.
  Parallelization: Wave 1 | Blocked by: T1-T2 | Blocks: T7, T17
  References (executor has NO interview context - be exhaustive): `src/review_bundle.rs:129-140`; `src/report.rs:255-264`; `src/report.rs:330-354`; `gui/evidence-viewer/app.js:1114-1118`; `src/package.rs`; `src/cli/handlers.rs`; `docs/RECOVERY_BOUNDARIES.md`; `.omo/ulw-loop/frame-review-progress-20260624/evidence/code-review-scan.md`
  Acceptance criteria (agent-executable): generated shared report/review/viewer/package artifacts omit absolute workstation/source paths by default; opt-in mode includes full paths and writes an explicit privacy disclosure artifact.
  QA scenarios (name the exact tool + invocation): happy: run a case under a temp path containing user/client-like names, generate report/review/package, and grep outputs to prove absolute temp path is absent; failure: run opt-in disclosure mode and prove the report marks full path disclosure as local/operator mode. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/`.
  Commit: Y | `fix(privacy): redact local paths from distributable outputs`

- [x] T4. Make missing required audit chains block report-defense
  What to do / Must NOT do: Define required-vs-optional audit chains based on present artifacts and report claims. Missing required logs must fail report-defense; optional unsupported/not-applicable logs must be typed and visible. Do not block on logs unrelated to the current case surface.
  Parallelization: Wave 1 | Blocked by: T1-T2 | Blocks: T7, T17
  References (executor has NO interview context - be exhaustive): `src/qa_report_defense.rs:28-33`; `src/audit.rs:236-246`; `src/report.rs`; `src/artifacts.rs`; `src/carve.rs`; `src/filesystem_recovery.rs`; `.omo/ulw-loop/frame-review-progress-20260624/evidence/code-review-scan.md`
  Acceptance criteria (agent-executable): report-defense has typed states `missing`, `empty`, `valid`, `tampered`, `unsupported`, `not-applicable`; missing required chains fail; unsupported/not-applicable are displayed and do not masquerade as pass.
  QA scenarios (name the exact tool + invocation): happy: run `frametrace qa report-defense <case>` on a case with expected logs and verify PASS; failure: delete/withhold a log for a reported derived artifact and verify FAIL with the exact log key. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/task-04-audit-chain-required/`.
  Commit: Y | `fix(audit): block report defense on missing required chains`

- [x] T5. Make validation target resolution typed, case-scoped, and audit-chain aware
  What to do / Must NOT do: Replace manual JSON string extraction with typed serde parsing. Verify the relevant audit chain before trusting log-derived paths. Require recovered/derived targets to resolve under the case directory by default. Add explicit external-source validation mode for direct paths. Do not silently trust stale JSONL records or arbitrary direct paths.
  Parallelization: Wave 1 | Blocked by: T1-T2 | Blocks: T17
  References (executor has NO interview context - be exhaustive): `src/validation/target.rs:17-33`; `src/validation/target.rs:79-115`; `src/audit.rs`; `src/media_contract.rs`; `tests/media_contract.rs`; `.omo/ulw-loop/frame-review-progress-20260624/evidence/code-review-scan.md`
  Acceptance criteria (agent-executable): poisoned JSONL, stale audit entries, external direct path, and case-contained derived artifact cases are covered by tests; only typed, audited, case-scoped targets pass by default.
  QA scenarios (name the exact tool + invocation): happy: validate a logged derived artifact selector from inside a case; failure: inject malformed/poisoned JSONL and verify selector resolution rejects it with provenance/audit error. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/task-05-validation-targets/`.
  Commit: Y | `fix(validation): require typed audited validation targets`

- [x] T6. Route all ffmpeg execution through external tool policy
  What to do / Must NOT do: Centralize ffmpeg/ffprobe discovery through the existing external-tool policy resolver. Record resolved binary path, version, command args, operator, source artifact, output artifact, output hash, and audit entry hash for export/proxy/thumbnail/frame operations. Do not call `Command::new("ffmpeg")` directly.
  Parallelization: Wave 1 | Blocked by: T1-T2 | Blocks: T15-T17
  References (executor has NO interview context - be exhaustive): `src/video_export.rs:100-109`; `src/artifacts.rs:102-106`; `src/artifacts.rs:164-168`; `src/artifacts.rs:226-230`; `src/tool_policy.rs`; `src/audit.rs`; `docs/WINDOWS_IMPLEMENTATION_HANDOFF.md`; `.omo/ulw-loop/frame-review-progress-20260624/evidence/code-review-scan.md`
  Acceptance criteria (agent-executable): tests prove approved ffmpeg path passes, rejected path/name fails, and derived logs include resolved tool path/version/args.
  QA scenarios (name the exact tool + invocation): happy: run proxy, thumbnail, frame, and clip export with an approved tool policy and inspect logs; failure: configure a disallowed fake ffmpeg path and verify commands fail before output creation. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy-ffmpeg/`.
  Commit: Y | `fix(tools): enforce ffmpeg provenance policy`

- [x] T7. Add executable privacy and report-defense QA surfaces
  What to do / Must NOT do: Add or harden `qa privacy-review` and report-defense output so redaction, disclosure mode, audit-chain status, failed/skipped/partial/unsupported/not-applicable states, and banned wording checks produce machine-readable artifacts. Do not rely on manual markdown inspection as the only pass/fail source.
  Parallelization: Wave 1 final | Blocked by: T3-T6 | Blocks: T17
  References (executor has NO interview context - be exhaustive): `src/qa_report_defense.rs`; `src/qa_release.rs`; `src/report.rs`; `docs/security-review.md`; `docs/FORENSIC_HARDENING_PLAN.md`; `.omo/ulw-loop/frame-review-progress-20260624/evidence/remaining-steps.md`
  Acceptance criteria (agent-executable): `reports/qa/privacy-review.json`, `reports/qa/report-defense-checklist.md`, and typed JSON release inputs are generated and used by `qa release`.
  QA scenarios (name the exact tool + invocation): happy: full report-defense/privacy check passes on a valid case; failure: banned wording or full-path leakage makes the QA command fail with exact finding key. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/task-07-privacy-report-qa/`.
  Commit: Y | `feat(qa): add executable privacy and report defense evidence`

- [x] T8. Move report generation to bounded SQLite-backed summaries
  What to do / Must NOT do: Stop default `make-report` from reading all `db/video_index.json` and mapping all videos into a single HTML table. Use SQLite summaries, bounded top-N appendices, aggregate counts, and explicit links to bounded review/inventory commands. Do not break small-case reports.
  Parallelization: Wave 2 | Blocked by: T1-T7 | Blocks: T10, T17
  References (executor has NO interview context - be exhaustive): `src/cli/handlers.rs:307-314`; `src/report.rs:306-323`; `src/case_db`; `src/review_bundle.rs:110-119`; `src/workstation.rs:97-104`; `docs/GUI_LARGE_INVENTORY_EXECUTION_PLAN.md`; `.omo/ulw-loop/frame-review-progress-20260624/evidence/code-review-scan.md`
  Acceptance criteria (agent-executable): report generation uses bounded SQLite query contracts by default; large-case output documents truncation/appendix boundaries; small-case golden assertions still pass.
  QA scenarios (name the exact tool + invocation): happy: generate report on 100k synthetic case without reading full JSON; failure: force missing SQLite or legacy JSON-only case and verify report returns a bounded compatibility error or explicit migration guidance. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/task-08-bounded-report/`.
  Commit: Y | `fix(report): use bounded sqlite report summaries`

- [x] T9. Stream JSONL/TSV compatibility outputs instead of full in-memory strings
  What to do / Must NOT do: Rewrite scan compatibility output creation to stream JSONL/TSV rows and avoid constructing large strings in memory. Keep JSON compatibility either bounded, explicit, or documented as legacy export-only. Do not remove compatibility files without migration notes and tests.
  Parallelization: Wave 2 | Blocked by: T1-T7 | Blocks: T10, T17
  References (executor has NO interview context - be exhaustive): `src/scan.rs:248-264`; `src/util.rs`; `src/case_db/scan.rs`; `docs/MVP_STATUS.md`; `.omo/ulw-loop/frame-review-progress-20260624/evidence/code-review-scan.md`
  Acceptance criteria (agent-executable): JSONL/TSV write paths stream row-by-row; tests confirm no duplicate/missing rows after repeated scan; compatibility JSON behavior is explicit and release-safe.
  QA scenarios (name the exact tool + invocation): happy: scan a generated multi-thousand file fixture and compare SQLite count, JSONL count, and TSV count; failure: simulate write failure mid-stream and verify no false success/report-ready state is emitted. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/task-09-stream-compat-outputs/`.
  Commit: Y | `fix(scan): stream compatibility index outputs`

- [x] T10. Prove large-case survival with 100k and 1M evidence profiles
  What to do / Must NOT do: Extend performance QA to cover report generation, inventory list/search/facet/sort, review bundle generation, and compatibility exports. Record max RSS, throughput, query timings, and full-json-load denial evidence. Do not treat browser mock 10k rows as production evidence.
  Parallelization: Wave 2 | Blocked by: T8-T9 | Blocks: T15-T17
  References (executor has NO interview context - be exhaustive): `src/workstation.rs:97-104`; `src/qa_performance.rs`; `docs/PERFORMANCE_VALIDATION.md`; `docs/gui-large-inventory-baseline.md`; `.omo/ulw-loop/frame-gui-20260617102845/evidence/performance-report-100k.json`; `.omo/ulw-loop/frame-gui-20260617102845/evidence/performance-report-1m.json`
  Acceptance criteria (agent-executable): 100k passes on every local verification; 1M profile exists before release candidate with pass/fail metrics for memory, throughput, and query latency.
  QA scenarios (name the exact tool + invocation): happy: `frametrace qa performance <out> --rows 100000` plus report/review generation checks; failure: intentionally request full JSON browser load path and verify it is blocked or marked unsupported. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/task-10-large-case-performance/`.
  Commit: Y | `test(performance): cover large-case report and inventory paths`

- [x] T11. Split oversized modules after behavior is pinned
  What to do / Must NOT do: Refactor `src/cli/handlers.rs`, `src/scan.rs`, `src/html_report.rs`, `src/report.rs`, `src/artifacts.rs`, `src/qa_report_defense.rs`, `src/qa_release.rs`, `src/qa_tests.rs`, `tests/cli_lifecycle.rs`, and `tests/cli_windows_prereq.rs` by responsibility after T3-T10 tests are green. Keep public command behavior unchanged. Do not mix new behavior into refactor commits.
  Parallelization: Wave 2 final | Blocked by: T1-T10 | Blocks: T15-T17
  References (executor has NO interview context - be exhaustive): `.omo/ulw-loop/frame-review-progress-20260624/evidence/code-review-scan.md`; `.omo/evidence/frametrace-production-hardening-review-plan/task-07-privacy-report-qa/t11-oversized-file-disposition.md`; `src/cli/handlers.rs`; `src/scan.rs`; `src/html_report.rs`; `src/report.rs`; `src/artifacts.rs`; `src/qa_report_defense.rs`; `src/qa_release.rs`; `src/qa_tests.rs`; `tests/cli_lifecycle.rs`; `tests/cli_windows_prereq.rs`
  Acceptance criteria (agent-executable): behavior tests pass unchanged; module-size report explains remaining large files or marks `SIZE_OK` with rationale; no command output drift except intentional safe text from earlier todos.
  QA scenarios (name the exact tool + invocation): happy: full fmt/clippy/tests plus selected CLI smoke flows before and after refactor; failure: diff behavior snapshots and reject refactor if reports/viewers/JSON contracts drift unexpectedly. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/task-11-module-split/`.
  Commit: Y | `refactor(core): split oversized modules by responsibility`

- [x] T12. Create real validation corpus manifests and accuracy/reproducibility metrics
  What to do / Must NOT do: Prepare non-client validation corpora with source hashes, ground truth, and expected outputs for deleted recovery, browser/video artifacts, event logs/timeline where supported, large evidence, and mixed real-world-like cases. Put lightweight fixtures under `tests/fixtures/corpus/`, manifests under `corpus/manifest/`, and large external corpora behind hash-only references. Mark unsupported domains as unsupported, not pass. Do not include private/client evidence in committed fixtures.
  Parallelization: Wave 3 | Blocked by: T1-T7 | Blocks: T17
  References (executor has NO interview context - be exhaustive): `docs/validation-corpus.md`; `docs/recovery-prd.md`; `docs/recovery-test-spec.md`; `src/qa_accuracy.rs`; `src/qa_reproducibility.rs`; `.omo/ulw-loop/frame-review-progress-20260624/evidence/progress-readiness-audit.md`
  Acceptance criteria (agent-executable): corpus manifest includes hashes and ground truth schema fields from Verification strategy; accuracy report records precision, recall, false positives, false negatives; reproducibility report compares two case outputs with allowed diff thresholds; synthetic-only corpus cannot satisfy the `mixed_real_world_like` release key.
  QA scenarios (name the exact tool + invocation): happy: `target/debug/frametrace qa accuracy <case> corpus/manifest/synthetic-video-corpus.json` and `target/debug/frametrace qa reproducibility <case-a> <case-b>` pass on committed synthetic/non-client fixtures; failure: alter one expected hash in a copied manifest and verify accuracy exits non-zero with a false-negative or mismatch key. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/task-12-corpus-metrics/`.
  Commit: Y | `feat(qa): add corpus accuracy and reproducibility evidence`

- [ ] T13. Validate Rust engine on Windows 10/11 x64 before GUI work
  What to do / Must NOT do: Run or update Windows validation script so Rust MSVC build, tests, ffmpeg/ffprobe, libewf, Sleuth Kit discovery, Unicode/long path, repeated scan, synthetic MP4, E01/raw workflows, file locks, and workstation-status prerequisites are verified. Do not start WinUI if this gate is red without a named blocker.
  Parallelization: Wave 3 | Blocked by: T1-T12 | Blocks: T14-T17
  References (executor has NO interview context - be exhaustive): `scripts/windows/validate-release.ps1`; `docs/WINDOWS_IMPLEMENTATION_HANDOFF.md`; `docs/WINDOWS_VALIDATION.md`; `docs/WINDOWS_RISK_REVIEW.md`; `src/workstation.rs`; `.github/workflows/windows-ci.yml`
  Acceptance criteria (agent-executable): Windows transcript and `reports/qa/windows-prerequisites.json` prove host OS, tool discovery, build/test, synthetic workflow, and status gate; if no Windows host is available, write `.omo/evidence/frametrace-production-hardening-review-plan/task-13-windows-engine/BLOCKED-missing-windows-runner.json` and stop T14-T17.
  QA scenarios (name the exact tool + invocation): happy: on Windows run `powershell -ExecutionPolicy Bypass -File scripts/windows/validate-release.ps1 -CaseRoot C:\\Temp\\frametrace-release-case`; failure: temporarily remove `dotnet.exe` from PATH or omit `reports\\qa\\winui-build.json` and verify `target\\release\\frametrace.exe qa release C:\\Temp\\frametrace-release-case --output-dir C:\\Temp\\frametrace-release-case\\reports\\qa` exits non-zero with `windows_prerequisites`. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/task-13-windows-engine/`.
  Commit: Y | `test(windows): enforce engine validation preflight`

- [ ] T14. Lock Windows dependency and package preflight contracts
  What to do / Must NOT do: Implement the default discovery policy from Verification strategy for ffmpeg/ffprobe, libewf, Sleuth Kit, .NET runtime, and WinUI dependencies. Add checksum/SBOM/license/package manifest expectations. Do not bundle third-party tools without license, hash, and update-policy evidence.
  Parallelization: Wave 3 final | Blocked by: T13 | Blocks: T16-T17
  References (executor has NO interview context - be exhaustive): `docs/TECH_STACK.md`; `docs/WINDOWS_IMPLEMENTATION_HANDOFF.md`; `docs/WINDOWS_USAGE.md`; `src/tool_policy.rs`; `scripts/windows/validate-release.ps1`
  Acceptance criteria (agent-executable): package/dependency preflight emits required tools, versions, resolved paths, policy decision, and missing/unsupported blockers; docs match the executable checks.
  QA scenarios (name the exact tool + invocation): happy: run `target\\release\\frametrace.exe workstation-status C:\\Temp\\frametrace-release-case > reports\\qa\\workstation-status.json` and verify dependency receipts list approved resolved tools; failure: point the tool policy to an unapproved ffmpeg path and verify derived artifact commands fail before output generation. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/task-14-windows-dependency-preflight/`.
  Commit: Y | `feat(windows): add dependency package preflight contract`

- [ ] T15. Implement WinUI 3 shell as a client-only workstation
  What to do / Must NOT do: Create `gui/winui` solution and tests. Implement case open/create, source intake, bounded inventory list/search/filter/sort, details, validation/progress, audit-chain display, report/package actions, and Korean-first dense examiner workflow. Required screens: Case Home, Evidence Source Intake, Inventory, Artifact Detail, Jobs/Progress, Audit Chain, Report/Package, Settings/Dependency Status. Required Korean UI assertions: primary navigation labels are Korean-first, inventory columns include 파일명/상태/출처/크기/시간/검증, blocker states show 실패/차단/부분/지원안함 distinctly. Call Rust engine commands and read SQLite/status/audit outputs; do not create a competing GUI-owned durable state.
  Parallelization: Wave 4 | Blocked by: T1-T14 | Blocks: T16-T17
  References (executor has NO interview context - be exhaustive): `docs/WINUI3_SHELL_CONTRACT.md`; `docs/EVIDENCE_VIEWER_GUI.md`; `docs/GUI_LARGE_INVENTORY_EXECUTION_PLAN.md`; `gui/evidence-viewer/index.html`; `gui/evidence-viewer/app.js`; `src/workstation.rs`; `src/cli/handlers.rs`
  Acceptance criteria (agent-executable): `dotnet build` and `dotnet test` pass; UI actions produce engine/audit artifacts; 100k inventory remains paged; `reports/qa/winui-build.json`, `reports/qa/winui-action-log.jsonl`, and screenshots for each required screen exist; action log proves every durable mutation invoked an engine command.
  QA scenarios (name the exact tool + invocation): happy: run `dotnet build gui\\winui\\FrameTrace.sln -c Release`, `dotnet test gui\\winui\\FrameTrace.Tests\\FrameTrace.Tests.csproj -c Release`, then execute the WinUI smoke automation or manual-surface driver to open synthetic case, browse/search inventory, validate artifact, view audit chain, generate report/package; failure: configure an engine command to return non-zero and verify UI displays blocked/failed state while `reports/qa/winui-action-log.jsonl` shows no GUI-owned durable mutation. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/task-15-winui-shell/`.
  Commit: Y | `feat(winui): add engine-backed forensic workstation shell`

- [ ] T16. Add installer, package, signing/checksum, and clean-VM validation
  What to do / Must NOT do: Build installer/package flow for Rust binary, WinUI shell, docs, dependency discovery, manifests, checksums, SBOM/license notes, uninstall/reinstall, and clean Windows VM validation. Implement signing only if a secure CI signing certificate is available; otherwise emit `signing-blocked` and prevent GA. Do not mark installer PASS without build/test receipts and dependency policy evidence.
  Parallelization: Wave 4 final | Blocked by: T14-T15 | Blocks: T17
  References (executor has NO interview context - be exhaustive): `docs/WINDOWS_USAGE.md`; `docs/WINDOWS_IMPLEMENTATION_HANDOFF.md`; `scripts/windows/validate-release.ps1`; `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md`
  Acceptance criteria (agent-executable): installer/package manifest includes hashes; clean VM install/uninstall/reinstall succeeds; `frametrace.exe --help`, WinUI launch, and smoke case flow work from installed location.
  QA scenarios (name the exact tool + invocation): happy: on clean Windows VM run `powershell -ExecutionPolicy Bypass -File scripts\\windows\\validate-package.ps1 -InstallerPath <installer> -CaseRoot C:\\Temp\\frametrace-installed-case` and verify `reports/qa/installer-package-validation.json`; failure: tamper package checksum and verify validation fails before install/use. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/task-16-installer-package/`.
  Commit: Y | `feat(release): add windows installer package validation`

- [ ] T17. Run full release readiness and external review gates
  What to do / Must NOT do: Run final `qa release` with review manifest, corpus manifest, comparison case, performance output, Windows prerequisite receipt, WinUI build receipt, privacy/report-defense evidence, package receipt, support/incident/known-limitations docs, and legal wording scan. Do not claim readiness if any blocker fails.
  Parallelization: Wave 5 | Blocked by: T1-T16 | Blocks: release decision
  References (executor has NO interview context - be exhaustive): `src/qa_release.rs`; `src/qa_release_gates.rs`; `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md`; `docs/FORENSIC_HARDENING_PLAN.md`; `docs/security-review.md`; `docs/validation-corpus.md`; `.omo/ulw-loop/frame-review-progress-20260624/evidence/final-quality-gate.json`
  Acceptance criteria (agent-executable): `reports/qa/release-readiness.json`, `reports/qa/release-readiness.md`, and `reports/qa/release-decision.json` show every required release decision key PASS with artifact evidence; failed/skipped/partial/unsupported are visible; no disallowed legal/readiness wording exists in generated output; any missing key sets `release-decision.json.decision` to `NO_GO`.
  QA scenarios (name the exact tool + invocation): happy: `target/debug/frametrace qa release <case> --corpus-manifest <manifest> --comparison-case <case-b> --review-manifest <manifest> --performance-output-dir <out> --performance-rows 100000` passes and writes `reports/qa/release-decision.json` with `decision: GO`; failure: remove `reports/qa/privacy-review.json` or `reports/qa/winui-build.json` and verify release exits non-zero with exact blocker and `decision: NO_GO`. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/task-17-release-readiness/`.
  Commit: Y | `chore(release): close report-defensible readiness gate`

## Final verification wave
> Runs in parallel after ALL todos. ALL verifier checks must PASS. User approval after this wave is a handoff/release decision, not verification evidence.
- [ ] F1. Plan compliance audit
  Verify every T1-T17 acceptance criterion has an evidence artifact and no todo was skipped without a blocker report. Command: `python3 scripts/qa/verify-plan-evidence.py .omo/plans/frametrace-production-hardening-review-plan.md .omo/evidence/frametrace-production-hardening-review-plan`. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/final/F1-plan-compliance.md`.
- [ ] F2. Code quality and maintainability review
  Review diff for behavior drift, oversized-module regressions, unsupported abstractions, weakened tests, and source-of-truth violations. Commands: `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo test --locked`, `node --check gui/evidence-viewer/app.js`, module-size script from T11. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/final/F2-code-quality.md`.
- [ ] F3. Real surface QA
  Drive CLI, generated HTML/report, Windows script, WinUI shell, installer/package, and release command through happy and failure paths. Commands: CLI smoke script from T1, Playwright/browser check for generated HTML, `scripts/windows/validate-release.ps1`, WinUI smoke driver from T15, `scripts/windows/validate-package.ps1`, final `qa release`. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/final/F3-real-surface-qa/`.
- [ ] F4. Security/privacy/legal wording review
  Verify no path leakage, no original evidence writes, no disallowed legal/readiness claims, no missing audit chain pass, no unapproved external tool execution. Commands: `frametrace qa privacy-review`, `frametrace qa report-defense`, legal wording lint from T7, symlink/source-output policy tests, tool-policy tests. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/final/F4-security-privacy-legal.md`.
- [ ] F5. Performance/reproducibility review
  Re-run 100k large-case checks, validate 1M profile artifact, and compare reproducibility outputs. Commands: `frametrace qa performance <out> --rows 100000`, 1M profile command from T10, `frametrace qa reproducibility <case-a> <case-b>`. Evidence `.omo/evidence/frametrace-production-hardening-review-plan/final/F5-performance-reproducibility.md`.

## Commit strategy
- Prefer one commit per todo when the todo changes product behavior, plus evidence-only commits only if repo policy intentionally tracks `.omo` evidence.
- Use Lore commit protocol from AGENTS.md for every commit:
  - intent line explains why the change was made
  - include `Constraint`, `Rejected`, `Confidence`, `Scope-risk`, `Directive`, `Tested`, and `Not-tested` trailers when they add value.
- Commit order must follow dependency order: T2 before T3-T7; T8-T10 before T11; T13 before T15; T16 before T17.
- No squash that erases blocker-to-fix traceability before release review.
- If Windows/WinUI work must happen on a Windows machine, commit macOS-safe Rust/docs/test changes separately from Windows shell/package commits.

## Success criteria
- Current high findings are closed with tests: path leakage, missing-audit report-defense, large-case full-load report/scan paths.
- Current medium findings are closed with tests: validation target trust, direct ffmpeg execution, oversized module risk.
- `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo test --locked`, `node --check gui/evidence-viewer/app.js`, and `git diff --check` pass after every behavior/refactor wave.
- `qa privacy-review`, `qa report-defense`, `qa accuracy`, `qa reproducibility`, `qa performance`, and `qa release` produce machine-readable evidence and fail closed on missing/malformed inputs.
- 100k large-case checks pass; 1M profile has recorded pass/fail metrics and blocker handling.
- Windows 10/11 x64 validation has build/test/workflow transcripts and `windows_prerequisites` receipt.
- WinUI 3 project builds/tests and proves it uses engine/SQLite/audit as source of truth.
- Installer/package validation passes on a clean Windows VM or records a hard blocker.
- Final release output uses `report-defensible` language, shows failed/skipped/partial/unsupported states, and never claims legal proof or guaranteed admissibility.
