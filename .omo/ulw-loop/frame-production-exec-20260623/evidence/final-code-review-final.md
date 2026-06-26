# Final Code Quality Review - FrameTrace ULW Final Gate

codeQualityStatus: WATCH
recommendation: APPROVE
reportPath: /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-code-review-final.md
blockers: []

Reviewed HEAD: 14eba5e Reject symlinked inventory export targets

Prior commits considered at high level for regression context:
- 541ec49 Block inventory exports from evidence paths
- a42a320 Harden forensic workstation release gates

## Skill Perspective Check

- `remove-ai-slops`: ran by loading `/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/remove-ai-slops/SKILL.md`. The diff does not violate this perspective: no deletion-only tests, no hollow tests, no tautological test that merely mirrors a constant, no unnecessary production parsing/normalization added by the final fix, and no overbroad helper abstraction.
- `programming`: ran by loading `/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/programming/SKILL.md`, `/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/programming/references/rust/README.md`, and `/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/programming/references/code-smells.md`. The diff does not violate this perspective: touched Rust files are under the 250 pure LOC ceiling, production error handling is explicit, and no untyped escape hatch or needless Rust abstraction was introduced.
- `code-review`: loaded `/Users/shinyoohag/.codex/skills/code-review/SKILL.md`. Independent subagent lanes were not spawned because the currently exposed subagent tool contract permits spawning only when explicitly requested by the user. This report is a direct read-only final-gate review.

## Evidence Reviewed

- `git status --short`
- `git log --oneline -5`
- `git show --stat --patch 14eba5e -- <touched files>`
- `git show --stat --patch 541ec49 -- <inventory export policy files>`
- `git show --stat --oneline a42a320`
- `git diff 541ec49..14eba5e -- <touched files>`
- Previous blocker report: `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-code-review-rerun.md`
- Line-numbered source/test inspection for:
  - `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs`
  - `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export_tests.rs`
  - `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_tests.rs`
  - `/Users/shinyoohag/Desktop/frametrace/src/case_db/mod.rs`
  - `/Users/shinyoohag/Desktop/frametrace/tests/cli_inventory.rs`
  - `/Users/shinyoohag/Desktop/frametrace/tests/cli_review.rs`
  - `/Users/shinyoohag/Desktop/frametrace/src/tool_policy.rs`

## Verification Run

- `cargo test --locked case_db::inventory_export_tests::export_manifest_rejects_dangling_symlink_output_without_creating_target -- --nocapture`: PASS, 1 passed.
- `cargo test --locked --test cli_inventory -- --nocapture`: PASS, 1 passed.
- `cargo test --locked --test cli_review -- --nocapture`: PASS, 2 passed.
- `cargo test --locked case_db::inventory_export_tests -- --nocapture`: PASS, 4 passed.
- `cargo test --locked case_db::inventory_tests -- --nocapture`: PASS, 5 passed.
- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: PASS.
- `cargo test --locked`: PASS, 110 lib tests, 12 integration tests, and 0 doc tests passed.
- Direct CLI surface probe for dangling symlink output:
  - Setup: create a real case, scan one media file, create `<case>/reports/dangling-output.json` as a symlink to an outside non-existent target, run `./target/debug/frametrace inventory-export-manifest <case> --operator qa --output <symlink> vid_000001`.
  - Result: `status=1`, `target_exists=no`, stderr contained `inventory manifest output cannot be a symlink`.
- LSP diagnostics: not run. The task states LSP diagnostics are known unavailable due MCP transport closed; Cargo, clippy, and tests passed, so this is recorded as a non-blocking limitation.

## Coverage Confirmation

- Ordinary outside output paths are covered by production containment at `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs:74` via `require_case_output_path`, with unit coverage at `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export_tests.rs:41` and CLI coverage at `/Users/shinyoohag/Desktop/frametrace/tests/cli_inventory.rs:169`.
- Registered source paths are covered by `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs:75` and `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs:98`, with unit coverage at `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export_tests.rs:119`.
- Existing output files are covered by `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs:80` and `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs:86`, with unit coverage at `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export_tests.rs:64` and CLI coverage at `/Users/shinyoohag/Desktop/frametrace/tests/cli_inventory.rs:192`.
- Dangling symlink final components are covered by `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs:80` through `symlink_metadata`, with unit coverage at `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export_tests.rs:84`, CLI coverage at `/Users/shinyoohag/Desktop/frametrace/tests/cli_inventory.rs:203`, and the direct CLI probe above.

## Pure LOC Check

- `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs`: 194 pure LOC.
- `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export_tests.rs`: 133 pure LOC.
- `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_tests.rs`: 159 pure LOC.
- `/Users/shinyoohag/Desktop/frametrace/src/case_db/mod.rs`: 168 pure LOC.
- `/Users/shinyoohag/Desktop/frametrace/tests/cli_inventory.rs`: 208 pure LOC.
- `/Users/shinyoohag/Desktop/frametrace/tests/cli_review.rs`: 97 pure LOC.

All touched source/test files are below the `programming` skill 250 pure LOC threshold. No exception is needed.

## Findings

### CRITICAL

None.

### HIGH

None.

### MEDIUM

None.

### LOW

1. `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs:36` and `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs:38`

   Issue: The output policy is still a pre-write validation followed by `write_text`; it is not an atomic create-new/no-follow write.

   Risk: This does not reproduce the prior P1 blocker, because an already supplied dangling symlink final component is now rejected before writing. A concurrent local actor with write access to the case tree could still theoretically swap the final component after validation and before the write.

   Recommendation: Treat this as future hardening unless the threat model includes hostile concurrent writers inside the case directory. If that threat model is in scope, replace pre-write validation plus `fs::write` with an atomic create-new/no-follow write path where the platform supports it.

## AI Slop Review

- Hollow tests: none found. The dangling symlink tests assert both the user-visible failure and that the outside target was not created.
- Redundant verification: none found in production. Test-side `target_exists=no` checks are meaningful because they prove the security regression did not occur.
- Overbroad helper abstraction: none found. `reject_existing_or_symlink_output_path` has one caller, but it isolates a distinct filesystem policy branch and keeps `manifest_output_path` readable.
- Negative naming: no blocking negative-form names. Domain terms such as `missing_ids` are legitimate result fields.
- Missing error handling: none found in production; `symlink_metadata` distinguishes symlink, existing non-symlink, not found, and other inspection errors.
- Pattern drift: none found. The final fix continues the existing case-output policy style while adding symlink-aware inspection.

## Final Decision

The previous P1 blocker is resolved. `inventory-export-manifest --output` no longer follows a dangling symlink final component outside the case; it fails closed and leaves the outside target absent. Ordinary outside paths, registered source paths, existing files, and dangling symlink paths are all covered by production code and tests.

APPROVE
