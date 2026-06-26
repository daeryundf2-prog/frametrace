# FrameTrace T6 External Tool Policy Security Review

Verdict: PASS
codeQualityStatus: WATCH
recommendation: APPROVE
reportPath: `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/security-review.md`
blockers: None

## Scope

- Repository: `/Users/shinyoohag/Desktop/frametrace`
- Updated diff reviewed: `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/t6-diff-after-ffprobe.patch`
- Primary security criteria: command execution policy, canonical path use, allowed tool names, no shell injection, rejected ffmpeg/ffprobe path/name failure, and no direct `Command::new("ffmpeg")` / `Command::new("ffprobe")` outside policy.

## Skill-Perspective Check

- `omo:remove-ai-slops`: loaded and applied as an overfit/slop review pass. I did not find deletion-only tests, tests that merely verify a requested removal, tautological tests, or tests that only mirror implementation constants. The added CLI tests exercise observable behavior: approved tool path provenance is logged and rejected fake tools fail before outputs/logs are written.
- `omo:programming`: loaded, including the Rust reference and code-smell reference. The diff has one non-blocking programming-perspective violation: `src/tool_policy.rs` now exceeds the 250 pure-LOC guideline. This is recorded under MEDIUM because it is maintainability debt, not a current command-execution bypass.
- `security-review`: loaded and applied for command injection, unsafe command construction, path traversal/provenance, and evidence verification.

## Findings

### CRITICAL

None.

### HIGH

None.

### MEDIUM

1. `src/tool_policy.rs:5` and `src/tool_policy.rs:107` - `ResolvedExternalTool` is publicly forgeable while `run_external_tool` is public.
   - Finding: current ffmpeg/ffprobe call sites construct `ResolvedExternalTool` through `resolve_external_tool`, but `tool_policy` is a public module and the struct fields are public. A future caller could construct `ResolvedExternalTool { path: "/bin/sh", ... }` and pass it to `run_external_tool`, bypassing the allowed-name/canonicalization invariant without using direct `Command::new("ffmpeg")`.
   - Impact: no current CLI exploit was found, but the policy invariant is not type-enforced.
   - Remediation: make `ResolvedExternalTool` fields private and expose read-only accessors, or make `run_external_tool` `pub(crate)` and keep construction limited to `resolve_external_tool`.

2. `src/tool_policy.rs:1` - T6 pushed the policy module over the loaded programming skill's 250 pure-LOC ceiling.
   - Finding: current file is 371 pure LOC total and about 268 pure LOC before the test module; `HEAD` production code before the test module was about 185 pure LOC. The module now contains executable resolution, process running, output path containment, source/output rejection, lexical normalization, and tests.
   - Impact: maintainability/review risk in a security-sensitive policy module.
   - Remediation: split external executable resolution/running into a focused module or split tests into a sibling test module so the command policy remains small and reviewable.

### LOW

1. Windows-specific executable-name behavior is implemented but not directly proven by the added evidence.
   - Evidence: `src/tool_policy.rs:156` handles `.exe` candidates under `#[cfg(windows)]`, and `src/tool_policy.rs:227` accepts allowed names with `.exe` suffixes. The new fake-tool integration tests are Unix-only shell-script tests.
   - Impact: low residual release risk for the Windows target.
   - Remediation: add a Windows CI/job or Windows-specific unit coverage for `ffmpeg.exe` / `ffprobe.exe` name resolution.

## Positive Security Results

- No direct `Command::new("ffmpeg")` or `Command::new("ffprobe")` remains in `src` or `tests`; a fresh `rg` search returned no matches.
- Current media call sites route through policy:
  - `src/artifacts.rs:237` resolves ffmpeg via `resolve_external_tool` and executes with `run_external_tool`.
  - `src/video_export.rs:110` resolves ffmpeg via `resolve_external_tool` and executes with `run_external_tool`.
  - `src/ffprobe.rs:11` resolves ffprobe via `resolve_external_tool`; `src/validation.rs:54` resolves once and passes the resolved tool into `probe_with_tool`.
- No shell injection path was found. All ffmpeg/ffprobe invocations use `Command::new(resolved_path).args(args)` with structured argument vectors.
- Canonical path and allowed-name checks are present for explicit paths and PATH-resolved bare names via `resolve_external_tool`.
- Rejected fake ffmpeg/ffprobe paths fail before output/log creation in the focused CLI tests.
- Provenance logs now include `resolved_tool_path`, `tool_version`, command path, command args, operator, hashes, and chained audit entry hashes for the reviewed media paths.

## Evidence Inspected

- Diff: `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/t6-diff-after-ffprobe.patch`
- Evidence record: `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/evidence-record.md`
- Direct launch sweep artifact: `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/post-ffprobe-fix-direct-tool-sites.txt`
- Test logs:
  - `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-tool-policy-tests-after-ffprobe.log`
  - `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-cli-ffmpeg-policy-after-ffprobe.log`
  - `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-cli-ffprobe-policy.log`
  - `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-full-cargo-test-after-ffprobe.log`
  - `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-clippy-after-ffprobe.log`
- Manual failure artifact: `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/manual-failure-command.stderr`

## Fresh Verification Performed

- `cargo test --locked validate_artifact_requires_policy_approved_ffprobe_and_logs_resolved_tool_metadata --test cli_e01_validation_log_output_policy -- --nocapture`: PASS.
- `cargo test --locked derived_media_commands_require_policy_approved_ffmpeg_and_log_resolved_tool_metadata --test cli_default_output_policy -- --nocapture`: PASS.
- `rg -n 'Command::new\("ffmpeg"\)|Command::new\("ffprobe"\)|Command::new\([^\n]*(ffmpeg|ffprobe)' src tests`: no matches.
- `rg -n 'resolve_external_tool\(|run_external_tool\(|probe_with_tool\(' src tests`: confirmed current ffmpeg/ffprobe media paths use the policy API.

## Final Assessment

The updated ffmpeg/ffprobe command execution surface passes the requested security criteria. The remaining items are policy-hardening and maintainability risks, not current blockers for T6 approval.
