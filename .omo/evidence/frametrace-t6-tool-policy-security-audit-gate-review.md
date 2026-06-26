recommendation: APPROVE
verdict: PASS

# T6 Tool Policy Security Gate Review

## originalIntent

Security-audit the completed T6 FrameTrace external tool policy fix in read-only mode. The user wanted a PASS/FAIL answer covering whether ffmpeg/ffprobe execution is centrally policy-gated, downstream callers cannot forge `ResolvedExternalTool`, rejected fake tools fail before artifact/log creation, provenance logs still record required metadata, and no new temp-file or cleanup risk was introduced.

## desiredOutcome

- Downstream safe Rust callers cannot construct forged `ResolvedExternalTool` values.
- Public `run_external_tool` cannot be used with forged values to bypass `resolve_external_tool`.
- No direct `Command::new("ffmpeg")` or `Command::new("ffprobe")` sites remain outside the policy path.
- CLI/evidence proves approved ffmpeg/ffprobe paths work and rejected fake paths fail closed before output/log creation.
- Logs retain resolved path, version, command args, hashes, operator, and chained entry hash.
- T6 evidence artifacts are coherent and non-empty, and cleanup receipts show manual QA temp roots were removed.

## userOutcomeReview

The shipped code satisfies the security outcome. `ResolvedExternalTool` is public but its fields are private, with only read-only accessors; the only in-module constructor is `resolve_external_tool`. Public `run_external_tool` takes `&ResolvedExternalTool`, so downstream safe Rust cannot fabricate arbitrary command paths. Current ffmpeg/ffprobe call sites resolve first, then execute via `run_external_tool`, and logging receives the resolved tool object instead of re-resolving or using a bare command name.

Fresh verification confirmed no direct `Command::new("ffmpeg")` or `Command::new("ffprobe")` matches in `src` or `tests`. Focused CLI tests pass for ffmpeg and ffprobe policy behavior. Manual happy-path logs prove approved fake ffmpeg metadata for proxy, thumbnail, frame capture, and export-video. The ffprobe CLI test explicitly checks rejected fake ffprobe creates no validation log. The ffmpeg rejected-path CLI test checks no requested output is created; the code writes export logs only after successful tool execution and output hashing.

## blockers

None.

## securityFindings

### CRITICAL

None.

### HIGH

None.

### LOW / EVIDENCE GAPS

1. `security-review.md` contains a stale medium finding claiming `ResolvedExternalTool` fields are public. Current code contradicts it: fields are private at `src/tool_policy/execution.rs:5-9`, and the downstream compile-fail test passes. This is an evidence-artifact inconsistency, not a current code vulnerability.

2. The automated ffmpeg CLI test is asymmetric. `tests/cli_default_output_policy.rs:126-187` proves approved resolved metadata for `make-proxy`, `make-thumbnail`, and `capture-frame`, then proves rejected fake `export-video` leaves the requested output absent. Approved `export-video` metadata is covered by `src/video_export.rs` unit coverage and manual happy logs, not by the same CLI test loop.

3. The rejected fake ffmpeg CLI test asserts output absence, but does not directly assert `artifacts/clips/export-log.jsonl` absence. Code review of `src/video_export.rs:86-95` and `src/video_export.rs:223-248` shows the export log is written only after `run_ffmpeg_export` succeeds and the output file is hashed.

4. Programming-size watch items remain in touched files: `src/artifacts.rs` 474 pure LOC, `src/video_export.rs` 421, and `src/validation/log.rs` 267. I did not find this to create a T6 security bypass, but it remains maintenance debt under the loaded programming criteria.

## directSecurityChecks

- Forge resistance: PASS. `ResolvedExternalTool` fields are private; no public constructor exists. Fresh test `cargo test --locked resolved_external_tool_cannot_be_forged_by_downstream_crates --test tool_policy_api -- --nocapture` passed.
- Public runner bypass: PASS for safe downstream callers. `run_external_tool` at `src/tool_policy/execution.rs:120-125` only accepts `&ResolvedExternalTool`; callers must obtain one through `resolve_external_tool`.
- Direct ffmpeg/ffprobe command sites: PASS. Fresh `rg` for direct `Command::new("ffmpeg")` / `Command::new("ffprobe")` returned no matches.
- ffmpeg path policy: PASS. Fresh `cargo test --locked derived_media_commands_require_policy_approved_ffmpeg_and_log_resolved_tool_metadata --test cli_default_output_policy -- --nocapture` passed.
- ffprobe path policy: PASS. Fresh `cargo test --locked validate_artifact_requires_policy_approved_ffprobe_and_logs_resolved_tool_metadata --test cli_e01_validation_log_output_policy -- --nocapture` passed.
- Log metadata: PASS. Fresh `cargo test --locked log_body --lib -- --nocapture` passed for export, derived artifact, and validation log body tests; manual happy summary contains resolved path, version, command args, operator, output hashes, and entry hashes.
- Resolver unit coverage: PASS. Fresh `cargo test --locked resolves_approved_ffmpeg_path_and_rejects_disallowed_path_name --lib -- --nocapture` passed.
- LSP diagnostics: PASS with only inactive Windows `cfg` hints in helper test code for `src/artifacts.rs`, `src/video_export.rs`, and `src/validation/log.rs`; no diagnostics for `src/tool_policy/execution.rs` or `tests/tool_policy_api.rs`.

## slopAndProgrammingPass

Loaded and applied `omo:remove-ai-slops` and `omo:programming` criteria directly over the scoped diff, production code, and tests. I did not find deletion-only tests, tests that merely verify a requested removal, tautological tests, implementation-mirroring tests that provide false confidence, or unnecessary production extraction/parsing/normalization in the T6 security path. The code review report also explicitly records the same skill-perspective checks and overfit/slop coverage in `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/code-review.md`.

## tempFileAndCleanupReview

No new production temp-file surface was added. T6 production code uses caller-selected/default case artifact paths and validates them before tool execution. Manual QA cleanup receipts show happy and failure temp roots were removed with status 0. Evidence hygiene checks found no zero-byte files under the T6 evidence directory. Unit/CLI tests use temp directories and remove them at the end; a panic before cleanup could leave test temp residue, but I found no production cleanup vulnerability.

## checkedArtifactPaths

- `/Users/shinyoohag/Desktop/frametrace/src/tool_policy.rs`
- `/Users/shinyoohag/Desktop/frametrace/src/tool_policy/execution.rs`
- `/Users/shinyoohag/Desktop/frametrace/src/tool_policy/output_paths.rs`
- `/Users/shinyoohag/Desktop/frametrace/src/tool_policy/tests.rs`
- `/Users/shinyoohag/Desktop/frametrace/tests/tool_policy_api.rs`
- `/Users/shinyoohag/Desktop/frametrace/src/artifacts.rs`
- `/Users/shinyoohag/Desktop/frametrace/src/video_export.rs`
- `/Users/shinyoohag/Desktop/frametrace/src/validation.rs`
- `/Users/shinyoohag/Desktop/frametrace/src/validation/log.rs`
- `/Users/shinyoohag/Desktop/frametrace/src/ffprobe.rs`
- `/Users/shinyoohag/Desktop/frametrace/tests/cli_default_output_policy.rs`
- `/Users/shinyoohag/Desktop/frametrace/tests/cli_e01_validation_log_output_policy.rs`
- `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/evidence-record.md`
- `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/code-review.md`
- `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/security-review.md`
- `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/qa-review.md`
- `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/manual-happy-log-summary.json`
- `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/manual-failure-transcript.log`
- `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/manual-failure-command.stderr`
- `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/manual-happy-cleanup.txt`
- `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/manual-failure-cleanup.txt`
- `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-tool-policy-api-forge-final.log`
- `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-cli-ffmpeg-policy-final.log`
- `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-cli-ffprobe-policy-final.log`
- `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/stop-hook-verification-claim-03-t6-scoped-rerun.log`

## exactEvidenceGaps

- No `.omo/notepad.md` exists in the repo, so there was no notepad artifact to inspect.
- `security-review.md` is stale on `ResolvedExternalTool` field visibility.
- Automated CLI evidence does not include approved `export-video` in the same loop as approved proxy/thumbnail/frame; manual happy evidence covers it.
- Automated rejected fake ffmpeg evidence checks output absence but not export-log absence directly; code ordering supports fail-closed behavior before logging.
- Full-worktree global test/fmt status was previously reported as blocked by non-T6 shared-worktree changes in `stop-hook-verification-claim-03.log`; T6-scoped rerun passes.

