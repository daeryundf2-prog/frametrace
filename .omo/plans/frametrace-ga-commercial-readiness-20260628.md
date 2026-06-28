# frametrace-ga-commercial-readiness-20260628 - Work Plan

## TL;DR (For humans)
**What you'll get:** A step-by-step path from the current blocked local prototype/engine state to field-pilot, then GA/commercial readiness. It tells the Windows operator exactly what to run first and keeps release claims evidence-bound.

**Why this approach:** The next real blocker is Windows engine validation, so WinUI, packaging, clean-VM, field-pilot, and GA work must stay downstream. The plan keeps Git clean by committing source, docs, scripts, tests, and durable plans while regenerating local evidence on the machine that runs each gate.

**What it will NOT do:** It will not claim GA, legal admissibility, certification, or integrity guarantees. It will not ask Windows operators to sort through stale local browser/session artifacts. It will not start WinUI/package work before Windows engine validation passes.

**Effort:** XL
**Risk:** High - real Windows validation, desktop shell, packaging, corpus evidence, support operations, and commercial/legal governance all remain release-gated.
**Decisions to sanity-check:** MSIX as primary package, unsigned ZIP as lab-only fallback, field-pilot before GA, and `.omo/evidence` excluded from clean Git commits.

Your next move: run T1 on a real Windows 10/11 x64 MSVC host, then continue in dependency order. Full execution detail follows below.

---

> TL;DR (machine): XL/high-risk Windows-first field-pilot-to-GA plan; T1 Windows engine validation blocks WinUI, package, clean VM, corpus, field-pilot, and commercial GA claims.

## Scope
### Must have
- Preserve the current local-first forensic workstation positioning.
- Keep `FIELD_PILOT_GO`, `NO_GO`, and `BLOCKED` as release decision states until a future GA gate is explicitly implemented.
- Run the first continuing gate on a real Windows 10/11 x64 MSVC host, not macOS emulation.
- Validate Rust engine behavior on Windows before WinUI, package, clean VM, or field-pilot gates.
- Implement WinUI 3 as an engine-command-only shell that reads Rust/SQLite/audit state.
- Package with MSIX as the primary commercial path; unsigned ZIP is lab-only and cannot produce field-pilot or GA GO.
- Validate on a clean Windows VM with no development checkout.
- Separate synthetic, lab, mixed real-world-like, and external practitioner corpus evidence.
- Add commercial operations gates: support triage, incident response, hotfix, rollback, logging/privacy, signing, SBOM, license/EULA, offline activation, and release governance.
- Keep Git clean: commit source, scripts, tests, docs, and durable plans; do not commit local `.omo/evidence` browser/session output.

### Must NOT have (guardrails, anti-slop, scope boundaries)
- Do not claim `production-ready`, `court-certified`, `forensically proven`, `guaranteed integrity`, `automated court-grade verification`, `fully verified`, or `legally admissible`.
- Do not bypass T1 Windows engine validation.
- Do not let WinUI write durable forensic state directly.
- Do not treat browser/static GUI QA as Windows desktop readiness.
- Do not commit local browser profiles, caches, generated case DBs, screenshots, server logs, or scratch QA output.
- Do not remove or revert unrelated dirty worktree content.

## Verification strategy
> Zero human intervention - all verification is agent-executed.
- Test decision: TDD for Rust release gates/scripts and tests-after/manual QA for Windows desktop/package surfaces.
- Evidence root: `.omo/evidence/frametrace-ga-commercial-readiness-20260628/`.
- Core local checks before any commit: `cargo fmt --all -- --check`, `cargo test --locked`, `node --check gui/evidence-viewer/app.js`, `node --check gui/evidence-viewer/translations.js`, `python3 scripts/qa/verify-plan-evidence.py .omo/plans/frametrace-overall-completion-uplift-20260627.md .omo/evidence/frametrace-overall-completion-uplift-20260627`, `git diff --check`.
- Windows checks: `scripts\windows\validate-engine.ps1`, WinUI build/test/smoke script, package build/validate scripts, clean-VM install validation, final `qa release`.
- Real-surface proof: PowerShell transcripts for Windows CLI gates, computer-use/UI automation screenshots/action logs for WinUI, package install receipts for clean VM, corpus manifests plus CLI outputs for field-pilot gates.

## Execution strategy
### Parallel execution waves
> Target 5-8 todos per wave. Fewer than 3 (except the final) means you under-split.
- Wave 0, repo cleanup and continuation prep: T0.
- Wave 1, Windows engine gate: T1 only; it is the hard critical path.
- Wave 2, post-Windows release reset and wording policy: T2-T3 after T1 PASS.
- Wave 3, desktop shell and package implementation: T4-T8 after T1/T2 PASS.
- Wave 4, clean VM and corpus field-pilot: T9-T12 after T4-T8 PASS.
- Wave 5, commercial/GA operations: T13-T18 after field-pilot evidence is stable.
- Wave 6, final decision and release evidence: T19-T21 plus F1-F4.

### Dependency matrix
| Todo | Depends on | Blocks | Can parallelize with |
| --- | --- | --- | --- |
| T0 | none | T1-T21 | none |
| T1 | T0 | T2-T21 | none |
| T2 | T1 | T5-T8, T19 | T3 |
| T3 | T1 | T13-T18 | T2 |
| T4 | T2 | T5-T12 | none |
| T5 | T4 | T6, T9, T19 | T7, T8 |
| T6 | T5 | T9, T19 | T7, T8 |
| T7 | T4 | T8, T10, T19 | T5 |
| T8 | T4, T7 | T10, T19 | T5, T6 |
| T9 | T5, T6, T7, T8 | T11, T12, T19 | T10 |
| T10 | T7, T8 | T11, T19 | T9 |
| T11 | T9, T10 | T12, T19 | none |
| T12 | T11 | T13-T18 | none |
| T13 | T12 | T19 | T14, T15, T16, T17, T18 |
| T14 | T12 | T19 | T13, T15, T16, T17, T18 |
| T15 | T12 | T19 | T13, T14, T16, T17, T18 |
| T16 | T12 | T19 | T13, T14, T15, T17, T18 |
| T17 | T12 | T19 | T13, T14, T15, T16, T18 |
| T18 | T12 | T19 | T13, T14, T15, T16, T17 |
| T19 | T13-T18 | T20, T21 | none |
| T20 | T19 | T21 | none |
| T21 | T20 | final decision | none |

## Todos
> Implementation + Test = ONE todo. Never separate.
<!-- APPEND TASK BATCHES BELOW THIS LINE WITH edit/apply_patch - never rewrite the headers above. -->
- [ ] T0. Clone hygiene and branch baseline
  What to do / Must NOT do: Confirm branch, remote, current blocker state, and `.gitignore` excludes disposable evidence. Do not stage local `.omo/evidence`, browser profiles, generated DBs, screenshots, or logs.
  Parallelization: Wave 0 | Blocked by: none | Blocks: T1-T21
  References: `.gitignore`; `.omo/boulder.json`; `.omo/start-work/ledger.jsonl`; `docs/GA_COMMERCIAL_READINESS_PLAN.md`.
  Acceptance criteria: `git status --short --ignored` shows disposable evidence ignored or unstaged; `git diff --cached --stat` includes only selected source/docs/scripts/tests/plans.
  QA scenarios: happy: `git status --short --ignored | rg '(^!! .omo/evidence|^!! .omo/ulw-loop/.*/evidence)'`; failure: create a temporary `.omo/evidence/git-hygiene-scratch/browser-profile/Cache/file.tmp` and assert it is ignored, then remove the scratch dir. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t0-git-hygiene.txt`.
  Commit: Y | `chore(git): keep local QA evidence out of clean clones`

- [ ] T1. Windows engine validation gate
  What to do / Must NOT do: On real Windows 10/11 x64 MSVC, run the engine-only validation script and capture the JSON receipt. Do not start WinUI/package work if this fails or is blocked.
  Parallelization: Wave 1 | Blocked by: T0 | Blocks: T2-T12
  References: `scripts/windows/validate-engine.ps1`; `tests/windows_engine_validation_script.rs`; `docs/WINDOWS_VALIDATION.md`; `.omo/evidence/frametrace-overall-completion-uplift-20260627/task-12-final-field-pilot/release-decision.json`.
  Acceptance criteria: `reports\qa\windows-engine-validation.json` records Windows host, MSVC Rust, ffmpeg/ffprobe, synthetic MP4 workflow, Unicode/long path, locked-file behavior, repeated scan, bounded inventory, and workstation status PASS.
  QA scenarios: happy: `powershell -ExecutionPolicy Bypass -File scripts\windows\validate-engine.ps1 -CaseRoot C:\Temp\frametrace-engine-case -PerformanceRows 100000`; failure: run on non-Windows or remove `ffprobe.exe`, expecting non-zero plus typed blocker receipt. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t1-windows-engine/`.
  Commit: N unless script/test changes are required.

- [ ] T2. Field-pilot release decision reset
  What to do / Must NOT do: After T1 PASS, update release readiness from `BLOCKED` to the next honest state. Do not mark `FIELD_PILOT_GO` until all field-pilot gates pass.
  Parallelization: Wave 0/1 | Blocked by: T1 PASS | Blocks: T5-T21
  References: `src/qa_release.rs`; `src/qa_release_decision.rs`; `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md`; `docs/WINUI3_SHELL_CONTRACT.md`.
  Acceptance criteria: `qa release` still emits `NO_GO` or `BLOCKED` when WinUI/package/corpus gates are missing.
  QA scenarios: happy: `target\release\frametrace.exe qa release C:\Temp\frametrace-engine-case --output-dir C:\Temp\frametrace-engine-case\reports\qa` exits non-zero with exact downstream blockers; failure: remove `windows_engine_validation` receipt and assert the blocker returns. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t2-release-reset/`.
  Commit: Y if release logic/docs change | `fix(release): preserve blocked downstream gates after Windows engine pass`

- [ ] T3. Commercial claim and wording policy
  What to do / Must NOT do: Add/verify wording policy for public docs, UI, reports, release notes, installer copy, EULA, and sales material. Do not soften unsupported claims into near-equivalents.
  Parallelization: Wave 0 | Blocked by: T1 baseline | Blocks: T13-T18
  References: `README.md`; `docs/EVIDENCE_VIEWER_GUI.md`; `src/qa_tests/release_privacy.rs`; `src/qa_report_defense.rs`; `docs/WINDOWS_RISK_REVIEW.md`.
  Acceptance criteria: banned claim scan passes across source/docs/installer UI strings except negative tests.
  QA scenarios: happy: `rg -n 'production-ready|court-certified|forensically proven|guaranteed integrity|automated court-grade verification|fully verified|legally admissible' README.md docs gui src scripts --glob '!target/**'` returns only intentional negative fixtures; failure: inject a temp report fixture with a banned phrase and assert report-defense rejects it. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t3-wording/`.
  Commit: Y | `docs(readiness): lock commercial wording boundaries`

- [ ] T4. WinUI 3 project scaffold and engine adapter
  What to do / Must NOT do: Create `gui/winui` as a C#/WinUI 3 shell that invokes Rust commands or reads bounded JSON. Do not write durable forensic state from C#.
  Parallelization: Wave 2 | Blocked by: T1 PASS | Blocks: T5, T6, T9, T19
  References: `docs/WINUI3_SHELL_CONTRACT.md`; `docs/GUI_DATA_ADAPTER_CONTRACT.md`; `src/workstation.rs`; `src/workstation_contract.rs`.
  Acceptance criteria: `dotnet build gui\winui\FrameTrace.sln -c Release` passes; adapter tests prove command invocation and bounded JSON parsing.
  QA scenarios: happy: `powershell -ExecutionPolicy Bypass -File scripts\windows\smoke-winui.ps1 -CaseRoot C:\Temp\frametrace-winui-case -ScreenshotDir .omo\evidence\frametrace-ga-commercial-readiness-20260628\t4-winui\screenshots`; failure: point adapter to a failing engine path and assert UI displays blocked state without case mutation. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t4-winui/`.
  Commit: Y | `feat(winui): add engine-owned workstation shell`

- [ ] T5. WinUI accessibility, keyboard, and dense review workflow
  What to do / Must NOT do: Implement case open/create, source intake, inventory, preview, inspector, validation, export, report, package, job state, and audit views with keyboard/focus support.
  Parallelization: Wave 2 | Blocked by: T4 | Blocks: T9, T19
  References: `docs/EVIDENCE_VIEWER_GUI.md`; `DESIGN.md`; `gui/evidence-viewer/*`; `docs/WINUI3_SHELL_CONTRACT.md`.
  Acceptance criteria: UI automation covers open case, search/filter, preview, validation queue, export/report draft, and blocked job states.
  QA scenarios: happy: `dotnet test gui\winui\FrameTrace.Tests\FrameTrace.Tests.csproj -c Release --logger trx`; failure: simulate missing `ffmpeg` and assert proxy/export actions are disabled with engine blocker text. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t5-winui-ux/`.
  Commit: Y | `feat(winui): implement forensic review workflows`

- [ ] T6. WinUI build/test receipt gate
  What to do / Must NOT do: Generate `reports\qa\winui-build.json` from actual build/test output. Do not let handwritten receipts satisfy release gates.
  Parallelization: Wave 2 | Blocked by: T4 | Blocks: T9, T19
  References: `src/windows_prerequisites.rs`; `scripts/windows/validate-release.ps1`; `docs/WINUI3_SHELL_CONTRACT.md`.
  Acceptance criteria: `windows_prerequisites` passes only when `.sln`/`.csproj` and `winui-build.json` are present and valid.
  QA scenarios: happy: `powershell -ExecutionPolicy Bypass -File scripts\windows\validate-release.ps1 -CaseRoot C:\Temp\frametrace-winui-build-case -EngineOnly:$false`; failure: delete `reports\qa\winui-build.json` and assert `windows_prerequisites` blocks. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t6-winui-build-receipt/`.
  Commit: Y | `ci(windows): require real WinUI build receipts`

- [ ] T7. Package build system
  What to do / Must NOT do: Implement `scripts/windows/build-package.ps1` for MSIX primary output and unsigned lab ZIP fallback. Do not include signing secrets.
  Parallelization: Wave 2 | Blocked by: T1 PASS | Blocks: T8, T10, T19
  References: `docs/TECH_STACK.md`; `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md`; Microsoft Windows App SDK/MSIX docs at implementation time.
  Acceptance criteria: Package output includes manifest, checksums, SBOM, dependency manifest, and signing status.
  QA scenarios: happy: `powershell -ExecutionPolicy Bypass -File scripts\windows\build-package.ps1 -Configuration Release -OutputDir C:\Temp\frametrace-package-out`; failure: run without certificate config and assert `signing-blocked` keeps release decision non-GO. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t7-package-build/`.
  Commit: Y | `build(windows): add workstation package build`

- [ ] T8. Package validation and tamper checks
  What to do / Must NOT do: Implement `scripts/windows/validate-package.ps1` for signature, timestamp, checksums, SBOM, dependencies, installability, and tamper rejection.
  Parallelization: Wave 2 | Blocked by: T7 | Blocks: T10, T19
  References: `src/package.rs`; `docs/WINDOWS_VALIDATION.md`; Microsoft MSIX signing docs at implementation time.
  Acceptance criteria: Tampered checksum fails before launch; unsigned lab ZIP is recorded as non-commercial.
  QA scenarios: happy: `powershell -ExecutionPolicy Bypass -File scripts\windows\validate-package.ps1 -PackagePath <package> -ReceiptPath C:\Temp\frametrace-package-out\installer-package-validation.json`; failure: alter checksum manifest and expect non-zero. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t8-package-validate/`.
  Commit: Y | `test(windows): validate package provenance`

- [ ] T9. Clean Windows VM install workflow
  What to do / Must NOT do: Validate package on a clean VM without dev checkout. Do not count developer machine as clean evidence.
  Parallelization: Wave 3 | Blocked by: T5, T6, T8 | Blocks: T11, T12, T19
  References: `docs/WINDOWS_VALIDATION.md`; `docs/WINUI3_SHELL_CONTRACT.md`.
  Acceptance criteria: Receipt records OS/build, install method, package path, checksum, launch proof, dependency status, synthetic workflow, uninstall/reinstall, and screenshots.
  QA scenarios: happy: `powershell -ExecutionPolicy Bypass -File scripts\windows\validate-package.ps1 -PackagePath <package> -CaseRoot C:\Temp\frametrace-installed-case -CleanVmReceipt C:\Temp\frametrace-installed-case\reports\qa\clean-vm-package.json -ScreenshotDir C:\Temp\frametrace-installed-case\reports\qa\screenshots`; failure: run with tampered package and assert no launch. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t9-clean-vm/`.
  Commit: Y | `test(windows): prove clean-vm install workflow`

- [ ] T10. Mixed corpus governance
  What to do / Must NOT do: Define approved corpus classes, redaction policy, chain of custody for samples, and unsupported-format boundaries. Do not use private/client evidence in Git.
  Parallelization: Wave 3 | Blocked by: T7, T8 | Blocks: T11, T12, T19
  References: `docs/validation-corpus.md`; `docs/MANUFACTURER_PARSER_RESEARCH.md`; `docs/RECOVERY_BOUNDARIES.md`.
  Acceptance criteria: Corpus manifests distinguish synthetic, lab, mixed real-world-like, external practitioner, and unsupported formats.
  QA scenarios: happy: `cargo run --locked -- qa accuracy <case> <corpus.tsv> --output-dir <out>` with declared corpus; failure: corrupt expected hash and assert accuracy report flags mismatch. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t10-corpus-governance/`.
  Commit: Y | `docs(qa): define commercial corpus governance`

- [ ] T11. Field-pilot QA matrix
  What to do / Must NOT do: Run accuracy, reproducibility, performance, report-defense, generated viewer QA, package validation, and clean VM evidence against declared corpus classes.
  Parallelization: Wave 3 | Blocked by: T9, T10 | Blocks: T12, T19
  References: `src/qa_accuracy/*`; `src/qa_repro.rs`; `src/qa_release.rs`; `docs/PERFORMANCE_VALIDATION.md`.
  Acceptance criteria: Field-pilot matrix passes or records exact blockers; synthetic-only evidence cannot produce field-pilot GO.
  QA scenarios: happy: run `qa accuracy`, `qa reproducibility`, `qa performance --rows 100000`, `qa report-defense`, `make-review`, browser QA, and package validation; failure: remove mixed corpus manifest and assert `field-pilot-blocked: mixed-corpus-unavailable`. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t11-field-pilot-matrix/`.
  Commit: Y | `test(qa): add field-pilot readiness matrix`

- [ ] T12. Field-pilot decision gate
  What to do / Must NOT do: Produce `FIELD_PILOT_GO`, `NO_GO`, or `BLOCKED` from evidence. Do not produce GA GO here.
  Parallelization: Wave 3 | Blocked by: T11 | Blocks: T13-T18
  References: `src/qa_release.rs`; `src/qa_release_decision.rs`; `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md`.
  Acceptance criteria: Release decision includes exact blockers, evidence paths, and scope caveats.
  QA scenarios: happy: `frametrace qa release <case> --review-manifest <release-review.json> --corpus-manifest <corpus.tsv> --comparison-case <case> --performance-output-dir <perf> --output-dir <qa>`; failure: mark package or corpus gate BLOCKED and assert decision is `NO_GO` or `BLOCKED`. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t12-field-pilot-decision/`.
  Commit: Y | `docs(readiness): record field-pilot decision`

- [ ] T13. Security and privacy commercial gate
  What to do / Must NOT do: Add security/privacy review checklist for local evidence handling, path redaction, logs, crash dumps, temp files, update checks, telemetry absence/policy, and support bundles.
  Parallelization: Wave 4 | Blocked by: T12 | Blocks: T19
  References: `src/qa_tests/release_privacy.rs`; `docs/WINDOWS_RISK_REVIEW.md`; `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md`.
  Acceptance criteria: Support/debug bundles redact sensitive paths unless operator opts in.
  QA scenarios: happy: run privacy/report-defense tests plus support bundle redaction scenario; failure: inject absolute private path into report/support output and assert gate rejects. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t13-security-privacy/`.
  Commit: Y | `test(privacy): gate commercial support evidence`

- [ ] T14. Supply-chain, signing, and SBOM gate
  What to do / Must NOT do: Add dependency inventory, SBOM, license notices, signing certificate policy, timestamping, and reproducible release record. Do not store signing secrets.
  Parallelization: Wave 4 | Blocked by: T12 | Blocks: T19
  References: `Cargo.toml`; Windows package scripts from T7-T8; Microsoft MSIX signing guidance at implementation time.
  Acceptance criteria: Release package has SBOM/checksums/signing receipt; unsigned builds cannot produce GA.
  QA scenarios: happy: package validation verifies signed MSIX and SBOM; failure: missing timestamp/signature produces `commercial-release-blocked`. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t14-supply-chain/`.
  Commit: Y | `build(release): gate signing and sbom evidence`

- [ ] T15. Support, incident, and hotfix operations
  What to do / Must NOT do: Create support triage policy, incident response playbook, hotfix/rollback policy, known limitations, and compatibility matrix. Do not imply 24/7 support unless commercially committed.
  Parallelization: Wave 4 | Blocked by: T12 | Blocks: T19
  References: `docs/MVP_STATUS.md`; `docs/WINDOWS_RISK_REVIEW.md`; release gate list in `src/qa_release_gates.rs` or equivalent.
  Acceptance criteria: `qa release` blocks if support/incident/hotfix docs are missing or stale.
  QA scenarios: happy: `qa release` with support/incident artifacts passes those gates; failure: omit incident response artifact and assert exact blocker. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t15-ops/`.
  Commit: Y | `docs(ops): add commercial support gates`

- [ ] T16. Commercial legal and licensing package
  What to do / Must NOT do: Draft EULA/license, third-party notices, privacy notice, evidence handling disclaimer, and sales/website wording boundaries. Do not assert legal admissibility.
  Parallelization: Wave 4 | Blocked by: T12 | Blocks: T19
  References: `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md`; `src/qa_report_defense.rs`; project license files.
  Acceptance criteria: Legal wording gate passes; banned claims fail.
  QA scenarios: happy: scan EULA/release notes/public copy for allowed bounded claims; failure: banned phrase fixture is rejected. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t16-legal-license/`.
  Commit: Y | `docs(legal): add bounded commercial terms`

- [ ] T17. External practitioner review
  What to do / Must NOT do: Run a structured pilot with at least two qualified forensic/video-review practitioners and record actionable feedback. Do not count internal self-review as external review.
  Parallelization: Wave 4 | Blocked by: T12 | Blocks: T19
  References: `docs/EVIDENCE_VIEWER_GUI.md`; `docs/validation-corpus.md`; `docs/RECOVERY_BOUNDARIES.md`.
  Acceptance criteria: Feedback log distinguishes fixed, accepted limitation, and blocking issues.
  QA scenarios: happy: import feedback CSV/JSON into a review disposition doc; failure: missing external reviewer identity/role blocks GA gate. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t17-external-review/`.
  Commit: Y | `docs(review): capture practitioner pilot disposition`

- [ ] T18. Post-GA monitoring and regression cadence
  What to do / Must NOT do: Define offline-friendly error reporting, manual update checks, regression schedule, compatibility matrix refresh, and vulnerability response. Do not add telemetry without opt-in policy.
  Parallelization: Wave 4 | Blocked by: T12 | Blocks: T19
  References: `docs/PERFORMANCE_VALIDATION.md`; `docs/WINDOWS_VALIDATION.md`; release gate list.
  Acceptance criteria: GA gate requires regression schedule and post-release monitoring policy.
  QA scenarios: happy: `qa release` sees regression schedule artifact; failure: stale schedule date blocks GA. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t18-post-ga/`.
  Commit: Y | `docs(release): define post-ga governance`

- [ ] T19. GA readiness decision engine
  What to do / Must NOT do: Add an explicit GA decision layer separate from field-pilot decision. Do not reuse `FIELD_PILOT_GO` as GA GO.
  Parallelization: Wave 5 | Blocked by: T13-T18 | Blocks: T20, T21
  References: `src/qa_release.rs`; `src/qa_release_decision.rs`; `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md`.
  Acceptance criteria: GA decision can only be `GA_GO`, `NO_GO`, or `BLOCKED`; it requires field-pilot evidence plus commercial gates.
  QA scenarios: happy: `frametrace qa release --ga` with complete artifacts emits `GA_GO`; failure: omit signing/external review and assert `NO_GO` or `BLOCKED`. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t19-ga-decision/`.
  Commit: Y | `feat(release): separate ga readiness decision`

- [ ] T20. GA release candidate rehearsal
  What to do / Must NOT do: Run a full RC rehearsal from clean clone to package, clean VM, corpus, release notes, and rollback. Do not skip uninstall/reinstall or rollback.
  Parallelization: Wave 5 | Blocked by: T19 | Blocks: T21
  References: all previous T1-T19 receipts.
  Acceptance criteria: RC rehearsal transcript is complete and cleanup receipt proves no leftover VM/process/temp/package state.
  QA scenarios: happy: run scripted RC rehearsal from a fresh clone; failure: intentionally break checksum/signature and assert rehearsal stops before launch. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t20-rc-rehearsal/`.
  Commit: Y | `ci(release): add ga rc rehearsal`

- [ ] T21. Final GA/commercial release report
  What to do / Must NOT do: Write final readiness report with score, evidence links, blockers, known limitations, support posture, and exact decision. Do not market beyond evidence.
  Parallelization: Wave 5 | Blocked by: T20 | Blocks: final verification
  References: `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md`; all T1-T20 evidence.
  Acceptance criteria: Report distinguishes local prototype, field-pilot, GA, and commercial support states.
  QA scenarios: happy: scope-fidelity script confirms all required scope terms and no banned claims; failure: add banned claim to temp report and assert lint fails. Evidence `.omo/evidence/frametrace-ga-commercial-readiness-20260628/t21-final-report/`.
  Commit: Y | `docs(readiness): publish ga commercial decision`

## Final verification wave
> Runs in parallel after ALL todos. ALL must APPROVE. Surface results and wait for the user's explicit okay before declaring complete.
- [ ] F1. Plan compliance audit
  Run a concrete verifier for this plan's receipts. Expected: T0-T21 have PASS, BLOCKED, or N/A receipts with no missing required artifact.
- [ ] F2. Code quality review
  Independent review over source, tests, scripts, and docs. Expected: APPROVE with no release-gate weakening, no GUI durable-state drift, no overclaim wording.
- [ ] F3. Real manual QA
  Re-run Windows CLI, WinUI, package, clean VM, corpus, and release decision scenarios from their evidence paths. Expected: PASS or exact blocker.
- [ ] F4. Scope fidelity
  Confirm final report separates prototype, field-pilot, GA, commercial support, legal posture, and unsupported formats; no banned claims.

## Commit strategy
- Keep commits atomic by wave: hygiene/plans, Windows engine, WinUI, package, clean VM, corpus, commercial gates, GA decision/report.
- Do not include `.omo/evidence` local QA output, browser profiles, generated DBs, screenshots, logs, or media files.
- Include source/docs/scripts/tests and durable `.omo/plans` needed for a fresh clone to continue.
- If Windows work happens on another machine, commit each Windows receipt-producing script/test update with the source that requires it.

## Success criteria
- A fresh Windows clone contains the source, docs, scripts, tests, and plans needed to continue without local macOS QA leftovers.
- T1 Windows engine validation is the first required action and remains the hard downstream gate.
- Field-pilot and GA are separate decision layers.
- Commercial release requires signed package, SBOM, clean VM, corpus, support, incident, privacy, legal, external review, and regression evidence.
- Every public claim remains local-first, evidence-bound, and review-assistive rather than certification/admissibility language.
