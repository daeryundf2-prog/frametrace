# FrameTrace T6 Tool Policy Code Review

codeQualityStatus: WATCH
recommendation: APPROVE
verdict: PASS
reportPath: `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/code-review.md`

## Scope Reviewed

- Repo: `/Users/shinyoohag/Desktop/frametrace`
- Diff artifact: `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/t6-diff-after-ffprobe.patch`
- Evidence inspected: `evidence-record.md`, `verify-full-cargo-test-after-ffprobe.log`, `verify-clippy-after-ffprobe.log`, `verify-cli-ffprobe-policy.log`, `verify-cli-ffmpeg-policy-after-ffprobe.log`, `manual-happy-log-summary.json`, `manual-happy-transcript.log`, `manual-failure-transcript.log`, `manual-failure-command.stderr`, and `shared-worktree-scope-note.md`.
- T6 review focus: external tool policy, ffmpeg/ffprobe fail-closed behavior, resolved metadata logging, tests, slop/overfit, and Rust discipline.
- Shared-worktree note applied: `src/validation/target_tests.rs` is an untracked pre-existing T5 file referenced by `src/validation.rs`; I did not treat its untracked state as a T6 blocker.

## Skill Perspective Checks

- `omo:remove-ai-slops`: loaded and applied to T6 production/test changes. Result: no deletion-only tests, tautological removal-only tests, implementation-only constant mirrors, or unnecessary production extraction/parsing/normalization that blocks T6.
- `omo:programming`: loaded, including Rust references for CLI/error/lint/test discipline. Result: no T6-owned blocker found. No unsafe code, no production `unwrap`/`expect`, no direct `Command::new("ffmpeg")` or `Command::new("ffprobe")`, and fresh `cargo clippy -D warnings` plus full `cargo test --locked` passed. Pre-existing oversized touched modules remain a WATCH item, not a T6 approval blocker.
- `code-review`: loaded. Native independent subagent lanes were unavailable in this tool surface, so this is a direct read-only reviewer report with fresh local verification.

## CRITICAL

None.

## HIGH

None.

## MEDIUM

None.

## LOW

1. `tests/cli_default_output_policy.rs:126`
   Issue: The focused ffmpeg CLI test proves approved resolved-tool metadata for `make-proxy`, `make-thumbnail`, and `capture-frame`, then proves rejected `export-video` fails closed. Approved `export-video` resolved metadata is covered by `src/video_export.rs` unit coverage plus manual happy evidence instead of this same CLI test.
   Risk: Low. The manual happy artifact shows `export-video` writes `resolved_tool_path`, `tool_version`, `output_artifact_sha256`, and `entry_sha256`, and the full suite passes. A future edit would be better protected by adding approved `export-video` to the CLI loop.
   Recommendation: Strengthen the CLI test when this area is touched again.

2. `src/artifacts.rs`, `src/video_export.rs`, `src/tool_policy.rs`
   Issue: These T6-touched production files are already over the `omo:programming` 250 pure-LOC review threshold.
   Risk: Low for T6 because the policy change is localized and verified; this is inherited structural debt.
   Recommendation: Track a later responsibility split rather than expanding these files further.

## Explicit T6 Blocker Checks

- Rejected `--ffprobe` fails closed before validation logging: PASS. `src/validation.rs:54` resolves the tool before target resolution, hashing, probing, or `append_validation_log`; `tests/cli_e01_validation_log_output_policy.rs:166-180` removes the prior validation log, runs `fake-ffprobe`, expects `unsupported tool binary`, and asserts no log is appended.
- Approved `--ffprobe` logs resolved metadata: PASS. `src/tool_policy.rs:69-104` resolves the tool path/version; `src/validation.rs:72-73` stores the resolved tool; `src/validation/log.rs:67-70` writes `tool_version`, `resolved_tool_path`, `command`, and `command_args`.
- Rejected `--ffmpeg` fails closed before output generation: PASS. `src/video_export.rs:110-119` and `src/artifacts.rs:237-247` route through `resolve_external_tool` before execution; `tests/cli_default_output_policy.rs:169-186` asserts `fake-ffmpeg` is rejected and the requested output is absent.
- Approved ffmpeg metadata logs: PASS. `src/artifacts.rs:363-384` and `src/video_export.rs:271-292` write resolved path/version and command args from the resolved tool. `manual-happy-log-summary.json` confirms this for proxy, thumbnail, frame capture, and export-video.
- Direct launch sweep: PASS. A repo search for direct `Command::new("ffmpeg")` or `Command::new("ffprobe")` in `src`/`tests` found no matches.

## Verification Performed

- Fresh run: `cargo test --locked derived_media_commands_require_policy_approved_ffmpeg_and_log_resolved_tool_metadata --test cli_default_output_policy -- --nocapture` -> PASS, 1 passed.
- Fresh run: `cargo test --locked validate_artifact_requires_policy_approved_ffprobe_and_logs_resolved_tool_metadata --test cli_e01_validation_log_output_policy -- --nocapture` -> PASS, 1 passed.
- Fresh run: `cargo test --locked resolves_approved_ffmpeg_path_and_rejects_disallowed_path_name --lib -- --nocapture` -> PASS, 1 passed.
- Fresh run: `cargo clippy --locked --all-targets --all-features -- -D warnings` -> PASS.
- Fresh run: `cargo test --locked` -> PASS, 142 library tests plus all integration/doc-test targets passed.
- Fresh run: `git diff --check -- <T6 files>` -> PASS, no output.
- LSP: initial `src` diagnostics request timed out; retry on `src/tool_policy.rs` returned no diagnostics. I did not rely on broad LSP as the main gate.

## Blockers

None.
