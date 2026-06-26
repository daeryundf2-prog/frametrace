# Final Code Review After Derived Symlink Fix

codeQualityStatus: WATCH
recommendation: APPROVE
reportPath: `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/reviews/final-code-review-after-derived-symlink-fix.md`
blockers: none

Reviewed range: `HEAD~2..HEAD`

Reviewed commits:
- `151fd5c` Treat symlinked generated outputs as occupied
- `a42c3af` Reject derived output symlink targets

Reviewed files:
- `/Users/shinyoohag/Desktop/frametrace/src/tool_policy.rs`
- `/Users/shinyoohag/Desktop/frametrace/src/lib.rs`
- `/Users/shinyoohag/Desktop/frametrace/src/derived_output_policy_tests.rs`
- `/Users/shinyoohag/Desktop/frametrace/src/util.rs`

## Skill Perspective Check

Ran before judging test relevance and maintainability:
- `remove-ai-slops`: loaded `/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/remove-ai-slops/SKILL.md`.
- `programming`: loaded `/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/programming/SKILL.md` and Rust reference `/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/programming/references/rust/README.md`.
- `code-review`: loaded `/Users/shinyoohag/.codex/skills/code-review/SKILL.md`.

Result: no blocking violation of either `remove-ai-slops` or `programming` perspectives was found. The new tests are behavior-shaped, not deletion-only, not tautological, and not limited to mirroring implementation constants. Production changes stay scoped to filesystem output policy; no needless extraction, parsing, normalization, untyped escape hatch, or speculative abstraction was introduced.

## Evidence

Diff inspected:
- `git diff --find-renames HEAD~2..HEAD -- src/tool_policy.rs src/lib.rs src/derived_output_policy_tests.rs src/util.rs`
- `git diff --check HEAD~2..HEAD`: PASS

Fresh verification:
- `cargo fmt --all -- --check`: PASS
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: PASS
- `cargo test --locked unique_path_treats_dangling_symlink_as_occupied -- --nocapture`: PASS, 1 passed
- `cargo test --locked symlink -- --nocapture`: PASS, 9 passed
- `cargo test --locked`: PASS, 117 lib tests plus integration tests passed

Pure LOC evidence:
- `src/lib.rs`: 29
- `src/tool_policy.rs`: 250
- `src/derived_output_policy_tests.rs`: 134
- `src/util.rs`: 223

The prior oversized touched-file blocker remains closed for this review range. `src/tool_policy.rs` is exactly at the 250 pure-LOC ceiling, not over it.

## Findings

### CRITICAL

None.

### HIGH

None.

### MEDIUM

None.

### LOW

1. `/Users/shinyoohag/Desktop/frametrace/src/tool_policy.rs:1`

   Issue: `src/tool_policy.rs` is exactly 250 pure LOC after the commit.

   Risk: This is not an oversized-file blocker under the provided evidence, but the next addition will cross the project ceiling unless the policy module is split by responsibility.

   Recommendation: Treat the next non-trivial policy addition as the point to extract tool-binary policy, case-output path policy, or tests into focused modules.

2. `/Users/shinyoohag/Desktop/frametrace/src/tool_policy.rs:104`

   Issue: The current fix rejects pre-existing symlink leaf paths before output creation, but output creation still remains a check-then-write flow at downstream writers.

   Risk: This does not reproduce the reviewed blocker, because a dangling symlink already present at validation time is rejected and generated default paths now avoid dangling symlink names. A concurrent local actor with write access to the case tree could still theoretically swap a path after validation and before the writer opens it.

   Recommendation: Non-blocking future hardening should use atomic create-new or platform no-follow behavior at the final open where practical.

## Test Relevance Review

The test additions are relevant and behavior-shaped:
- `src/derived_output_policy_tests.rs` calls the public derived-output surfaces (`generate_proxy`, `generate_thumbnail`, `capture_frame`, `export_video`, `recover_inode`) with dangling symlink output paths and verifies both the user-visible rejection and that the outside symlink target is not created.
- `src/tool_policy.rs` covers the shared case-output policy directly for a dangling symlink leaf.
- `src/util.rs` verifies `unique_path` treats a dangling symlink as occupied by selecting `clip_001.mp4` and leaving the outside target absent.

These tests would fail against the pre-fix behavior because `Path::exists()` treats dangling symlinks as absent. They are not hollow success checks, deletion-only tests, or tests that merely assert a removed code path is gone.

## Scope And Maintainability Review

`a42c3af` places the explicit-output policy at `require_case_output_path`, which is the shared boundary already used by proxy, thumbnail, frame capture, video export, inode recovery, E01 raw output, and inventory export paths. That keeps the fix centralized instead of patching each writer separately.

`151fd5c` updates `unique_path` occupancy semantics for generated default paths, which is the correct shared seam for generated outputs that would otherwise treat dangling symlinks as free names. The helper `path_is_available` is small and local to `unique_path`; it removes duplicated symlink-aware metadata checks inside the suffix loop.

No new blocker was found in `HEAD~2..HEAD`.

APPROVE
