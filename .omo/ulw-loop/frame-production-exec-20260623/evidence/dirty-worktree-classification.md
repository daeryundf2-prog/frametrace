# FrameTrace Dirty Worktree Classification

Generated: 2026-06-23
Branch: codex/frametrace-forensic-hardening
ULW session: frame-production-exec-20260623

## Command Surface Used

- `git status --short`
- `git diff --stat`
- `git diff --name-only`
- `git log -8 --oneline`
- targeted reads of `src/lib.rs`, `src/cli/mod.rs`, `src/cli/commands.rs`, and `src/cli/handlers.rs`

## Observed Commit Style

Recent commits are imperative subject lines without Conventional Commit prefixes:

- `Gate GUI work behind verified ULW completion contracts`
- `Make forensic inventory review bounded and audit-backed`
- `Measure resource usage in performance QA`
- `Require query-plan evidence in performance QA`
- `Block release review on active SQLite jobs`

Commit messages for this branch should keep that style unless explicitly changed.

## Dirty Worktree Groups

### Group A: Windows/WinUI prerequisite release gate

Primary files:

- `src/windows_prerequisites.rs`
- `src/windows_prerequisites_tests.rs`
- `src/workstation.rs`
- `src/qa_shell_contract.rs`
- `src/qa_release.rs`
- `tests/cli_windows_prereq.rs`
- `tests/cli_lifecycle.rs`
- `scripts/windows/validate-release.ps1`
- `docs/WINUI3_SHELL_CONTRACT.md`
- `docs/WINDOWS_IMPLEMENTATION_HANDOFF.md`
- `docs/WINDOWS_VALIDATION.md`
- `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md`
- selected `src/lib.rs`, `src/cli/commands.rs`, `src/cli/mod.rs`, `src/cli/handlers.rs` hunks for `workstation-status`

Risk: the shared CLI/lib files also contain media and inventory commands, so staging only Windows hunks requires careful hunk staging. Safer first validation is full-tree verification, then either commit the integrated state or split with hunk-level staging after confirming no cross-module dependency breaks.

macOS executable: yes for negative readiness proof. Windows-only blocker: positive WinUI build/test and `dotnet build/test` receipt.

### Group B: SQLite inventory/query/export and bounded large inventory GUI support

Primary files:

- `src/case_db/inventory.rs`
- `src/case_db/inventory_query.rs`
- `src/case_db/inventory_types.rs`
- `src/case_db/inventory_tests.rs`
- `src/case_db/inventory_export.rs`
- `src/case_db/inventory_facets.rs`
- `src/cli/inventory_cmd.rs`
- `src/cli/inventory_json.rs`
- `tests/cli_inventory.rs`
- `gui/evidence-viewer/app.js`
- `gui/evidence-viewer/index.html`
- `gui/evidence-viewer/styles.css`
- `docs/EVIDENCE_VIEWER_GUI.md`
- `docs/GUI_LARGE_INVENTORY_EXECUTION_PLAN.md`
- `docs/gui-large-inventory-traceability.md`

macOS executable: yes for CLI, generated HTML, static browser/localhost proof. Windows-only blocker: none for HTML prototype.

### Group C: Media validation, derived artifacts, audit chain, report defensibility

Primary files:

- `src/artifacts.rs`
- `src/audit.rs`
- `src/ffprobe.rs`
- `src/media_contract.rs`
- `src/playback.rs`
- `src/validation.rs`
- `src/video_export.rs`
- `src/html_report.rs`
- `src/report.rs`
- `src/qa_report_defense.rs`
- `src/tool_policy.rs`
- `src/util.rs`
- `src/cli/media_cmd.rs`
- `tests/media_contract.rs`
- `docs/recovery-test-spec.md`

macOS executable: yes for CLI with ffmpeg/ffprobe if installed and for generated report/viewer proof. Windows-only blocker: Windows Media Player playback confirmation cannot be genuinely exercised on macOS.

### Group D: QA/release hardening and global release readiness

Primary files:

- `.github/workflows/windows-ci.yml`
- `src/qa.rs`
- `src/qa_release.rs`
- `src/qa_release_gates.rs`
- `src/qa_tests.rs`
- `src/package.rs`
- `docs/WINDOWS_RISK_REVIEW.md`
- `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md`
- `.omo/ulw-loop/*` evidence artifacts

macOS executable: partial, with negative Windows prereq and non-Windows release-blocking proof. Windows-only blocker: Windows CI and WinUI shell receipt.

### Group E: ULW evidence and plans

Primary files:

- `.omo/ulw-loop/frame-gui-20260617102845/`
- `.omo/ulw-loop/frame-media-validation-20260617024104/`
- `.omo/ulw-loop/frame-windows-prereq-gate-20260622/`
- `.omo/ulw-loop/frame-production-exec-20260623/`
- `.omo/plans/phase-3-media-validation-derived-audit-report.md`

Commit policy: include only evidence that directly supports the work unit being committed. Avoid sweeping all historical `.omo` sessions into one code commit unless the repo already treats those artifacts as release evidence for this branch.

## Recommended Sequential Execution

1. Run fresh full-tree validation before any staging. This detects whether the integrated dirty tree is self-consistent.
2. Run C002 Windows prerequisite negative readiness proof on macOS. This is the highest-value executable gate and should remain blocked-positive / pass-negative.
3. If full validation is green, make one integrated safety commit only if hunk-splitting shared files is too risky. Subject style: `Gate Windows release readiness on verified prerequisites` or broader `Harden forensic workstation release gates`.
4. If full validation is not green, fix only the failing executable slice, then rerun the same evidence.
5. Continue with GUI inventory browser proof and media/report CLI proof as separate work units.
6. Stop and record a true blocker when the only remaining proof is Windows-only WinUI build/test and `dotnet` receipt generation.

## Cleanup Receipt

No QA runtime, server, tmux session, browser context, container, bound port, or temp directory was spawned for this classification artifact.
cleanup: not-applicable; reason=read-only git/source classification only.
