# FrameTrace Final Security Review

Commit reviewed: `14eba5eb9c8b469f143980b9fa0eee9a11ecc6ac`

Scope: read-only security final gate for inventory export path safety, original evidence immutability, symlink handling, export surfaces, derived output policy, and audit/report claims. Windows/WinUI native validation was treated as host-specific and out of scope.

## Verdict

BLOCKED

The inventory manifest fix itself addresses the prior dangling-symlink final-output bug, but the release gate is blocked because neighboring derived-output surfaces still allow the same class of dangling symlink escape.

## Findings

[HIGH] Derived artifact outputs can still follow a dangling final symlink and write outside the case directory.

Files:
- `/Users/shinyoohag/Desktop/frametrace/src/artifacts.rs:81`
- `/Users/shinyoohag/Desktop/frametrace/src/artifacts.rs:84`
- `/Users/shinyoohag/Desktop/frametrace/src/artifacts.rs:142`
- `/Users/shinyoohag/Desktop/frametrace/src/artifacts.rs:145`
- `/Users/shinyoohag/Desktop/frametrace/src/artifacts.rs:203`
- `/Users/shinyoohag/Desktop/frametrace/src/artifacts.rs:206`
- `/Users/shinyoohag/Desktop/frametrace/src/video_export.rs:57`
- `/Users/shinyoohag/Desktop/frametrace/src/video_export.rs:60`
- `/Users/shinyoohag/Desktop/frametrace/src/tsk.rs:270`
- `/Users/shinyoohag/Desktop/frametrace/src/tsk.rs:293`
- `/Users/shinyoohag/Desktop/frametrace/src/tool_policy.rs:78`

Issue: `require_case_output_path` validates the case root and nearest existing parent, but it does not reject a symlink final path. The derived output call sites then use `output_path.exists()` as their final leaf guard. A dangling symlink returns false for `exists()`, so `make-proxy`, `make-thumbnail`, `capture-frame`, `export-video`, and `recover-inode` can accept a symlink located under the case tree whose target is outside the case tree. The subsequent ffmpeg output or `File::create` follows that symlink and creates the outside target.

Reproduction evidence: a temp-only CLI run created a valid case and video, made `/tmp/.../case/artifacts/proxies/dangling-proxy.mp4` a dangling symlink to `/tmp/.../outside-proxy.mp4`, then ran:

`./target/debug/frametrace make-proxy "$case_dir" vid_000001 --output "$link" --operator qa`

Observed result:
- command status: `0`
- stdout: `proxy generated`
- outside target existed after command: `yes`
- case output path remained a symlink: `yes`

Fix: centralize the inventory-style final-leaf rejection in `tool_policy`, apply it to all output-producing surfaces, and avoid `Path::exists()` as a security check. Prefer atomic creation semantics such as `OpenOptions::create_new(true)` plus platform no-follow handling where available, and add regressions for dangling symlink outputs on proxy, thumbnail, frame capture, video export, and inode recovery paths.

## Inventory Manifest Assessment

Inventory-specific path safety passes this review.

Evidence:
- `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs:74` calls the shared case-root output policy before writing.
- `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs:75` rejects outputs that resolve to registered source evidence paths.
- `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs:80` through `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs:95` uses `std::fs::symlink_metadata` and rejects both existing files and symlink final paths, including dangling symlinks.
- `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export_tests.rs:82` through `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export_tests.rs:112` covers the dangling symlink regression and asserts the outside target is not created.
- `/Users/shinyoohag/Desktop/frametrace/tests/cli_inventory.rs:203` through `/Users/shinyoohag/Desktop/frametrace/tests/cli_inventory.rs:222` covers the same behavior through the CLI surface.
- `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export_tests.rs:118` through `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export_tests.rs:147` verifies registered source evidence cannot be targeted as the inventory manifest output.

No path traversal issue was found in the inventory manifest path after `require_case_output_path` plus `reject_existing_or_symlink_output_path`; the prior exploit path using a pre-existing dangling symlink final output is closed for `inventory-export-manifest`.

## Neighboring Policy Assessment

The shared parent/root policy is directionally correct but incomplete for output leaves:

- `/Users/shinyoohag/Desktop/frametrace/src/tool_policy.rs:83` through `/Users/shinyoohag/Desktop/frametrace/src/tool_policy.rs:104` canonicalizes the case root and nearest existing parent, rejecting parents that resolve outside the case directory.
- `/Users/shinyoohag/Desktop/frametrace/src/tool_policy.rs:107` through `/Users/shinyoohag/Desktop/frametrace/src/tool_policy.rs:129` rejects direct source-evidence output targets when canonicalization can prove equality.
- `/Users/shinyoohag/Desktop/frametrace/src/tool_policy.rs:205` through `/Users/shinyoohag/Desktop/frametrace/src/tool_policy.rs:245` has tests for outside-case and source-evidence rejection.

The missing shared final-leaf symlink policy is what lets derived outputs diverge from the now-hardened inventory manifest behavior.

## Validation

- `cargo test inventory_export --lib`: passed, 4 inventory export tests.
- `cargo test --test cli_inventory --test cli_review`: passed, 3 integration tests.
- `cargo test tool_policy --lib`: passed, 5 output/tool policy tests.
- `cargo check --all-targets`: passed.
- LSP diagnostics were attempted for all changed files, but the daemon/rust-analyzer backend was unavailable. This matches the gate's stated non-blocking condition and was replaced with compile/test validation.
- Static/security search found no hardcoded secret patterns in `src` or `tests`.

BLOCKED
