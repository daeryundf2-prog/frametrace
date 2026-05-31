# Static Analysis Baseline

Baseline captured for Phase 0 of `docs/FORENSIC_HARDENING_PLAN.md`.

## Environment

- Date: 2026-05-31 00:22:33 KST
- Branch: `codex/frametrace-forensic-hardening`
- Baseline commit: `e23fd42a3569cb901448bae7f635ecaf113e49c7`
- Rust: `rustc 1.94.0 (4a4ef493e 2026-03-02)`
- Cargo: `cargo 1.94.0 (85eff7c80 2026-01-15)`
- Node: `v25.8.1`
- FFmpeg: `ffmpeg version 8.0.1`
- ffprobe: `ffprobe version 8.0.1`
- libewf CLI: `20140816`
- Sleuth Kit: `4.14.0`

## Baseline Commands

| Command | Result |
| --- | --- |
| `cargo fmt --check` | Pass |
| `cargo check --all-targets` | Pass |
| `cargo clippy --all-targets -- -D warnings` | Pass before hardening edits |
| `cargo test` | Pass before hardening edits: 49 library tests, 3 CLI smoke tests |
| `node --check gui/evidence-viewer/app.js` | Pass |

## Phase 0 Finding Summary

The baseline build was healthy. The hardening audit identified production-readiness risks rather than build-break failures:

1. Case output folders could be scanned as source evidence if a case directory lived below the selected source root.
2. Case packages could be created with missing required artifacts because fixed package files were skipped silently.
3. Recursive packaging did not explicitly reject symlinked inputs.
4. `ffprobe` JSON was embedded without fail-closed structural validation.
5. SQLite schema had no migration path beyond exact version matching.
6. Several report/viewer paths expose full source paths; privacy redaction remains open for later phases.
7. External binary paths remain permissive; stricter command allowlisting remains open for later phases.

## Remediation Tracking

| Finding | Target Phase | Status |
| --- | --- | --- |
| Case output re-scan contamination | Phase 4 | Fixed in first hardening slice |
| Silent incomplete package | Phase 4 | Fixed in first hardening slice |
| Package symlink traversal | Phase 2/4 | Fixed in first hardening slice |
| Invalid `ffprobe` JSON corrupts JSON index | Phase 4 | Fixed in first hardening slice |
| SQLite migration framework | Phase 4 | Fixed for v1->v2 |
| Full-path privacy exposure in distributable reports | Phase 2/8 | Pending |
| External tool binary allowlisting | Phase 2 | Fixed for user-configurable forensic tools |

## Current Verification Snapshot

After the Phase 2-8 executable slice:

| Command | Result |
| --- | --- |
| `cargo fmt --check` | Pass after formatting |
| `cargo check --all-targets` | Pass |
| `cargo test` | Pass: 62 library tests, 3 CLI smoke tests |
| `cargo clippy --all-targets -- -D warnings` | Pass |
| `node --check gui/evidence-viewer/app.js` | Pass |
| `cargo run -- qa performance ./target/frametrace-scale-validation --rows 100000` | Pass: 100000 rows, 5326 ms, 1126549 rows/minute |
