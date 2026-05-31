# Cleanup Review

Phase 1 cleanup is being executed conservatively. The current codebase exposes most modules through `src/lib.rs` for tests and CLI wiring, so broad deletion is not safe until symbol usage is proven.

## Current Cleanup Decision

No large deletion pass has been performed yet. The first hardening slice focused on test-backed risk reduction rather than speculative removal.

## Symbol Handling Rules

| Classification | Action |
| --- | --- |
| `DELETE` | Remove only with usage evidence and tests. |
| `KEEP: CLI` | Required by `src/cli/mod.rs` or `src/cli/handlers.rs`. |
| `KEEP: TEST SUPPORT` | Required by current unit/integration tests. |
| `KEEP: COMPATIBILITY` | Required by JSON/TSV/report compatibility artifacts. |
| `INVESTIGATE` | Do not delete until the owner proves it is unused. |

## Initial Cleanup Candidates

| Candidate | Status | Reason |
| --- | --- | --- |
| Duplicated manual JSON helpers | `INVESTIGATE` | Used across scan, audit, validation, and ffprobe paths; replacement should be typed and test-backed. |
| `videos.record_json` compatibility payload | `KEEP: COMPATIBILITY` | Duplicates typed columns but supports existing JSONL/report flows. Candidate for schema audit, not immediate removal. |
| Public module exports in `src/lib.rs` | `INVESTIGATE` | Tests and CLI currently rely on broad module visibility. Narrowing should be done module-by-module. |

## Completed Cleanup-Safe Hardening

1. Added a shared JSON structural compacting helper for `ffprobe` payload safety.
2. Reused that helper in model serialization and ffprobe validation.
3. Added regression tests around package completeness, symlink rejection, and scan exclusion.
4. Added a shared external-tool/output-path policy module instead of duplicating ad hoc checks.
5. Extended existing CLI smoke coverage rather than adding a parallel validation harness.

## Validation

- `cargo fmt --check`: pass after formatting.
- `cargo test`: pass after QA/security/viewer integration slice.
- `cargo clippy --all-targets -- -D warnings`: pass after latest edits.
