# FrameTrace Ordered Remaining Steps

Session: `frame-review-progress-20260624`
Purpose: executable next-stage sequence after code/progress review.
Constraint: do not claim production-ready or GA until all release blockers pass with artifacts.

## Phase 1 - Security and privacy hardening

Owner: security engineer + backend engineer.

Tasks:

1. Add distributable redaction mode for report/viewer/package output.
2. Replace full local paths in shared outputs with source IDs and case-relative artifact paths by default.
3. Add explicit opt-in mode for local full-path disclosure.
4. Make `privacy_review` an executable QA gate with artifact checks.

Validation:

- New tests proving shared reports omit full source paths by default.
- New CLI/report QA output proving redaction status and opt-in status.

Evidence:

- `reports/qa/privacy-review.json`
- `reports/qa/report-redaction-checklist.md`
- focused Rust tests for report/viewer redaction.

Blocker removed:

- HIGH path leakage in report/viewer outputs.

## Phase 2 - Audit-chain completeness and report-defense hardening

Owner: forensic engineer + test engineer.

Tasks:

1. Treat missing audit logs as blockers when reports or artifact/index rows depend on them.
2. Add typed audit-chain status policy: missing, empty, valid, tampered, unsupported, not-applicable.
3. Add regression tests for report-present/log-missing and validation-claim/log-missing.
4. Ensure report-defense output distinguishes failed, skipped, partial, unsupported, and not-applicable.

Validation:

- `cargo test --locked qa_report_defense audit`
- `frametrace qa report-defense <case_dir>` on cases with and without expected logs.

Evidence:

- `reports/qa/report-defense-checklist.md`
- `reports/qa/audit-chain-status.json`

Blocker removed:

- HIGH report-defense can pass with missing required audit logs.

## Phase 3 - Large-case safe report and compatibility output path

Owner: database engineer + backend engineer + performance engineer.

Tasks:

1. Move `make-report` off full `db/video_index.json` reads.
2. Use SQLite-backed bounded summaries and appendices.
3. Stream JSONL/TSV compatibility artifacts instead of constructing full strings in memory.
4. Make full compatibility export explicit and operator-triggered, not a default prerequisite for report/package readiness.
5. Add tests for 100k and 1M row behavior without full browser JSON load.

Validation:

- `frametrace qa performance <out> --rows 100000`
- 1M row performance profile on target hardware.
- Memory/RSS evidence for report generation.

Evidence:

- `reports/qa/performance-report.json`
- `reports/qa/large-case-report-generation.json`

Blocker removed:

- HIGH large-case contract mismatch between workstation status and report/scan paths.

## Phase 4 - Provenance-safe validation target resolution

Owner: forensic engineer + security engineer.

Tasks:

1. Replace manual JSON string extraction in validation target resolution with typed JSON parsing.
2. Verify the relevant audit chain before trusting log-derived output paths.
3. Require recovered/derived artifact paths to resolve under the case directory by default.
4. Add explicit external-source validation mode for direct paths.

Validation:

- Regression tests for poisoned JSONL, stale log entries, external direct path, and case-contained derived artifact.
- `cargo test --locked validation`.

Evidence:

- focused validation target test output.
- `reports/qa/provenance-resolution-checklist.md`.

Blocker removed:

- MEDIUM poisoned/stale logs can influence validation target resolution.

## Phase 5 - External tool provenance unification

Owner: backend engineer + forensic engineer.

Tasks:

1. Route all ffmpeg calls through the external tool policy resolver.
2. Record resolved tool path, version, command args, operator, source artifact, output artifact, and output hash.
3. Reject disallowed ffmpeg names or paths.
4. Update export/proxy/thumbnail/frame capture tests.

Validation:

- Tests with approved and rejected ffmpeg paths.
- Real ffmpeg smoke workflow for export, proxy, thumbnail, and frame capture.

Evidence:

- artifact logs with `tool_path`, `tool_version`, `command_args`, `entry_sha256`.
- `reports/qa/tool-policy-checklist.md`.

Blocker removed:

- MEDIUM PATH-dependent ffmpeg provenance.

## Phase 6 - Module-size and maintainability refactor

Owner: architect + backend engineer.

Tasks:

1. Split `src/cli/handlers.rs` by command family.
2. Split `src/scan.rs` into detection, indexing, compatibility export, and SQLite write paths.
3. Split `src/html_report.rs` and `src/report.rs` into data preparation and rendering.
4. Split `src/artifacts.rs` into proxy, thumbnail, frame, and shared derived-artifact helpers.
5. Keep behavior locked with existing tests before moving code.

Validation:

- `cargo fmt --all -- --check`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `cargo test --locked`
- LOC report showing high-priority modules below agreed ceiling or marked with justified `SIZE_OK`.

Evidence:

- `reports/qa/module-size-report.md`
- refactor-only test logs.

Blocker removed:

- Maintainability risk before WinUI and release hardening work.

## Phase 7 - Real validation corpus and forensic metrics

Owner: QA owner + forensic engineer.

Tasks:

1. Prepare non-client validation corpora with source hashes and ground truth.
2. Cover deleted file recovery, browser/video artifacts, timeline reconstruction, large evidence, and mixed real-world-like cases.
3. Run accuracy, false-positive, false-negative, and reproducibility checks.
4. Archive corpus manifests and reports.

Validation:

- `frametrace qa accuracy <case_dir> <corpus_manifest>`
- `frametrace qa reproducibility <case_dir_a> <case_dir_b>`

Evidence:

- `reports/qa/accuracy-report.json`
- `reports/qa/reproducibility-report.json`
- corpus manifest with hashes and expected outputs.

Blocker removed:

- Missing real corpus accuracy/reproducibility proof.

## Phase 8 - Windows engine validation before WinUI implementation

Owner: Windows engineer + release manager.

Tasks:

1. Run Rust engine on Windows 10/11 x64 with MSVC.
2. Validate path handling, Unicode, long paths, file locks, case folder layout, ffmpeg/libewf/TSK discovery.
3. Run synthetic MP4 workflow and E01/raw image workflows on Windows.
4. Capture `workstation-status` and release preflight artifacts.

Validation:

```powershell
cargo fmt --check
cargo check --locked
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

Evidence:

- `reports/qa/windows-prerequisites.json`
- `reports/qa/workstation-status.json`
- Windows CLI workflow transcripts.

Blocker removed:

- Native Windows engine behavior unproven.

## Phase 9 - WinUI 3 production shell implementation

Owner: frontend/WinUI engineer + backend engineer.

Entry criteria:

- Phases 1-8 complete.
- Engine commands and SQLite query contracts stable.
- Windows engine validation passes.

Tasks:

1. Create `gui/winui` solution and test project.
2. Build WinUI shell as a client only: no durable state outside Rust/SQLite/audit.
3. Use engine/SQLite bounded inventory queries, not full JSON loads.
4. Implement case open, inventory list, search/facet/sort, details, validation/progress, audit-chain display, report/package actions.
5. Add Korean-first examiner workflow and dense file-list UX.
6. Produce `reports/qa/winui-build.json`.

Validation:

```powershell
dotnet build gui/winui/<project>.sln -c Release
dotnet test gui/winui/<tests>.csproj -c Release
scripts/windows/validate-release.ps1 -CaseRoot <case-root>
```

Evidence:

- `reports/qa/winui-build.json`
- WinUI screenshot/action-log artifacts.
- Windows release validation transcript.

Blocker removed:

- Missing final Windows shell.

## Phase 10 - Installer, packaging, and dependency distribution

Owner: release manager + security engineer.

Tasks:

1. Package Rust binary, WinUI shell, and required external tool discovery guidance.
2. Decide bundle-vs-discovery policy for ffmpeg, libewf, and Sleuth Kit.
3. Add checksums, SBOM/license notes, and package verification.
4. Add signing/timestamping if required by release policy.

Validation:

- Clean Windows VM install test.
- Installer uninstall/reinstall test.
- Package manifest checksum verification.

Evidence:

- `packages/package-manifest.json`
- `reports/qa/installer-package-validation.json`
- SBOM/license artifact.

Blocker removed:

- No installable Windows workstation package.

## Phase 11 - Full release gate and external review

Owner: release manager + reviewer + legal wording reviewer.

Tasks:

1. Run full `qa release` with review manifest, corpus manifest, comparison case, performance output, Windows prerequisite receipt, and WinUI receipt.
2. Complete technical, security, privacy, supply-chain, operator, legal wording, support, hotfix, incident-response, and known-limitations reviews.
3. Archive all release artifacts.
4. Keep wording to `report-defensible`; do not claim legal proof or guaranteed admissibility.

Validation:

```bash
frametrace qa release <case_dir> \
  --corpus-manifest <corpus_manifest> \
  --comparison-case <case_dir_b> \
  --review-manifest <release_review_manifest> \
  --performance-output-dir <performance_out> \
  --performance-rows 100000
```

Evidence:

- `reports/qa/release-readiness.json`
- `reports/qa/release-readiness.md`
- complete review manifest with artifact links.

Exit criteria:

- Every release blocker is PASS with artifact evidence.
- No unsupported Windows/WinUI or legal-readiness claim remains.
- Known limitations are present in report/release notes.

## Recommended immediate next sprint

1. Fix path privacy/redaction.
2. Fix missing-audit report-defense logic.
3. Fix large-case report/scan full-load paths.
4. Route ffmpeg through external tool policy.
5. Refactor the largest modules only after behavior is pinned.

This order removes the highest production-readiness risks before spending effort on WinUI.

## Explicit unknowns to resolve during execution

- Native Windows behavior: requires a Windows 10/11 x64 host run.
- WinUI shell behavior: requires `gui/winui` implementation and build/test receipt.
- Real corpus precision/recall: requires prepared validation corpus and ground truth.
- Real large-case survival: requires mixed evidence workload beyond synthetic SQLite rows.
- Installer/package behavior: requires clean Windows VM install/uninstall validation.
