T6 tool-policy evidence record

Tier: HEAVY
Justification: external ffmpeg/ffprobe command execution policy and forensic audit metadata are security/provenance-sensitive external integration surfaces.

Skills used:
- omo:start-work: selected because the task is an implementation item from a Prometheus plan and requires ledger/evidence discipline.
- omo:programming: selected because the implementation edits Rust source and tests.
- omo:ulw-loop: selected because the injected ultrawork mode requires artifact-backed scenarios and cleanup receipts.
- omo:review-work: selected because this is significant implementation work and a final review gate is required.

Success criteria:
- Approved ffmpeg path passes through policy: `cargo test --locked derived_media_commands_require_policy_approved_ffmpeg_and_log_resolved_tool_metadata --test cli_default_output_policy -- --nocapture`.
- Rejected ffmpeg path/name fails before output generation: same CLI test plus manual failure scenario.
- Approved ffprobe path passes through policy and rejected ffprobe path/name fails closed before validation logging: `cargo test --locked validate_artifact_requires_policy_approved_ffprobe_and_logs_resolved_tool_metadata --test cli_e01_validation_log_output_policy -- --nocapture`.
- Derived logs include resolved path/version/args/operator/source/output/output hash/audit entry hash: CLI test and manual happy scenario.
- No direct `Command::new("ffmpeg")` or `Command::new("ffprobe")` remains outside the central policy path: `rg` sweep artifact.

RED evidence:
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/baseline-direct-ffmpeg-sites.txt`: four direct ffmpeg launch sites and two version metadata sites.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/red-cli-tool-policy.log`: CLI rejected `--ffmpeg` before implementation.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/red-export-log-metadata.log`: export log metadata assertion failed on missing `resolved_tool_path`.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/red-artifact-log-metadata.log`: artifact log metadata assertion failed on missing `resolved_tool_path`.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/red-ffprobe-cli-policy.log`: rejected `fake-ffprobe` still exited successfully before the review fix.

GREEN automated evidence:
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-tool-policy-tests-after-ffprobe.log`: policy unit tests passed, including approved ffmpeg path and rejected fake name.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-cli-ffmpeg-policy-after-ffprobe.log`: CLI approved/rejected ffmpeg policy test passed.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-cli-ffprobe-policy.log`: CLI approved/rejected ffprobe policy test passed.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-derived-output-policy-tests.log`: existing derived output policy tests passed.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-clippy-after-ffprobe.log`: `cargo clippy --locked --all-targets --all-features -- -D warnings` passed.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-full-cargo-test-after-ffprobe.log`: full `cargo test --locked` passed.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-git-diff-check-after-ffprobe.log`: `git diff --check` passed.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-rustfmt-t6-files-after-ffprobe.log`: rustfmt check passed for T6-touched files.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-fmt-check-after-ffprobe.log`: global `cargo fmt --all -- --check` passed.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/final-zero-byte-check.txt`: final closeout check found no zero-byte evidence files.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/artifact-index-final.txt`: final evidence artifact index.

Review evidence:
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/code-review.md`: final code review passed with no blockers.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/security-review.md`: final security review passed with no blockers.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/qa-review.md`: final QA review passed with `<verdict>PASS</verdict>`.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/stop-hook-verification-claim-01.log`: failed verifier harness attempt before assertions due to a zsh read-only variable name.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/stop-hook-verification-claim-01-rerun.log`: stop-hook verification rerun passed; it checked required evidence files, QA PASS marker, no direct ffmpeg/ffprobe command sites, focused policy tests, `cargo fmt --all -- --check`, and `git diff --check`.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/stop-hook-verification-claim-02.log`: failed verifier harness attempt due to an incorrect JSON shape assumption in the manual summary assertion.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/stop-hook-verification-claim-02-rerun.log`: second stop-hook verification rerun passed; it checked evidence/review files, failure stderr, QA PASS marker, no direct ffmpeg/ffprobe command sites, manual happy metadata fields, focused policy tests, `cargo fmt --all -- --check`, `git diff --check`, and zero-byte evidence files.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/stop-hook-verification-claim-03.log`: third stop-hook verification found current shared-worktree failures outside T6: full `cargo test --locked` fails in report-defense tests and global `cargo fmt --all -- --check` fails in `src/qa_tests.rs`.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/stop-hook-verification-claim-03-t6-scoped.log`: first T6-scoped rerun exposed formatting drift in T6-touched files; fixed afterward.
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/stop-hook-verification-claim-03-t6-scoped-rerun.log`: T6-scoped verification passed after formatting fixes; it checked no direct ffmpeg/ffprobe command sites, focused tool-policy/ffmpeg/ffprobe tests, T6-file rustfmt, `git diff --check`, manual happy/failure metadata, and zero-byte evidence files.

Manual QA:
- Happy scenario invocation: `target/debug/frametrace make-proxy|make-thumbnail|capture-frame|export-video <case> vid_000001 --ffmpeg <approved fake ffmpeg> --operator qa-tool-policy`.
- Happy observable: exit 0, copied logs contain `resolved_tool_path`, `tool_version`, `command_args`, `operator`, source/output artifact fields, `output_artifact_sha256`, and `entry_sha256`; `verify-audit` succeeds.
- Happy artifacts: `manual-happy-transcript.log`, `manual-happy-log-summary.json`, `manual-happy-output-hashes.txt`, `manual-happy-logs/*.jsonl`, `manual-happy-cleanup.txt`.
- Failure scenario invocation: `target/debug/frametrace export-video <case> vid_000001 --format mp4 --ffmpeg <fake-bin/fake-ffmpeg> --output <case>/artifacts/clips/rejected.mp4`.
- Failure observable: exit non-zero, stderr contains `unsupported tool binary`, and rejected output path is absent.
- Failure artifacts: `manual-failure-transcript.log`, `manual-failure-command.stderr`, `manual-failure-cleanup.txt`.

Adversarial classes:
- malformed_input: probed by disallowed fake ffmpeg and ffprobe names/paths; commands failed with `unsupported tool binary`.
- stale_state: probed by fresh temp cases and copied logs; no prior log state was reused.
- dirty_worktree: probed by `git-status-short.txt`; worktree has unrelated T1-T5/T4 changes, and T6 stayed within scoped files.
- misleading_success_output: probed by parsing JSON logs and verifying output hashes/audit chains rather than trusting stdout.
- flaky_tests: probed by focused tests plus full `cargo test --locked`; all passed on rerun.
- hung_or_long_commands: probed with fake bounded ffmpeg scripts; no long-running processes were started.
- prompt_injection: not applicable; no untrusted LLM/user text is executed or interpreted.
- cancel/resume: not applicable; commands are short synchronous CLI operations with no resumable job state added.
- repeated_interruptions: not applicable; no interrupt/resume behavior was introduced.

Cleanup:
- Happy QA temp root removed; see `manual-happy-cleanup.txt`.
- Failure QA temp root removed; see `manual-failure-cleanup.txt`.
- No tmux sessions, browser contexts, servers, containers, or bound ports were started for T6 QA.

Known risks:
- Shared worktree contains unrelated active T1-T5 changes; no revert was attempted.
- Current shared worktree is not globally green after the third stop-hook check: report-defense tests and `src/qa_tests.rs` formatting fail outside T6 ownership. T6 scoped verification passes in `stop-hook-verification-claim-03-t6-scoped-rerun.log`.
