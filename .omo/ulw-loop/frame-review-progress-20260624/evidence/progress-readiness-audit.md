# FrameTrace Progress and Readiness Audit

Session: `frame-review-progress-20260624`
Scope: read-only progress audit across implementation, docs, prior ULW state, release gates, and current executable checks.
Result: PARTIALLY READY. Do not claim GA or production-ready Windows workstation status.

## Current implemented baseline

### Completed / strong evidence

- Case folder creation and manifest fields are documented as MVP-complete in `docs/MVP_STATUS.md:5-55`.
- SQLite `case.db` is the primary video index according to `docs/MVP_STATUS.md:13-14`.
- E01 inspection/import, TSK inspection/recovery, carving, validation logs, and chained artifact logs are documented as MVP-complete in `docs/MVP_STATUS.md:15-51`.
- Workstation status declares Rust engine and SQLite/audit as source of truth in `src/workstation.rs:36-52`.
- Inventory summary declares `transport:"sqlite-bounded-query"` and `full_json_load_allowed:false` in `src/workstation.rs:97-104`.
- Validation summary separates ffprobe video-stream confirmation from playback confirmation in `src/workstation.rs:107-123`.
- WinUI durable mutation contract is machine-readable in `src/workstation.rs:210-217`.
- `qa release` enforces report defense, workstation shell contract, Windows prerequisites, review gates, accuracy, reproducibility, and performance in `src/qa_release.rs:20-64`.

### Fresh check results

- Formatting, clippy, Rust tests, JS syntax, git whitespace, and LSP diagnostics passed.
- `qa performance --rows 100000` passed.
- Empty-case `qa release` failed closed with 27 blockers, including Windows prerequisites, missing review manifest, missing corpus manifest, and missing comparison case.

## Partial / implementation exists but not enough for release claims

### HTML evidence viewer

Evidence:

- `gui/evidence-viewer/app.js:172-183` generates 10,000 mock rows in browser memory.
- `gui/evidence-viewer/index.html` and `styles.css` exist.

Assessment:

The HTML prototype is useful for UX review and virtualization design, but it is not the production workstation. It must remain a derived/prototype surface unless connected to bounded engine/SQLite queries.

### Large-case data path

Evidence:

- `src/workstation.rs:97-104` forbids full JSON loading for production inventory.
- `src/review_bundle.rs:110-119` supports bounded embedded review rows.
- `src/cli/handlers.rs:307-314`, `src/report.rs:306-323`, and `src/scan.rs:248-264` still contain full JSON/all-record report or compatibility paths.

Assessment:

Inventory query contract is progressing, but report and scan compatibility paths are not fully large-case safe.

### Release governance

Evidence:

- `src/qa_release.rs:31-51` blocks without corpus and comparison case.
- `src/qa_release.rs:225-229` accepts broad textual values such as `complete`, `done`, and `x` as review-gate pass values.

Assessment:

Release gates exist and fail closed on missing manifests, but review gate values should be backed by artifacts and typed gate schemas before being trusted for production readiness.

## Blocked / not proven

### WinUI 3 production shell

Evidence:

- `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md:5-10` states no real C#/WinUI shell was introduced and no buildable WinUI project exists.
- `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md:45-58` marks Phase 4 WinUI/Windows integration as FAIL.
- `scripts/windows/validate-release.ps1:55-67` throws if the WinUI directory or `.sln/.csproj` is missing.
- `scripts/windows/validate-release.ps1:119-121` requires `dotnet build` and `dotnet test`.
- `.github/workflows/windows-ci.yml:39-41` runs the Windows release validation smoke.
- `find gui -maxdepth 3 -type f` returned only `gui/evidence-viewer/app.js`, `index.html`, and `styles.css`.

Assessment:

There is no buildable `gui/winui` shell in this repo snapshot. Windows-native release readiness cannot be claimed.

### Windows validation

Evidence:

Current `workstation-status` on macOS reports:

- `host_os:"macos"`
- `release_validation_host_ready:false`
- `missing_tools:["dotnet"]`
- `winui_project_present:false`
- blockers: `unsupported-host`, `missing-tool:dotnet`, `missing-winui-project`

Assessment:

Fail-closed behavior is correct. Native Windows execution remains unproven.

### Real forensic corpus accuracy and reproducibility

Evidence:

- `src/qa_release.rs:31-51` blocks release when `--corpus-manifest` or `--comparison-case` is missing.
- Empty-case release check failed with `accuracy: missing --corpus-manifest` and `reproducibility: missing --comparison-case`.
- `docs/MVP_STATUS.md:57-71` explicitly does not claim automatic bulk deleted-file reconstruction, proprietary DVR/NVR recovery, report-defensible proprietary validation, signing, Windows GUI packaging, PDF report rendering, or ML analysis.

Assessment:

Synthetic and unit/integration checks are strong, but real corpus precision/recall, reproducibility, and report-defensible field evidence are not proven.

## Unknown / not inspected in this pass

- Native Windows runtime behavior is unknown because this audit ran on macOS and did not execute `scripts/windows/validate-release.ps1` on Windows.
- WinUI behavior is unknown because no `gui/winui` project files are present in this snapshot.
- Real-world E01/libewf/TSK workflows are unknown for field evidence because this pass did not run against a committed real validation corpus.
- Installer behavior is unknown because no Windows installer/package validation was executed.
- External legal/operator acceptance is unknown because this pass reviewed repository evidence only and did not perform a human legal review.
- Report performance on a real mixed 1M-file case is unknown; the fresh performance evidence is synthetic SQLite at 100k rows.

## Independent review synthesis

Code review lane:

- Recommendation: REQUEST CHANGES for production-readiness review.
- Primary issues: path privacy leakage, missing audit logs not blocking report-defense, full JSON report/scan paths, validation target trust, ffmpeg tool provenance.

Architecture lane:

- Status: WATCH.
- Summary: core architecture is coherent; Rust engine, SQLite, audit and bounded inventory are real; remaining gap is missing Windows-native WinUI shell and Windows validation receipt.

Test/validation lane:

- Verdict: PARTIAL.
- Strong local macOS-compatible Rust/CLI/HTML evidence; missing Windows/WinUI validation and real corpus accuracy/reproducibility.

## Readiness score

| Area | Score | Notes |
| --- | ---: | --- |
| Rust CLI/core tests | 85 | Current fmt/clippy/tests pass. |
| SQLite/audit/source-of-truth contract | 75 | Good structure; missing-audit gate and provenance resolution need hardening. |
| Large-case readiness | 55 | SQLite query and 100k synthetic pass, but report/scan paths still full-load. |
| GUI prototype | 55 | Useful prototype; not production WinUI. |
| Windows production shell | 15 | Contract exists; project/build receipt missing. |
| Accuracy/reproducibility defensibility | 35 | Gates exist; real corpus evidence missing. |
| Release readiness | 40 | Gates fail closed; blockers remain. |

Overall: 50/100, PARTIALLY READY.

## Final audit conclusion

FrameTrace has a credible hardened Rust/SQLite forensic core and strong fail-closed release posture. It is not finished as a production Windows forensic workstation. The next work should fix security/evidence-contract gaps first, then finish large-case safe report paths, then implement and validate WinUI on Windows, then run real corpus accuracy/reproducibility and release gates.
