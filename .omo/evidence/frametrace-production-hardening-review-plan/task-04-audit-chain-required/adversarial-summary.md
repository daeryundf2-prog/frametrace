# Adversarial Class Summary

- malformed_input: probed by tampered required audit log test; see adversarial-malformed-input.log.
- stale_state: probed by deleting/withholding a required proxy log for an existing artifact; see manual-qa-failure.log and adversarial-stale-state.log.
- dirty_worktree: probed by status/diff capture; see adversarial-dirty-worktree.log. Shared unrelated changes remain present and were not reverted.
- misleading_success_output: probed by checking exit codes plus exact blocker keys; see adversarial-misleading-success-output.log.
- flaky_tests: probed by rerunning focused report-defense tests; see adversarial-flaky-tests-rerun.log.
- hung_or_long_commands: bounded command execution completed; see adversarial-hung-or-long-commands.log.
- prompt_injection: not applicable; T4 does not process untrusted prompt/LLM text.
- cancel_resume: not applicable; T4 adds no resumable workflow.
- repeated_interruptions: not applicable; T4 adds no interrupt/resume surface.
