recommendation: REJECT

blockers:
- `doneclaim.json` claims `review_rechecks[gate-reviewer].status == "pass"` for `.omo/evidence/frametrace-production-hardening-review-plan-task-06-tool-policy-gate-review.md`, but that referenced artifact itself says `recommendation: REJECT` and lists a blocker. This leaves stale/misleading success evidence in the doneclaim, even though the zero-byte artifacts are repaired.

originalIntent:
- Re-verify FrameTrace T6 after the evidence hygiene fix in read-only review mode.
- T6 intent from `.omo/plans/frametrace-production-hardening-review-plan.md`: route ffmpeg and ffprobe execution through the external tool policy resolver, record resolved binary path/version/args/provenance for export/proxy/thumbnail/frame operations, and avoid direct `Command::new("ffmpeg")` execution.

desiredOutcome:
- No zero-byte files remain in `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/`.
- `verify-closeout-doneclaim-json.log` is non-empty and records `jq . doneclaim.json` with `exit_code: 0`.
- Prior substantive T6 checks still stand: unforgeable `ResolvedExternalTool`, size gate, no direct ffmpeg/ffprobe command bypass, focused tests, global fmt/clippy/test/diff evidence.
- Evidence is coherent: pass labels in `doneclaim.json` must be supported by their referenced artifacts.

userOutcomeReview:
- Fresh `find .omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy -type f -size 0 -print` exited 0 with no output.
- `verify-closeout-doneclaim-json.log` is 11685 bytes, contains the jq-rendered doneclaim, and ends with `exit_code: 0`.
- Fresh `jq . doneclaim.json` exited 0.
- Fresh `git diff --check` exited 0.
- Fresh direct-site `rg` for `Command::new("ffmpeg")` and `Command::new("ffprobe")` exited 1 with no matches.
- Production inspection confirmed ffmpeg/ffprobe routes through `resolve_external_tool` and `run_external_tool` in `src/artifacts/ffmpeg.rs`, `src/video_export.rs`, `src/ffprobe.rs`, and `src/validation.rs`; `ResolvedExternalTool` fields are private in `src/tool_policy/execution.rs`.
- Final evidence logs support the substantive behavior: forge test passes, size log lists all scoped files below 250 pure LOC, focused ffmpeg/ffprobe policy tests pass, clippy/fmt/full cargo test/git diff logs pass.
- The stale doneclaim-to-gate-review reference prevents confirmation because it is misleading success output from the artifact set the user asked to verify.

checkedArtifactPaths:
- `.omo/plans/frametrace-production-hardening-review-plan.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/doneclaim.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-closeout-doneclaim-json.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/zero-byte-artifacts-final-rerun.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/zero-byte-artifacts-final.txt`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-closeout-zero-byte-artifacts.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/cleanup-receipt.json`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-tool-policy-api-forge-final.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-tool-policy-size.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-direct-ffmpeg-ffprobe-command-sites.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-resolved-tool-construction-sites.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-tool-policy-tests-final.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-cli-ffmpeg-policy-final.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-cli-ffprobe-policy-final.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-derived-output-policy-tests-final.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-fmt-check-final.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-clippy-final.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-full-cargo-test-final.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-closeout-full-cargo-test.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-git-diff-check-final.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-closeout-git-diff-check.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/code-review.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy-recheck-code-review.md`
- `.omo/evidence/frametrace-production-hardening-review-plan-task-06-tool-policy-gate-review.md`
- `.omo/evidence/frametrace-t6-tool-policy-security-audit-gate-review.md`
- `src/tool_policy.rs`
- `src/tool_policy/execution.rs`
- `src/artifacts/ffmpeg.rs`
- `src/video_export.rs`
- `src/ffprobe.rs`
- `src/validation.rs`

exactEvidenceGaps:
- `doneclaim.json` was created after the newer code-review recheck, but its `gate-reviewer` pass entry still references the older `.omo/evidence/frametrace-production-hardening-review-plan-task-06-tool-policy-gate-review.md` artifact from 17:55, whose content is `recommendation: REJECT`.
- A newer approving security gate exists at `.omo/evidence/frametrace-t6-tool-policy-security-audit-gate-review.md`, but it is not the doneclaim-referenced gate-review artifact.
- The cleanup section in `doneclaim.json` still names `zero-byte-artifacts-final.txt` rather than the supplied `zero-byte-artifacts-final-rerun.txt`; this is not a zero-byte blocker because fresh `find` and the rerun receipt are clean, but it is another sign that the doneclaim was not fully rewired to the repaired evidence.

slopAndProgrammingPass:
- Loaded and applied `omo:remove-ai-slops` and `omo:programming` criteria directly over the scoped evidence, diff implications, tests, and production paths. The substantive T6 code path did not show unresolved slop: tests are behavior-oriented enough for the security policy goal, production extraction into `tool_policy/execution.rs` and related modules addresses the size/API-opacity issue, and final clippy/fmt/test evidence is present.
- The code review reports now explicitly include the same skill-perspective and overfit/slop coverage in `code-review.md` and `task-06-tool-policy-recheck-code-review.md`; report coverage does not cure the stale doneclaim gate-review reference.
