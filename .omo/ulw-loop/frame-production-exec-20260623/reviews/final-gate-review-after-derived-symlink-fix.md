# Final Gate Review After Derived Symlink Fix

recommendation: BLOCKED

## originalIntent

Complete the FrameTrace production-hardening checkpoint after the derived-output symlink blocker fix. The expected result is a final gate decision for current `HEAD`, including commits `a42c3af` and `151fd5c`, with both symlink evidence files inspected and no unresolved review blocker left.

## desiredOutcome

The acceptable user-visible outcome is a clean quality gate: current `HEAD` is the intended symlink-hardened state, the two new evidence files are present and non-empty, their recorded commands passed, cleanup receipts exist, the latest commits are atomic, no touched file exceeds 250 pure LOC, and every prior blocking review artifact is either superseded by a current approval or no longer applicable with explicit evidence.

## userOutcomeReview

Current `HEAD` is `151fd5c Treat symlinked generated outputs as occupied`, directly after `a42c3af Reject derived output symlink targets`.

The implementation evidence for the two symlink fixes is strong. `a42c3af` centralizes dangling final-leaf symlink rejection in `require_case_output_path`, and the derived-output tests cover proxy, thumbnail, frame capture, video export, and inode recovery. `151fd5c` makes `unique_path` treat dangling symlinks as occupied so default generated-output path selection does not reuse a symlink leaf. The targeted tests are behavioral rather than tautological: they assert rejection or alternate path selection and prove outside symlink targets are not created.

Fresh verification on this review passed:

- `cargo test --locked symlink -- --nocapture`: PASS, 9 symlink tests.
- `cargo test --locked unique_path_treats_dangling_symlink_as_occupied -- --nocapture`: PASS.
- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: PASS.
- `cargo test --locked`: PASS, including 117 lib tests and all integration/doc tests.
- `git diff --check`: PASS.

Evidence files are present and non-empty:

- `derived-output-symlink-policy-fix.txt`: 17,672 bytes.
- `unique-path-symlink-policy-fix.txt`: 16,495 bytes.

Cleanup receipts exist in both evidence files. The derived-output evidence records no spawned server/browser/tmux/container and cargo/test temp dirs removed by tests. The unique-path evidence records no spawned server/browser/tmux/container, cargo commands exited, unique-path symlink test temp dirs removed, and no worker left open.

Atomicity and size checks pass for the implementation commits:

- `a42c3af`: one focused shared output-policy hardening commit touching `src/tool_policy.rs`, `src/derived_output_policy_tests.rs`, and `src/lib.rs`.
- `151fd5c`: one focused generated-output occupancy commit touching only `src/util.rs`.
- Pure LOC at current `HEAD`: `src/lib.rs` 29, `src/tool_policy.rs` 250, `src/derived_output_policy_tests.rs` 134, `src/util.rs` 223. No touched file is greater than 250 pure LOC.

Direct `remove-ai-slops` / `programming` pass found no unresolved slop in these two commits: no deletion-only tests, no tests that merely verify a requested removal, no tautological constant pinning, no implementation-mirroring mock checks, no unnecessary production parsing/normalization, no oversized touched file, and no speculative abstraction. `path_is_available` is used at both initial and suffix candidate checks and encodes the filesystem occupancy rule that `Path::exists()` could not express for dangling symlinks.

The gate still cannot approve because the required current review coverage is missing. The existing unconditional code-review approval, `final-code-review-final.md`, reviews `14eba5e`, not `a42c3af` or `151fd5c`. The security review artifact, `final-security-review-final.md`, still ends `BLOCKED` on the derived-output symlink class. I found no later code-review or security-review report that explicitly approves current `HEAD` and covers the required `remove-ai-slops` overfit/slop criteria plus `programming` criteria for commits `a42c3af` and `151fd5c`.

## blockers

1. Missing current unconditional code-review artifact for `a42c3af..151fd5c`.
   Evidence: `final-code-review-final.md` says `Reviewed HEAD: 14eba5e` and approves the earlier inventory-export symlink fix. It does not cover the derived-output shared policy commit or the unique-path occupancy commit.

2. Existing security-review blocker has no superseding security approval artifact.
   Evidence: `final-security-review-final.md` ends `BLOCKED` because derived artifact outputs could follow dangling symlink leaves. The code and tests now appear to fix that, but the artifact set still lacks a current security review clearing the blocker for `HEAD` `151fd5c`.

3. Required report coverage for the final diff is unsupported.
   Evidence: no current review report explicitly covers `remove-ai-slops` overfit/slop checks or `programming` size/boundary checks for `src/tool_policy.rs`, `src/derived_output_policy_tests.rs`, `src/lib.rs`, and `src/util.rs` at `151fd5c`.

## checkedArtifactPaths

- `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/derived-output-symlink-policy-fix.txt`
- `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/unique-path-symlink-policy-fix.txt`
- `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-code-review-final.md`
- `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-code-review-rerun.md`
- `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-security-review-final.md`
- `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-gate-review-final.md`
- `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-goal-review-final.md`
- `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-review-final.md`
- `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-derived-symlink-fix/cargo-test-locked-head-151fd5c.txt`
- `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-derived-symlink-fix/cargo-test-symlink-nocapture-head-151fd5c.txt`
- `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-derived-symlink-fix/cargo-test-unique-path-dangling-symlink.txt`
- `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-derived-symlink-fix/git-diff-check-head-151fd5c.txt`
- `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/goals.json`
- `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/ledger.jsonl`
- `/Users/shinyoohag/Desktop/frametrace/src/tool_policy.rs`
- `/Users/shinyoohag/Desktop/frametrace/src/derived_output_policy_tests.rs`
- `/Users/shinyoohag/Desktop/frametrace/src/lib.rs`
- `/Users/shinyoohag/Desktop/frametrace/src/util.rs`
- `/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/remove-ai-slops/SKILL.md`
- `/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/programming/SKILL.md`
- `/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/programming/references/rust/README.md`
- `/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/programming/references/code-smells.md`

## exactEvidenceGaps

- No current code-review report approving `a42c3af` and `151fd5c`.
- No current security-review report clearing `final-security-review-final.md`'s derived-output symlink blocker after `a42c3af`.
- No current review artifact explicitly covering `remove-ai-slops` criteria for the final two commits: excessive/useless tests, deletion-only tests, tautological tests, implementation-mirroring tests, unnecessary production extraction/parsing/normalization, and maintenance burden.
- No current review artifact explicitly covering `programming` criteria for the final two commits: strict Rust boundary handling, no unsupported escape hatches, and no touched file over 250 pure LOC.
- LSP diagnostics remain unavailable in the evidence due MCP transport closure. This is recorded as a validation gap, not the blocking reason, because `fmt`, `clippy -D warnings`, and `cargo test --locked` passed freshly.

BLOCKED
