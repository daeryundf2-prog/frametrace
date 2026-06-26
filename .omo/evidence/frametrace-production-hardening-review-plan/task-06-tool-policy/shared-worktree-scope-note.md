T6 shared-worktree scope note

The repository was dirty before T6 began. The untracked file `src/validation/target_tests.rs` is part of the pre-existing T5 validation-target work and is referenced by `src/validation.rs`.

T6 did modify `src/validation.rs` to resolve `ffprobe` through external tool policy before validation, but T6 did not create, edit, stage, delete, or revert `src/validation/target_tests.rs`.

Verification impact:
- `cargo test --locked` passes with the shared worktree as-is; see `verify-full-cargo-test-after-ffprobe.log`.
- This untracked file is not a T6 deliverable, but removing or ignoring it would break unrelated T5 work and violates the shared-worktree instruction.
- T6 patch artifacts therefore include T6 diffs plus `git-status-short-after-ffprobe.txt` rather than absorbing unrelated untracked T5 files.
