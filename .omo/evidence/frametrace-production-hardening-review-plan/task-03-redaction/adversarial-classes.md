# T3 Adversarial Class Probe Summary

captured_utc: 2026-06-24T08:02:00Z

## malformed_input

Scenario: manual QA used temp paths with spaces, client-like names, and Korean text:

- `/tmp/FrameTrace Client ACME 유출 Manual PASS a4JF/...`
- `/tmp/FrameTrace Client ACME OptIn 유출 sufY/...`

Binary observables:

- Default mode artifact: `manual-default-redaction-pass.md`
- Default grep results: `grep_tmp_exit=1`, `grep_source_exit=1`, `grep_redacted_exit=0`
- Opt-in artifact: `manual-opt-in-disclosure.md`
- Opt-in grep results: `grep_source_exit=0`, `grep_mode_exit=0`

The fixtures also included `file://` source URLs. Default mode redacted them; opt-in mode retained them with `local_operator_full_paths` metadata.

## dirty_worktree

Artifact: `dirty-worktree-snapshot.md`

The snapshot records T3-owned files separately from existing shared dirty files. T3 did not revert or overwrite the listed T1/T2/adjoining worktree state.

## stale_state

Manual QA used freshly-created `mktemp -d` roots for each run and generated report/review/package outputs under those roots. The passing default artifact is `manual-default-redaction-pass.md`; the earlier failed default artifact remains as `manual-default-redaction.md` and is not used as success evidence.

## misleading_success_output

Artifacts include command exit codes and grep exit codes, not prose-only claims:

- `manual-default-redaction-pass.md`: `exit_make_report=0`, `exit_make_review=0`, `exit_package=0`, `grep_tmp_exit=1`, `grep_source_exit=1`, `grep_redacted_exit=0`, `grep_internal_audit_exit=0`
- `manual-opt-in-disclosure.md`: `exit_make_report=0`, `exit_make_review=0`, `exit_package=0`, `grep_source_exit=0`, `grep_mode_exit=0`, `disclosure_files_exit=0`
- `verification-gates.log`: every required verification command records `exit_code=0`, with `final_exit=0`

## flaky_tests_hung_or_long_commands

Artifacts include elapsed time and exit status metadata:

- `focused-tests-green.meta`
- `manual-default-redaction-pass.meta`
- `manual-opt-in-disclosure.meta`
- `verification-gates.meta`

No hung commands or abandoned long-running sessions remained after verification.

## prompt_injection

Not applicable beyond ordinary artifact handling: T3 generated and inspected local JSON/HTML/log artifacts and treated them as data. No untrusted artifact text was executed as instructions.

## cancel_resume

Not applicable: there was no cancel/resume branch during implementation. A status ping occurred during active progress and was answered with the required `WORKING:` status before continuing.

## repeated_interruptions

Not applicable: the task had one status ping and no repeated conflicting interruptions. The newest request was honored without changing the T3 scope.
