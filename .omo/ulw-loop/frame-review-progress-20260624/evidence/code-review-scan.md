# FrameTrace Code Review Scan

Session: `frame-review-progress-20260624`
Scope: read-only review of Rust engine, SQLite case database, report/viewer, Windows release gates, media validation, derived artifacts, and prior evidence.
Result: REQUEST CHANGES for production-readiness claims. No code changes were made.

## Fresh verification

Commands run from `/Users/shinyoohag/Desktop/frametrace`:

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --locked` | PASS: 117 library tests plus integration suites passed |
| `node --check gui/evidence-viewer/app.js` | PASS |
| `git diff --check` | PASS |
| LSP diagnostics for `src/main.rs` | PASS: no diagnostics |
| `target/debug/frametrace qa performance /tmp/frametrace-review-performance-100k --rows 100000` | PASS: wrote `/tmp/frametrace-review-performance-100k/performance-report.json` |
| `target/debug/frametrace qa release /tmp/frametrace-review-gate-case --output-dir /tmp/frametrace-review-gate-case/reports/qa-review-audit` | EXPECTED FAIL-CLOSED: exit 1 with 27 blockers |

The fail-closed release check reported missing report artifacts, missing Windows/WinUI prerequisites, missing review manifest, missing corpus manifest, and missing comparison case.

## Findings

### HIGH: distributable report/viewer outputs expose full local paths

Evidence:

- `src/review_bundle.rs:129-140` serializes `source_path` and `file_url` from `row.full_path`.
- `src/report.rs:255-264` renders `scan.source_path` in the processing table.
- `src/report.rs:330-354` renders source/output paths for clip exports and derived artifacts.
- `gui/evidence-viewer/app.js:1114-1118` displays `record.path` directly in the prototype detail panel.

Risk:

Shared review bundles and reports can leak investigator workstation paths, client names, volume layout, mount paths, or case folder names.

Required next action:

Add default distributable redaction using source IDs and case-relative artifact paths. Full local paths should require explicit operator opt-in and an executable privacy review gate.

### HIGH: report-defense can pass while missing audit logs are not blockers

Evidence:

- `src/audit.rs:236-246` classifies missing audit logs as `AuditChainState::Missing`.
- `src/qa_report_defense.rs:28-33` only blocks on `tampered_audit_chain_messages`; missing audit logs are not included in the `passed` expression.

Risk:

A report can be treated as report-defensible even when validation, proxy, thumbnail, frame, carving, export, or filesystem-recovery audit logs are absent for surfaces the report presents.

Required next action:

Make missing audit logs blocking when corresponding artifacts, index rows, or validation/report claims exist. Add regression tests for report-present/audit-log-missing cases.

### HIGH: large-case contract is inconsistent across report and scan compatibility paths

Evidence:

- `src/workstation.rs:97-104` declares `transport:"sqlite-bounded-query"` and `full_json_load_allowed:false`.
- `src/cli/handlers.rs:307-314` still reads all `db/video_index.json` for `make-report`.
- `src/report.rs:306-323` maps all `videos` into the report table.
- `src/scan.rs:248-264` builds merged JSON/JSONL/TSV strings in memory.
- `src/review_bundle.rs:110-119` does cap review-bundle embedded rows, so the bounded pattern exists but is not universal.

Risk:

Very large cases can still exhaust memory in report or scan compatibility output paths even though the workstation contract rejects full JSON loads for production GUI.

Required next action:

Move report generation to SQLite-backed summaries and bounded appendices. Stream compatibility JSONL/TSV artifacts or make full compatibility exports explicit export-only operations.

### MEDIUM: validation target resolution trusts direct paths and manually parsed log fields

Evidence:

- `src/validation/target.rs:17-33` accepts any direct file path as a validation target.
- `src/validation/target.rs:79-99` extracts JSONL fields through manual string extraction.
- The resolver does not verify the relevant audit chain before trusting `output_path` or `output_artifact_path` fields.

Risk:

Poisoned, stale, or malformed logs can steer validation/provenance to unintended files. This weakens audit-chain defensibility.

Required next action:

Parse typed JSON records, verify the relevant audit chain before selector resolution, and require derived/recovered artifact paths to resolve under the case directory unless an explicit external-source mode is used.

### MEDIUM: ffmpeg execution bypasses the external-tool allowlist boundary

Evidence:

- `src/video_export.rs:100-109` calls `Command::new("ffmpeg")`.
- `src/artifacts.rs:102-106`, `src/artifacts.rs:164-168`, and `src/artifacts.rs:226-230` do the same for proxy, thumbnail, and frame capture.

Risk:

Derived artifacts depend on whichever `ffmpeg` appears first in `PATH`. That is weak provenance for forensic artifacts.

Required next action:

Route all ffmpeg calls through the same tool-policy resolver as other external tools. Record resolved binary path and version in audit logs. Add rejection tests for unapproved ffmpeg paths/names.

### MEDIUM: oversized Rust modules remain a maintainability risk

Evidence from pure LOC scan:

```text
897 src/scan.rs
888 src/html_report.rs
774 src/cli/handlers.rs
598 src/detector.rs
432 src/artifacts.rs
405 src/package.rs
397 src/report.rs
375 src/audit.rs
370 src/video_export.rs
340 src/carve.rs
265 src/model.rs
255 src/case_db/scan.rs
250 src/tool_policy.rs
```

Risk:

The current tests are green, but these files exceed the local programming discipline ceiling and will be hard to safely extend during WinUI, privacy, audit, and large-case work.

Required next action:

Refactor by responsibility before adding substantial new features to these files. Start with `src/cli/handlers.rs`, `src/scan.rs`, `src/html_report.rs`, and `src/artifacts.rs`.

## Positive evidence

- Rust engine/SQLite/audit boundaries are real, not only documented.
- `workstation-status` declares engine and SQLite/audit as source of truth.
- Candidate promotion separates ffprobe validation from playback confirmation.
- Release readiness gates fail closed when review/corpus/Windows prerequisites are missing.
- Synthetic 100k performance smoke passed on this host.
- No forbidden legal claim wording was found in positive report output paths during targeted scan; matches were denylist strings, test fixtures, or cautionary docs.

## Verdict

Codebase health for local Rust/CLI prototype: GREEN.

Production readiness: NOT READY.

Primary blockers: path privacy, missing-audit handling, full JSON/report paths, trusted provenance resolution, ffmpeg tool provenance, WinUI/Windows validation, and real corpus accuracy/reproducibility.
