# T6 FrameTrace Tool Policy Security/Size Code Review

codeQualityStatus: BLOCK
recommendation: REQUEST_CHANGES
verdict: FAIL
reportPath: `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy-code-review.md`

## Scope And Evidence

- Reviewed repository: `/Users/shinyoohag/Desktop/frametrace`
- Scoped files inspected: `src/tool_policy.rs`, `src/tool_policy/execution.rs`, `src/tool_policy/output_paths.rs`, `src/tool_policy/tests.rs`, `tests/tool_policy_api.rs`, `src/artifacts.rs`, `src/video_export.rs`, `src/validation/log.rs`.
- Evidence inspected: `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/`, including `evidence-record.md`, `t6-security-size-final.diff`, focused/final test logs, prior `code-review.md`, `security-review.md`, `qa-review.md`, and status artifacts.
- Fresh verification run: `cargo test --locked --test tool_policy_api -- --nocapture` PASS; `cargo test --locked resolves_approved_ffmpeg_path_and_rejects_disallowed_path_name --lib -- --nocapture` PASS; focused ffmpeg/ffprobe CLI tests PASS; scoped `git diff --check` PASS; direct `Command::new("ffmpeg"/"ffprobe")` search found no matches.

## Skill Perspective Check

- `code-review` skill loaded. Its independent subagent lane instructions could not be applied in this tool surface, so this is a direct read-only reviewer report under the active code-quality-reviewer role.
- `omo:remove-ai-slops` loaded and applied as the overfit/slop pass. The diff violates this perspective because scoped production files remain over the 250 pure-LOC ceiling and the final size evidence only measures the newly split `tool_policy` module.
- `omo:programming` loaded with the Rust reference. The diff violates this perspective on the same >250 pure-LOC rule and carries API-design risk from new required public struct fields.

## CRITICAL

None.

## HIGH

1. `src/artifacts.rs:1`, `src/video_export.rs:1`, `src/validation/log.rs:1` - The scoped files still violate the explicit `>250` pure-LOC size gate.
   - Evidence: fresh counts are `src/artifacts.rs` 474, `src/video_export.rs` 421, and `src/validation/log.rs` 267 pure LOC. `src/validation/log.rs` crossed the threshold from 222 to 267 in this change; the other two were already oversized and grew further.
   - Risk: This fails the user's "no oversized files (>250 pure LOC)" criterion and the loaded programming/remove-ai-slops rules. The evidence file `verify-tool-policy-size.log` reports only the split `src/tool_policy*` files, so it gives false confidence for the full requested review scope.
   - Required fix: Split or reduce the oversized scoped files, or explicitly document a justified size exception accepted by the plan/reviewer. At minimum, the size evidence must measure every scoped file.

2. `src/tool_policy.rs:1` - The build depends on new source files that are still untracked.
   - Evidence: `git status --short -- <scoped files>` shows `?? src/tool_policy/execution.rs`, `?? src/tool_policy/output_paths.rs`, `?? src/tool_policy/tests.rs`, and `?? tests/tool_policy_api.rs`. `src/tool_policy.rs` declares `mod execution; mod output_paths; mod tests;`, so omitting these files from a commit/patch breaks the build.
   - Risk: The current working tree passes tests, but the normal tracked diff does not carry the files required by the module split and API-forgery test. This is a delivery-integrity blocker before approval.
   - Required fix: Add these files to the actual change set and make the final diff artifact a real complete patch, not only a tracked diff plus informal untracked-file sections.

3. `src/artifacts.rs:11`, `src/artifacts.rs:30`, `src/artifacts.rs:49`, `src/video_export.rs:35` - Public option structs gained required public fields.
   - Evidence: `ProxyOptions`, `ThumbnailOptions`, `FrameCaptureOptions`, and `ExportOptions` now require `ffmpeg_bin: String` in struct literals. `frametrace::artifacts` and `frametrace::video_export` are publicly exported from `src/lib.rs`.
   - Risk: Downstream path dependencies using struct literals will fail to compile. The crate is `publish = false`, so this may be acceptable internally, but it is still API drift and this task explicitly asked for Rust API design/no behavior drift review.
   - Required fix: Either provide a compatibility-preserving construction path and avoid breaking literals, or document the intentional API break and add a downstream compile test that proves the intended external construction pattern.

## MEDIUM

1. `src/tool_policy/execution.rs:169`, `src/tool_policy/execution.rs:185`, `src/tool_policy/tests.rs:21`, `tests/cli_default_output_policy.rs:1`, `tests/cli_e01_validation_log_output_policy.rs:1` - Windows executable behavior is implemented but not directly covered.
   - Risk: The policy accepts `.exe` names and has Windows candidate lookup, but the new fake-tool policy tests are Unix-gated. This leaves residual risk for Windows command resolution.
   - Recommendation: Add Windows-specific unit coverage for `ffmpeg.exe` / `ffprobe.exe` name matching and PATH resolution, or record it as a release-gate gap.

2. `src/tool_policy.rs:7`, `src/tool_policy/execution.rs:120` - `run_external_tool` is re-exported publicly.
   - Risk: `ResolvedExternalTool` is now non-forgeable, and the fresh API-forgery test passes, but the public module still exposes a low-level process-running primitive to downstream crates.
   - Recommendation: If the policy surface is intended for internal enforcement rather than a public library utility, make `run_external_tool` `pub(crate)` and keep public APIs at resolver/logging boundaries.

## LOW

1. `src/artifacts.rs:459`, `src/video_export.rs:418`, `src/validation/log.rs:248` - Test-only fake tool construction is duplicated and executes a copied test binary just to create `ResolvedExternalTool`.
   - Risk: Low. It works today, but it is noisy test setup and makes log-body unit tests depend on process execution instead of a small internal fixture.
   - Recommendation: Consider a `#[cfg(test)]` constructor inside `tool_policy` if these tests remain in-file after the size split.

## Blockers

- Oversized scoped files remain above 250 pure LOC, with incomplete size evidence.
- Required module/test files for the split are untracked and can be omitted from the actual patch/commit.
- Public option structs were changed in a source-breaking way without an explicit compatibility decision or downstream construction test.

## Final Assessment

The core ffmpeg/ffprobe security behavior is present in the current working tree: focused tests pass, direct command launches are centralized, and `ResolvedExternalTool` cannot be forged by a downstream crate. Approval is still blocked by size-gate failure, delivery integrity, and unacknowledged Rust API drift.
