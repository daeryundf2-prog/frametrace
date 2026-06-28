# frametrace-overall-completion-uplift-20260627 - Work Plan

## TL;DR (For humans)
**What you'll get:** FrameTrace becomes a more complete local Windows forensic video review workstation by first making its release gates internally consistent, then turning the Evidence Viewer from a strong static prototype into an engine-backed review surface, and finally moving through real Windows validation, WinUI shell, installer, and field-pilot release evidence.

**Why this approach:** The Rust/SQLite engine is already the real source of truth, while the current GUI is still mock/prototype-scoped. The fastest honest uplift is to connect UI actions to engine receipts and keep the Windows blocker hard until a real Windows transcript exists.

**What it will NOT do:** It will not claim legal readiness, admissibility, or production readiness without evidence. It will not load full large-case inventories into browser memory. It will not start WinUI, installer, or final release work before the Windows engine gate passes.

**Effort:** XL
**Risk:** High - the remaining work crosses release gates, Rust engine contracts, GUI IA, Windows validation, WinUI shell, packaging, and field-pilot readiness evidence.
**Decisions I made for you:** Treat the target as field-pilot readiness, not GA; keep Korean-first UI with English forensic status tokens; preserve Rust/SQLite/audit as the only durable forensic state; require T13 Windows validation before T14-T17.

Your next move: approve this plan for execution with `$omo:start-work` when you want implementation to begin. Full execution detail follows below.

---

> TL;DR (machine): XL/high-risk plan to raise FrameTrace from 68/100 prototype/core readiness toward field-pilot readiness by fixing release-contract drift, engine-backed GUI workflow, large-case IA, Windows T13 validation, WinUI shell, installer/package, and field-pilot gates; GA remains out of scope.

## Scope
### Must have
- Preserve existing dirty worktree artifacts, especially untracked `.omo` evidence and plan files.
- Close the release-contract mismatch: Windows validation must generate typed JSON review manifests accepted by `qa release`, and `qa release` must emit an explicit `release-decision.json`.
- Keep release readiness fail-closed on missing Windows, WinUI, corpus, privacy, package, support, or review evidence.
- Improve the Evidence Viewer as a forensic workstation: dense inventory, source/facet IA, composable filters, clear prototype/live-case boundary, keyboard/focus support, and conservative decision gates.
- Bridge GUI actions to Rust/SQLite contracts: `workstation-status`, bounded `inventory`, `inventory-bulk-preview`, `inventory-export-manifest`, validation/playback/export/report/package commands, and audit receipts.
- Add long-running operation readiness: progress/ETA, dependency check, disk preflight, interrupted job state, audit-chain verification, and operator-facing blocked states.
- Obtain T13 evidence on a real Windows 10/11 x64 MSVC host before any WinUI/installer/release downstream work.
- Build the final Windows shell as engine-command-only C#/WinUI 3 after T13 passes.
- Validate installer/package on a clean Windows VM or record a hard blocker.
- Run final plan compliance, code quality, manual QA, security/privacy/legal wording, performance/reproducibility, and scope fidelity gates for field-pilot readiness or blocked readiness.

### Must NOT have (guardrails, anti-slop, scope boundaries)
- Do not edit product code while executing this plan in planning mode; implementation begins only after an explicit start request.
- Do not revert, delete, clean, or overwrite existing dirty worktree changes that are unrelated to the executor's current task.
- Do not claim `production-ready`, `court-certified`, `forensically proven`, `guaranteed integrity`, `automated court-grade verification`, `fully verified`, or `legally admissible`.
- Do not let `verified`, `complete`, or green status appear unless the engine and evidence state actually support that narrower claim.
- Do not let GUI write durable forensic state outside Rust engine commands, SQLite case DB, or chained audit logs.
- Do not load 100k/1M inventory rows into browser memory for production review.
- Do not start T14-T17, WinUI shell, installer/package, or final release gates while T13 is blocked.
- Do not treat static prototype browser QA as Windows/WinUI readiness.

## Verification strategy
> Evidence-first verification - agent-executed wherever the required local or Windows environment is available.
- Test decision: TDD for release contracts, CLI/engine JSON, and safety gates; characterization tests before refactors; browser/desktop manual QA for GUI and WinUI surfaces.
- All verification is agent-executed. Human approval is only for starting execution, external credentials/remote pushes if needed, and accepting the final handoff; it is not a substitute for evidence.
- Mac-safe baseline gates: `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo test --locked`, `node --check gui/evidence-viewer/app.js`, `node --check gui/evidence-viewer/translations.js`, `git diff --check`.
- GUI browser QA: launch a local static server for `gui/evidence-viewer`, run `agbrowse start --headed`, and capture `agbrowse snapshot`, `agbrowse screenshot`, and `agbrowse console` at 375, 768, 1280, 1440, and 1920 widths. If the generated case viewer changes, generate a real case with the CLI first and repeat browser QA against `review/evidence-viewer.html`.
- Windows QA: on a real Windows 10/11 x64 MSVC host, first run engine-only T13 validation with `scripts\windows\validate-engine.ps1 -CaseRoot C:\Temp\frametrace-engine-case -PerformanceRows 100000` or `scripts\windows\validate-release.ps1 -EngineOnly -CaseRoot C:\Temp\frametrace-engine-case -PerformanceRows 100000` after that mode is added. Only after WinUI/package work exists, run full release validation with `scripts\windows\validate-release.ps1 -CaseRoot C:\Temp\frametrace-release-case -PerformanceRows 100000`, plus targeted failure runs for missing tools, WinUI receipt, typed review manifest, Unicode/long path, and file-lock cases.
- Evidence root: `.omo/evidence/frametrace-overall-completion-uplift-20260627/`.

## Execution strategy
### Parallel execution waves
> Target 5-8 todos per wave. Fewer than 3 (except the final) means you under-split.
- Wave 1, Mac-safe contract and prototype honesty: T1-T4 can run mostly in parallel after T0.
- Wave 2, engine-to-GUI bridge and runtime readiness: T5-T6 depend on T1/T3 and can run in parallel with final GUI prototype QA.
- Wave 3, Windows T13: T7 is the hard external validation gate. If it blocks, record the blocked terminal state, mark T8-T11 N/A with evidence instead of attempting them, then run T12 to write the blocked readiness decision.
- Wave 4, Windows product shell and package: T8-T10 run only after T7 passes.
- Wave 5, field-pilot readiness: T11-T12 and final verification run after T8-T10, or the blocked-terminal verification runs after a T7 blocker.

### Dependency matrix
| Todo | Depends on | Blocks | Can parallelize with |
| --- | --- | --- | --- |
| T0 | none | T1-T12 | none |
| T1 | T0 | T7, T12 | T2, T3, T4 |
| T2 | T0 | T5, T12 | T1, T3, T4 |
| T3 | T0 | T5, T8 | T1, T2, T4 |
| T4 | T0 | T5, T8, T12 | T1, T2, T3 |
| T5 | T1, T3, T4 | T8, T12 | T6 |
| T6 | T1, T3 | T7, T8, T12 | T5 |
| T7 | T1, T6 | T8, T9, T10, T11, T12 or blocked-terminal final verification | none |
| T8 | T5, T6, T7 PASS | T9, T10, T12 | T11 after shell contract stabilizes |
| T9 | T7 PASS, T8 | T10, T12 | T11 |
| T10 | T7 PASS, T8, T9 | T12 | T11 |
| T11 | T7 PASS, T8 | T12 | T9, T10 |
| T12 | T1-T11 complete or T7 BLOCKED with T8-T11 N/A receipts | field-pilot decision or blocked handoff | final verification wave |

## Todos
> Implementation + Test = ONE todo. Never separate.
<!-- APPEND TASK BATCHES BELOW THIS LINE WITH edit/apply_patch - never rewrite the headers above. -->
- [x] T0. Baseline current state and protect dirty worktree
  What to do / Must NOT do: Record `pwd`, `git rev-parse --show-toplevel`, current branch, `git status --short`, current `.omo/boulder.json`, active start-work handoff, and latest UI review evidence before edits. Do not clean, reset, delete, or stage unrelated `.omo` artifacts.
  Parallelization: Wave 0 | Blocked by: none | Blocks: T1-T12
  References (executor has NO interview context - be exhaustive): `.omo/boulder.json:13-23`; `.omo/start-work/new-session-handoff.md:18-25`; `.omo/ulw-loop/frametrace-ui-ux-ia-completion-review-20260627/evidence/completion-score.md:5-15`; `.omo/ulw-loop/frametrace-ui-ux-ia-completion-review-20260627/evidence/source-review-summary.txt:38-41`.
  Acceptance criteria (agent-executable): `pwd`, `git rev-parse --show-toplevel`, `git branch --show-current`, `git status --short`, and `git diff --stat` transcripts are saved under `.omo/evidence/frametrace-overall-completion-uplift-20260627/task-00-baseline/`; existing dirty entries are listed as out-of-scope.
  QA scenarios (name the exact tool + invocation): happy: `omo sparkshell --shell 'pwd; git rev-parse --show-toplevel; git branch --show-current; git status --short --untracked-files=all; git diff --stat' --budget 20000` and verify no tracked product diff is hidden; failure: create `.omo/evidence/frametrace-overall-completion-uplift-20260627/task-00-baseline/scratch.untracked`, verify it appears in `git status --short --untracked-files=all` as out-of-scope and is not deleted by any cleanup, then remove only that scratch file. Evidence `.omo/evidence/frametrace-overall-completion-uplift-20260627/task-00-baseline/baseline.txt`.
  Commit: N | baseline evidence only.

- [x] T1. Normalize release gate contract and release decision artifacts
  What to do / Must NOT do: Add red-first coverage for the current mismatch where `scripts/windows/validate-release.ps1` emits text review input while `src/qa_release_manifest.rs` requires typed JSON. Update the script or add a shared release-fixture generator so preflight failures still emit typed review manifest and `release-decision.json` before exit. The typed manifest must carry artifact paths, reviewer/operator, timestamp, tool/evidence fields, and PASS/FAIL/BLOCKED status for every required review gate. Add `release-decision.json` output with `FIELD_PILOT_GO`, `NO_GO`, or `BLOCKED` semantics and exact blockers. Update stale docs to remove text/checkbox manifest acceptance. Do not loosen typed manifest validation.
  Parallelization: Wave 1 | Blocked by: T0 | Blocks: T7, T12
  References (executor has NO interview context - be exhaustive): `scripts/windows/validate-release.ps1:169-190`; `src/qa_release_manifest.rs:13-25`; `src/qa_release.rs:74-95`; `docs/recovery-test-spec.md:128-136`; `.omo/plans/frametrace-production-hardening-review-plan.md:57`; `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md:5-10`.
  Acceptance criteria (agent-executable): A failing-first test proves text `release-review.txt` is rejected; after implementation, typed JSON generated by the script/shared generator is accepted by `qa release`; `reports/qa/release-decision.json` is written for PASS, failing, and preflight-blocked release runs; docs no longer show text manifest as accepted format.
  QA scenarios (name the exact tool + invocation): red before implementation: `cargo test --locked qa_release_manifest qa_release -- --nocapture` must show text manifests are rejected, and `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/windows/validate-release.ps1 -CaseRoot /tmp/frametrace-release-contract-smoke -PerformanceRows 10` may fail before emitting typed artifacts, proving the current mismatch; green after T1 implementation: rerun the same PowerShell command on macOS and require a host/WinUI blocker plus parseable typed JSON manifest and `release-decision.json` with `NO_GO` or `BLOCKED`; failure: `CASE=$(mktemp -d /tmp/frametrace-release-contract-case.XXXXXX); cargo run --locked -- init-case "$CASE" --title "Release contract"; mkdir -p "$CASE/reports/qa"; printf 'privacy_review=pass\n' > "$CASE/reports/qa/release-review.txt"; cargo run --locked -- qa release "$CASE" --review-manifest "$CASE/reports/qa/release-review.txt" --output-dir "$CASE/reports/qa"` exits non-zero and contains `typed JSON review manifest`. Evidence `.omo/evidence/frametrace-overall-completion-uplift-20260627/task-01-release-contract/`.
  Commit: Y | `fix(release): align validation manifests and decisions`.

- [x] T2. Make readiness wording conservative across GUI, generated viewer, reports, and docs
  What to do / Must NOT do: Replace ambiguous `complete`, `verified`, or `court-ready`-adjacent UI/readiness wording with bounded states: `indexed`, `container check recorded`, `playback review recorded`, `report draft`, `export draft`, `examiner/legal review required`, `candidate-unvalidated`, `verification required`. Keep exact evidence values, paths, hashes, parser IDs, and raw validation states unchanged. Do not hide uncertainty behind green styling.
  Parallelization: Wave 1 | Blocked by: T0 | Blocks: T5, T12
  References (executor has NO interview context - be exhaustive): `README.md:137-140`; `docs/EVIDENCE_VIEWER_GUI.md:168-174`; `DESIGN.md:5-35`; `gui/evidence-viewer/translations.js:20-36`; `gui/evidence-viewer/app.js:453-463`; `src/qa_report_defense.rs:1-20`; `docs/WINDOWS_RISK_REVIEW.md:207-219`.
  Acceptance criteria (agent-executable): Restricted wording scan over `README.md`, `docs`, `gui/evidence-viewer`, `src/html_report.rs`, and report templates finds no banned overclaim or normal UI `court-ready` text except tests intentionally asserting rejection; decision gate labels never show generic `complete` for report/playback unless the corresponding engine artifact exists.
  QA scenarios (name the exact tool + invocation): happy: `rg -n 'production-ready|court-certified|forensically proven|guaranteed integrity|automated court-grade verification|fully verified|legally admissible|court-ready' README.md docs gui/evidence-viewer src --glob '!**/target/**'` returns only intentional negative tests and report-defense fixtures; browser happy: `agbrowse snapshot` at 1280 shows report/export/validation draft states and no normal UI `court-ready`; failure: inject a temporary fixture report containing `court-ready recovery` and run `cargo test --locked qa_report_defense -- --nocapture`, expecting failure. Evidence `.omo/evidence/frametrace-overall-completion-uplift-20260627/task-02-wording/`.
  Commit: Y | `fix(ui): keep readiness wording evidence-bound`.

- [x] T3. Add engine-backed GUI data adapter contract before more shell work
  What to do / Must NOT do: Define the exact JSON surfaces the final shell and generated viewer use for case open, inventory page/search/facets/detail, source tree, bulk preview, export manifest, validation/playback state, and report/package status. Prefer extending existing `workstation-status`, `inventory`, `inventory-bulk-preview`, and `inventory-export-manifest` outputs instead of inventing a parallel state model. Do not make the static prototype the production data owner.
  Parallelization: Wave 1 | Blocked by: T0 | Blocks: T5, T8
  References (executor has NO interview context - be exhaustive): `src/cli/commands.rs:11-260`; `src/cli/inventory_cmd.rs:99-162`; `src/case_db/inventory.rs:14-143`; `src/case_db/inventory_types.rs:1-105`; `docs/EVIDENCE_VIEWER_GUI.md:95-142`; `docs/WINUI3_SHELL_CONTRACT.md:26-60`; `docs/WINUI3_SHELL_CONTRACT.md:61-106`.
  Acceptance criteria (agent-executable): A schema/contract doc and tests prove the shell can open a case, read bounded inventory/facets/detail, preview actions, export manifests, and refresh status without GUI-owned durable state; page size remains capped at 500; missing case fails clearly.
  QA scenarios (name the exact tool + invocation): happy: `CASE=$(mktemp -d /tmp/frametrace-gui-contract-case.XXXXXX); SRC=$(mktemp -d /tmp/frametrace-gui-contract-src.XXXXXX); printf '\0\0\0\030ftypmp42payload' > "$SRC/clip.mp4"; cargo run --locked -- init-case "$CASE" --title "GUI contract"; cargo run --locked -- scan-folder "$CASE" "$SRC" --no-ffprobe; cargo run --locked -- workstation-status "$CASE"; cargo run --locked -- inventory "$CASE" --limit 1000 --extension mp4 --validation-state candidate-unvalidated --sort risk-timestamp-asc; cargo run --locked -- inventory "$CASE" --facets; cargo run --locked -- inventory "$CASE" --file-id vid_000001; cargo run --locked -- inventory-bulk-preview "$CASE" --action add-to-report --operator qa vid_000001; cargo run --locked -- inventory-export-manifest "$CASE" --operator qa --output "$CASE/reports/inventory-export.json" vid_000001`, saving JSON outputs; failure: `BAD=$(mktemp -d /tmp/frametrace-not-case.XXXXXX); cargo run --locked -- inventory "$BAD" --limit 10` exits non-zero and contains `not a FrameTrace case`. Evidence `.omo/evidence/frametrace-overall-completion-uplift-20260627/task-03-gui-data-contract/`.
  Commit: Y | `feat(gui-contract): expose shell-safe case data`.

- [x] T4. Upgrade Evidence Viewer IA to dense source/facet workstation mode
  What to do / Must NOT do: Compress the first-screen workflow cards into a workstation status strip on desktop, add a compact source/facet rail or drawer, make filters composable by source/type/parser/validation/review/hash/report/size/time, expose mock/prototype status in-render, reduce row density toward documented 34/44/64px modes, preserve preview-first review, and keep keyboard/focus visible. Do not add decorative hero UI or extra marketing copy.
  Parallelization: Wave 1 | Blocked by: T0 | Blocks: T5, T8, T12
  References (executor has NO interview context - be exhaustive): `docs/EVIDENCE_VIEWER_GUI.md:21-94`; `docs/EVIDENCE_VIEWER_GUI.md:144-153`; `docs/GUI_LARGE_INVENTORY_EXECUTION_PLAN.md:15-31`; `docs/gui-large-inventory-traceability.md:21-31`; `DESIGN.md:87-104`; `DESIGN.md:121-135`; `gui/evidence-viewer/index.html:33-173`; `gui/evidence-viewer/app.js:5-6`; `gui/evidence-viewer/app.js:192-260`; `gui/evidence-viewer/app.js:1035-1050`; `gui/evidence-viewer/styles.css:272`.
  Acceptance criteria (agent-executable): At 375px no horizontal overflow; at 768px panes stack without hidden CTAs; at 1280px preview/candidates/inspector are visible; at 1440px default mode shows at least 12 rows or explicitly records a lower supported density; at 1920px shows at least 18 rows; inventory-focused mode shows at least 30 rows; keyboard focus is visible for search, row navigation, preview, export/report/verify actions.
  QA scenarios (name the exact tool + invocation): happy: `python3 -m http.server 4183 --bind 127.0.0.1` from `gui/evidence-viewer`, `agbrowse start --headed`, `agbrowse navigate http://127.0.0.1:4183/index.html`, then `agbrowse snapshot`, `agbrowse screenshot`, `agbrowse console` at 375, 768, 1280, 1440, 1920; failure: browser script sets a long Korean/UNC path fixture and asserts `document.documentElement.scrollWidth <= window.innerWidth`, visible focus ring after Tab, and no console errors. Evidence `.omo/evidence/frametrace-overall-completion-uplift-20260627/task-04-evidence-viewer-ia/`.
  Commit: Y | `feat(viewer): add dense source-aware review IA`.

- [x] T5. Replace browser-only action mutations with auditable preview/receipt workflow
  What to do / Must NOT do: Change static GUI actions to clearly behave as non-mutating previews and queue drafts, and wire generated/live-case paths to Rust preview/export/validation/report/package commands where possible. Bulk actions must show selected count, filters, expected mutation, operator, warnings, preview ID, and audit path before mutation. Do not let a browser click mark durable review/report/export/validation state without engine output.
  Parallelization: Wave 2 | Blocked by: T1, T3, T4 | Blocks: T8, T12
  References (executor has NO interview context - be exhaustive): `gui/evidence-viewer/app.js:504-564`; `src/case_db/inventory.rs:102-143`; `src/case_db/inventory_export.rs`; `tests/cli_inventory.rs:125-168`; `docs/EVIDENCE_VIEWER_GUI.md:80-83`; `docs/GUI_LARGE_INVENTORY_EXECUTION_PLAN.md:213-219`; `DESIGN.md:168`.
  Acceptance criteria (agent-executable): Browser UI never describes local state toggles as durable changes; engine-backed CLI tests prove preview does not mutate DB; export manifest writes inside the case directory only and refuses outside/source/symlink/existing paths; generated UI reports action outputs as `draft` or `engine receipt` according to the actual surface.
  QA scenarios (name the exact tool + invocation): happy CLI: `cargo test --locked cli_inventory bulk_preview inventory_export -- --nocapture`; happy browser: click report/export/verify/preview actions in `agbrowse` and assert visible preview/receipt language includes `draft`, `preview`, or `audit path`; failure CLI: `CASE=$(mktemp -d /tmp/frametrace-action-case.XXXXXX); SRC=$(mktemp -d /tmp/frametrace-action-src.XXXXXX); printf '\0\0\0\030ftypmp42payload' > "$SRC/clip.mp4"; cargo run --locked -- init-case "$CASE" --title "Action preview"; cargo run --locked -- scan-folder "$CASE" "$SRC" --no-ffprobe; cargo run --locked -- inventory-export-manifest "$CASE" --operator qa --output "/tmp/frametrace-outside-export.json" vid_000001` exits non-zero with `inside the case directory`; failure browser: assert no `.textContent` contains `durable`, `committed`, or `complete` for uncommitted prototype actions. Evidence `.omo/evidence/frametrace-overall-completion-uplift-20260627/task-05-action-preview/`.
  Commit: Y | `fix(viewer): require audit preview before GUI mutations`.

- [x] T6. Add runtime readiness surfaces for long forensic jobs
  What to do / Must NOT do: Add or complete engine/CLI surfaces needed by Windows GUI for dependency status, disk-space preflight, job progress/ETA, interrupted job review, audit-chain verification, and missing-tool feature gating. Use existing job tables, `workstation-status`, `mark-interrupted-jobs`, and `windows_prerequisites` where possible. Do not implement unreliable resume semantics unless the engine can prove idempotence; label resume blockers honestly.
  Parallelization: Wave 2 | Blocked by: T1, T3 | Blocks: T7, T8, T12
  References (executor has NO interview context - be exhaustive): `README.md:87`; `src/case_db/jobs.rs`; `src/cli/commands.rs:231-249`; `src/windows_prerequisites.rs:1-100`; `docs/WINDOWS_RISK_REVIEW.md:7-17`; `docs/WINDOWS_RISK_REVIEW.md:18-81`; `docs/WINDOWS_RISK_REVIEW.md:104-170`; `docs/WINUI3_SHELL_CONTRACT.md:108-115`.
  Acceptance criteria (agent-executable): A case with running/interrupted/completed jobs has bounded status output; missing tools produce feature-specific blockers; disk-space preflight exists for import/carve/proxy/export/package or emits an explicit blocker; audit verification command remains separate and fails on tampered logs.
  QA scenarios (name the exact tool + invocation): happy: `CASE=$(mktemp -d /tmp/frametrace-runtime-case.XXXXXX); SRC=$(mktemp -d /tmp/frametrace-runtime-src.XXXXXX); printf '\0\0\0\030ftypmp42payload' > "$SRC/clip.mp4"; cargo run --locked -- init-case "$CASE" --title "Runtime readiness"; cargo run --locked -- scan-folder "$CASE" "$SRC" --no-ffprobe; cargo run --locked -- workstation-status "$CASE"; cargo run --locked -- inspect "$CASE"; cargo run --locked -- mark-interrupted-jobs "$CASE" --reason qa; cargo test --locked qa_shell_contract jobs audit -- --nocapture`; failure: run the same status/preflight checks with missing tool path overrides or tampered JSONL hash-chain fixtures and assert feature-specific blockers/non-zero output. Evidence `.omo/evidence/frametrace-overall-completion-uplift-20260627/task-06-runtime-readiness/`.
  Commit: Y | `feat(engine): expose workstation runtime readiness`.

- [x] T7. Complete T13 on real Windows or preserve the hard blocker
  What to do / Must NOT do: Split engine-only T13 validation from post-WinUI release validation. Add `scripts/windows/validate-engine.ps1` or an `-EngineOnly` mode to `validate-release.ps1` so T13 does not require `gui\winui`, `dotnet build`, or `winui-build.json`. On a real Windows 10/11 x64 MSVC host or safe Windows runner, run the T13 Rust engine validation transcript before WinUI work. Adding `workflow_dispatch` is allowed as a reviewed code change, but triggering remote CI or pushing requires explicit active-session git/remote authorization; absent that, use local Windows evidence or refresh the blocker receipt. If no runner exists, refresh the blocker receipt, mark T8-T11 N/A, then continue to T12 for a blocked readiness decision.
  Parallelization: Wave 3 | Blocked by: T1, T6 | Blocks: T8, T9, T10, T11, T12
  References (executor has NO interview context - be exhaustive): `.omo/boulder.json:13-23`; `.omo/evidence/frametrace-production-hardening-review-plan/task-13-windows-engine/BLOCKED-missing-windows-runner.json:1-17`; `.omo/plans/frametrace-production-hardening-review-plan.md:191-199`; `docs/WINDOWS_IMPLEMENTATION_HANDOFF.md:70-139`; `docs/WINDOWS_VALIDATION.md:1-45`; `.github/workflows/windows-ci.yml:1-41`.
  Acceptance criteria (agent-executable): Passing evidence includes Windows host identity, Rust MSVC build/test/clippy/fmt, ffmpeg/ffprobe discovery, synthetic MP4 workflow, validation/playback separation, Unicode/long path checks, repeated scans, file-lock behavior, bounded inventory, workstation-status, and `reports\qa\windows-engine-validation.json`. If unavailable, a refreshed BLOCKED JSON states why T8-T11 are N/A and why T12 must write `release-decision.json` as `BLOCKED`.
  QA scenarios (name the exact tool + invocation): red before implementation: `test -f scripts/windows/validate-engine.ps1 || ! pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/windows/validate-release.ps1 -EngineOnly -CaseRoot /tmp/frametrace-t13-red -PerformanceRows 10` proves the engine-only lane is absent or blocked and must not count as T13 PASS; green after T7 script/mode implementation on Windows: `powershell -ExecutionPolicy Bypass -File scripts/windows/validate-engine.ps1 -CaseRoot C:\Temp\frametrace-engine-case -PerformanceRows 100000` or `powershell -ExecutionPolicy Bypass -File scripts/windows/validate-release.ps1 -EngineOnly -CaseRoot C:\Temp\frametrace-engine-case -PerformanceRows 100000`; failure Windows: remove `ffprobe.exe` or run on non-MSVC Rust and assert engine validation exits non-zero with a toolchain/tool blocker; blocker macOS after implementation: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/windows/validate-release.ps1 -EngineOnly -CaseRoot /tmp/frametrace-t13-blocker -PerformanceRows 10` must not count as T13 PASS and must write a blocker receipt. Evidence `.omo/evidence/frametrace-overall-completion-uplift-20260627/task-07-windows-t13/`.
  Commit: Y if workflow/script/docs changed | `ci(windows): make engine validation triggerable and fail-closed`.

- [x] T8. Implement minimal WinUI 3 shell as engine-command-only workstation
  What to do / Must NOT do: After T7 PASS only, create `gui/winui` with a buildable C#/WinUI 3 solution, engine adapter, and minimum workflows: open/create case, source intake, dependency/status panel, dense inventory page, media/proxy preview handoff, inspector, validation/playback/report/export/package actions, job state, and audit trail. Use the current stable Microsoft Windows App SDK package at implementation time, pin the exact version in `Directory.Packages.props` or the WinUI project file, and record the Microsoft Learn/NuGet source used. Add a UI automation harness using MSTest plus FlaUI/UI Automation or Microsoft-documented WinUI test guidance, pinned in test project dependencies. GUI must execute Rust commands or read bounded JSON/SQLite-derived outputs; it must not write durable forensic state directly.
  Parallelization: Wave 4 | Blocked by: T5, T6, T7 | Blocks: T9, T11
  References (executor has NO interview context - be exhaustive): `docs/WINUI3_SHELL_CONTRACT.md:1-60`; `docs/WINUI3_SHELL_CONTRACT.md:61-133`; `docs/WINDOWS_IMPLEMENTATION_HANDOFF.md:240-260`; `README.md:23-35`; `docs/EVIDENCE_VIEWER_GUI.md:248-260`; `src/windows_prerequisites.rs:6-66`.
  Acceptance criteria (agent-executable): `dotnet build gui\winui\FrameTrace.sln -c Release` and `dotnet test gui\winui\FrameTrace.Tests\FrameTrace.Tests.csproj -c Release` pass; `reports\qa\winui-build.json` is generated from actual build/test; shell can open a synthetic case and show bounded inventory/status without durable GUI-owned state.
  QA scenarios (name the exact tool + invocation): happy Windows GUI: `powershell -ExecutionPolicy Bypass -File scripts/windows/smoke-winui.ps1 -Solution gui\winui\FrameTrace.sln -CaseRoot C:\Temp\frametrace-winui-smoke -ScreenshotDir .omo\evidence\frametrace-overall-completion-uplift-20260627\task-08-winui-shell\screenshots -ActionLog .omo\evidence\frametrace-overall-completion-uplift-20260627\task-08-winui-shell\winui-action-log.jsonl` creates/opens a case, registers a source, browses/searches inventory, opens preview, queues validation/export/report through the engine adapter, refreshes status, and captures at least one screenshot plus JSONL action log; failure: `powershell -ExecutionPolicy Bypass -File scripts/windows/smoke-winui.ps1 -EnginePath C:\Temp\failing-frametrace.exe -CaseRoot C:\Temp\frametrace-winui-fail` asserts UI displays blocked/failed state while writing no durable case mutation. Evidence `.omo/evidence/frametrace-overall-completion-uplift-20260627/task-08-winui-shell/`.
  Commit: Y | `feat(winui): add engine-owned workstation shell`.

- [x] T9. Define and implement Windows package technology and validation scripts
  What to do / Must NOT do: Choose MSIX as the primary installer/package technology for the WinUI shell and Rust engine because FrameTrace targets Windows 10/11 local workstation use; allow a portable ZIP only as an unsigned lab fallback that cannot produce field-pilot GO. Create `scripts/windows/build-package.ps1` and `scripts/windows/validate-package.ps1`. Add SPDX JSON SBOM generation, checksum manifest, dependency manifest, uninstall/reinstall checks, and a signing policy that emits `signing-blocked` when no secure certificate path is configured. Do not embed signing secrets in the repo or accept a manually fabricated package receipt.
  Parallelization: Wave 4 | Blocked by: T7 PASS, T8 | Blocks: T10, T12
  References (executor has NO interview context - be exhaustive): `.omo/plans/frametrace-production-hardening-review-plan.md:215-220`; `docs/TECH_STACK.md:26-32`; `docs/MVP_STATUS.md:57-70`; `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md:45-58`; `docs/WINDOWS_VALIDATION.md:33-45`; `src/package.rs:27-178`.
  Acceptance criteria (agent-executable): `scripts/windows/build-package.ps1` produces an MSIX or unsigned lab ZIP plus manifest/checksums/SBOM; `scripts/windows/validate-package.ps1` exists, validates signatures/checksums/manifests/dependencies, and emits a typed package receipt; unsigned lab-only output may be produced for local testing, but it records `signing-blocked` and must keep the final decision `NO_GO` or `BLOCKED`, never `FIELD_PILOT_GO`.
  QA scenarios (name the exact tool + invocation): happy: `powershell -ExecutionPolicy Bypass -File scripts\windows\build-package.ps1 -Configuration Release -OutputDir C:\Temp\frametrace-package-out` followed by `powershell -ExecutionPolicy Bypass -File scripts\windows\validate-package.ps1 -PackagePath C:\Temp\frametrace-package-out\<package> -ReceiptPath C:\Temp\frametrace-package-out\installer-package-validation.json`; failure: tamper a checksum in the package manifest and assert `validate-package.ps1` exits non-zero before install/use. Evidence `.omo/evidence/frametrace-overall-completion-uplift-20260627/task-09-package-scripts/`.
  Commit: Y | `build(windows): define workstation package validation`.

- [x] T10. Validate package on a clean Windows VM
  What to do / Must NOT do: Run the package from T9 on a clean Windows 11 x64 VM with no development checkout. Validate install, dependency discovery, launch, synthetic case workflow, uninstall/reinstall, checksum verification, and blocked signing/package states. Do not count the developer machine as clean-VM evidence.
  Parallelization: Wave 4 | Blocked by: T7 PASS, T8, T9 | Blocks: T12
  References (executor has NO interview context - be exhaustive): `scripts/windows/validate-package.ps1` created by T9; `.omo/plans/frametrace-production-hardening-review-plan.md:215-220`; `docs/WINDOWS_VALIDATION.md:33-45`; `docs/WINUI3_SHELL_CONTRACT.md:116-133`.
  Acceptance criteria (agent-executable): Clean VM receipt includes OS/build, install method, package path, checksum, launch proof, dependency status, synthetic case workflow, uninstall/reinstall proof, screenshots, and `FIELD_PILOT_BLOCKED` when signing/package requirements are not satisfied.
  QA scenarios (name the exact tool + invocation): happy: `powershell -ExecutionPolicy Bypass -File scripts\windows\validate-package.ps1 -PackagePath <package> -CaseRoot C:\Temp\frametrace-installed-case -CleanVmReceipt C:\Temp\frametrace-installed-case\reports\qa\clean-vm-package.json -ScreenshotDir C:\Temp\frametrace-installed-case\reports\qa\screenshots`; failure: run with a package whose checksum manifest was altered and assert validation fails before launch. Evidence `.omo/evidence/frametrace-overall-completion-uplift-20260627/task-10-clean-vm-package/`.
  Commit: Y | `test(windows): prove clean-vm package workflow`.

- [x] T11. Run field-pilot corpus and real-surface workflow evidence without overclaiming GA
  What to do / Must NOT do: Add a field-pilot validation lane with synthetic, known-good, and mixed real-world-like sample manifests where available. Keep unsupported vendor formats, missing E01/TSK tools, and synthetic-only corpus as explicit blockers or limitations. Do not claim corpus accuracy beyond the evidence class.
  Parallelization: Wave 4 | Blocked by: T7 PASS, T8 | Blocks: T12
  References (executor has NO interview context - be exhaustive): `docs/validation-corpus.md:1-80`; `src/qa_accuracy/*`; `src/qa_repro.rs`; `docs/MANUFACTURER_PARSER_RESEARCH.md`; `docs/RECOVERY_BOUNDARIES.md`; `docs/FILESYSTEM_RECOVERY.md:67`; `docs/MVP_STATUS.md:73-80`.
  Acceptance criteria (agent-executable): `qa accuracy`, `qa reproducibility`, `qa performance`, `qa report-defense`, generated viewer/browser QA, and package validation run against declared corpus classes; final docs distinguish synthetic, lab, field-pilot, and external-review evidence. A mixed real-world-like corpus is required for field-pilot GO; if unavailable, record `field-pilot-blocked: mixed-corpus-unavailable` rather than claiming readiness.
  QA scenarios (name the exact tool + invocation): happy synthetic smoke: `ROOT=$(mktemp -d /tmp/frametrace-field-pilot.XXXXXX); CASE="$ROOT/case"; SRC="$ROOT/src"; OUT="$ROOT/qa"; mkdir -p "$SRC"; printf '\0\0\0\030ftypmp42payload' > "$SRC/clip.mp4"; cargo run --locked -- init-case "$CASE" --title "Field pilot corpus smoke"; cargo run --locked -- scan-folder "$CASE" "$SRC" --no-ffprobe; printf '%s\n' "$SRC/clip.mp4" > "$ROOT/corpus.tsv"; cargo run --locked -- qa accuracy "$CASE" "$ROOT/corpus.tsv" --output-dir "$OUT/accuracy"; cargo run --locked -- qa reproducibility "$CASE" "$CASE" --output-dir "$OUT/repro"; cargo run --locked -- qa performance "$OUT/performance" --rows 100000; cargo run --locked -- make-review "$CASE"` followed by browser QA against `$CASE/review/evidence-viewer.html`; happy field-pilot corpus: repeat the same commands with the declared mixed real-world-like corpus manifest path and save the corpus manifest receipt; failure: `printf '%s\tbad-hash\n' "$SRC/clip.mp4" > "$ROOT/corpus-bad.tsv"; cargo run --locked -- qa accuracy "$CASE" "$ROOT/corpus-bad.tsv" --output-dir "$OUT/accuracy-bad"` must fail or report false negatives, and removing required validation/report logs must make report-defense fail. Evidence `.omo/evidence/frametrace-overall-completion-uplift-20260627/task-11-field-pilot-corpus/`.
  Commit: Y | `test(qa): capture field-pilot readiness evidence`.

- [x] T12. Run final field-pilot readiness and update the honest completion score
  What to do / Must NOT do: Run final `qa release` with typed manifest, corpus manifest, comparison case, performance output, Windows engine receipt, Windows prerequisite receipt, WinUI build receipt, package receipt or typed package gate, support/incident/known-limitations docs, and legal wording scan. If T7 blocked, do not run downstream release; instead write N/A receipts for T8-T11 and a blocked readiness score. Update readiness docs and score using only evidence-backed claims. Do not issue a general-availability release decision; this plan can only produce `FIELD_PILOT_GO`, `NO_GO`, or `BLOCKED`.
  Parallelization: Wave 5 | Blocked by: T1-T11 complete or T7 BLOCKED | Blocks: field-pilot decision
  References (executor has NO interview context - be exhaustive): `src/qa_release.rs:65-95`; `src/qa_release_gates.rs`; `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md:139-143`; `.omo/ulw-loop/frametrace-ui-ux-ia-completion-review-20260627/evidence/completion-score.md:5-15`; `.omo/plans/frametrace-production-hardening-review-plan.md:223-241`.
  Acceptance criteria (agent-executable): `release-readiness.json`, `release-readiness.md`, and `release-decision.json` exist; if all field-pilot gates PASS the decision is `FIELD_PILOT_GO`; if any blocker remains the decision is `NO_GO` or `BLOCKED` with exact blocker list; if T7 blocked then T8-T11 are N/A with receipts; updated completion score separates static GUI, generated viewer, Rust engine, Windows engine, WinUI shell, package, corpus, field pilot, and GA-out-of-scope.
  QA scenarios (name the exact tool + invocation): happy: `target\release\frametrace.exe qa release C:\Temp\frametrace-release-case --review-manifest C:\Temp\frametrace-release-case\reports\qa\release-review.json --corpus-manifest C:\Temp\frametrace-release-case\corpus.tsv --comparison-case C:\Temp\frametrace-release-case --performance-output-dir C:\Temp\frametrace-release-case\reports\qa\performance --output-dir C:\Temp\frametrace-release-case\reports\qa`; failure: remove or mark FAIL/BLOCKED for `windows_workstation_validation`, `installer_package_validation`, or `mixed_real_world_like_corpus` in the typed manifest and assert non-zero plus exact blocker; blocked path: with T7 BLOCKED, run the readiness summarizer and assert `release-decision.json` is `BLOCKED` and T8-T11 are N/A. Evidence `.omo/evidence/frametrace-overall-completion-uplift-20260627/task-12-final-field-pilot/`.
  Commit: Y | `docs(readiness): report evidence-bound completion`.

## Final verification wave
> Runs after all todos are complete or explicitly N/A due to a T7 blocker. ALL applicable gates must APPROVE; N/A gates require blocker receipts.
- [x] F1. Plan compliance audit
  Verify every T0-T12 reference, acceptance criterion, QA artifact, blocker, and commit is present or explicitly blocked/N/A. T12 must update `scripts/qa/verify-plan-evidence.py` for this plan's T0-T12 receipt profile or add a new concrete verifier before F1 uses it. Command: `python3 scripts/qa/verify-plan-evidence.py .omo/plans/frametrace-overall-completion-uplift-20260627.md .omo/evidence/frametrace-overall-completion-uplift-20260627`; expected result: exit 0 and list T0-T12 plus F1-F4 covered, or exit non-zero with exact missing evidence. Evidence `.omo/evidence/frametrace-overall-completion-uplift-20260627/final/F1-plan-compliance.md`.
- [x] F2. Code quality review
  Run independent code review over all changed files and the final diff. Exact invocation: `codex exec --cd /Users/shinyoohag/Desktop/frametrace "Review the final diff for frametrace-overall-completion-uplift-20260627. Check: no unrelated dirty worktree rollback, no GUI durable state ownership, no release gate weakening, no full-case JSON production path, no overclaim wording. Return APPROVE or ITERATE with file:line findings." > .omo/evidence/frametrace-overall-completion-uplift-20260627/final/F2-code-quality.md`; expected result: reviewer returns APPROVE. Evidence `.omo/evidence/frametrace-overall-completion-uplift-20260627/final/F2-code-quality.md`.
- [x] F3. Real manual QA
  Drive CLI, static/generated browser UI, Windows validation script, WinUI shell, installer/package, and final `qa release` through explicit PASS or BLOCKED branches. Local browser invocation always runs: `cd gui/evidence-viewer && python3 -m http.server 4183 --bind 127.0.0.1`, `agbrowse start --headed`, `agbrowse navigate http://127.0.0.1:4183/index.html`, then `agbrowse snapshot`, `agbrowse screenshot`, `agbrowse console` at 375/768/1280/1440/1920; expected result: no console errors, no horizontal overflow, required forensic states visible. If T7 PASS exists, run Windows invocations in order: `scripts\windows\validate-engine.ps1`, `scripts\windows\smoke-winui.ps1`, `scripts\windows\build-package.ps1`, `scripts\windows\validate-package.ps1`, and final `qa release` commands from T7-T12; expected result: PASS receipts or exact task-level failure. If T7 is BLOCKED, do not run WinUI/package/release PASS commands; instead verify T7 BLOCKED receipt, T8-T11 N/A receipts, T12 `release-decision.json` with `BLOCKED`, and F4 scope fidelity. Evidence `.omo/evidence/frametrace-overall-completion-uplift-20260627/final/F3-real-manual-qa/`.
- [x] F4. Scope fidelity
  Confirm the final report distinguishes prototype, generated viewer, Rust engine, Windows engine, WinUI shell, package, corpus, field-pilot, and GA-out-of-scope states; T13/T14-T17 are not bypassed; all remaining blockers are honest. Exact invocation:
  ```bash
  python3 - <<'PY'
  import json
  import pathlib
  import re

  root = pathlib.Path(".")
  evidence = root / ".omo/evidence/frametrace-overall-completion-uplift-20260627"
  decision_files = list(evidence.rglob("release-decision.json"))
  if not decision_files:
      raise SystemExit("FAIL missing release-decision.json")
  decision = json.loads(decision_files[-1].read_text(encoding="utf-8"))
  status = decision.get("decision") or decision.get("status")
  if status not in {"FIELD_PILOT_GO", "NO_GO", "BLOCKED"}:
      raise SystemExit(f"FAIL invalid decision: {status!r}")

  text = "\n".join(
      p.read_text(encoding="utf-8", errors="ignore")
      for p in evidence.rglob("*")
      if p.is_file() and p.suffix.lower() in {".md", ".txt", ".json"}
  )
  banned = re.compile(r"production-ready|court-certified|forensically proven|guaranteed integrity|automated court-grade verification|fully verified|legally admissible", re.I)
  if banned.search(text):
      raise SystemExit("FAIL forbidden claim in evidence")
  required_terms = ["prototype", "generated viewer", "Rust engine", "Windows engine", "WinUI", "package", "corpus", "field-pilot", "GA-out-of-scope"]
  missing = [term for term in required_terms if term.lower() not in text.lower()]
  if missing:
      raise SystemExit(f"FAIL missing scope terms: {missing}")

  t7_blockers = list((evidence / "task-07-windows-t13").glob("**/*BLOCKED*.json"))
  downstream_pass = any(
      "PASS" in p.read_text(encoding="utf-8", errors="ignore")
      for n in (8, 9, 10, 11)
      for p in evidence.glob(f"task-{n:02d}-*/**/*")
      if p.is_file()
  )
  if t7_blockers and downstream_pass:
      raise SystemExit("FAIL T8-T11 PASS evidence exists while T7 is BLOCKED")
  print("SCOPE_FIDELITY_PASS")
  PY
  ```
  Expected result: `SCOPE_FIDELITY_PASS`. Evidence `.omo/evidence/frametrace-overall-completion-uplift-20260627/final/F4-scope-fidelity.md`.

## Commit strategy
- Keep commits atomic and dependency-ordered. Do not auto-commit from planning mode.
- Suggested order:
  1. `fix(release): align validation manifests and decisions`
  2. `fix(ui): keep readiness wording evidence-bound`
  3. `feat(gui-contract): expose shell-safe case data`
  4. `feat(viewer): add dense source-aware review IA`
  5. `fix(viewer): require audit preview before GUI mutations`
  6. `feat(engine): expose workstation runtime readiness`
  7. `ci(windows): split engine validation from release validation`
  8. `feat(winui): add engine-owned workstation shell`
  9. `build(windows): define workstation package validation`
  10. `test(windows): prove clean-vm package workflow`
  11. `test(qa): capture field-pilot readiness evidence`
  12. `docs(readiness): report evidence-bound completion`
- Every commit message must follow the repo's Lore protocol if the executor is committing in this environment.
- If Windows work must happen on another machine, commit Mac-safe Rust/docs/GUI work separately from Windows shell/package changes.

## Success criteria
- Release contract is internally consistent: typed JSON review manifests only, `release-decision.json` produced, stale docs fixed, and failing manifests fail closed.
- Evidence Viewer behaves like a dense forensic workstation rather than a marketing page or toy prototype, while still clearly disclosing mock/prototype state where applicable.
- GUI actions are auditable previews/engine receipts, not browser-owned durable mutations.
- Large-case review uses bounded SQLite/page/query contracts; production does not load 100k/1M rows into browser memory.
- T13 passes on a real Windows 10/11 x64 MSVC host, or downstream Windows tasks remain explicitly blocked.
- WinUI shell builds/tests on Windows and preserves Rust/SQLite/audit source-of-truth boundaries, only after T13 passes.
- Package technology, signing policy, validation scripts, and clean-VM evidence exist, or record a field-pilot blocker.
- Final readiness report gives separate scores/status for engine, static GUI, generated viewer, Windows shell, package, corpus, field pilot, and GA-out-of-scope.
- No forbidden legal/forensic overclaims appear outside negative tests.
- Existing unrelated dirty worktree entries remain untouched.
