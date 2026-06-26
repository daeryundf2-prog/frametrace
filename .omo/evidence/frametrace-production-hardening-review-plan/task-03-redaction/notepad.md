
## Final Evidence Map

- Baseline leaks: `baseline-leak-before-fix.md`, `baseline-review-leak-before-fix.md`.
- Failing-first test: `red-failing-regression.log` (`cargo test --locked review_bundle::tests::default_review_bundle_redacts_absolute_source_paths -- --nocapture`, failed before implementation).
- Focused green tests: `focused-tests-green.log`.
- Manual default redaction: `manual-default-redaction-pass.md` (`grep_tmp_exit=1`, `grep_source_exit=1`, `grep_redacted_exit=0`, `grep_internal_audit_exit=0`).
- Manual opt-in disclosure: `manual-opt-in-disclosure.md` (`grep_source_exit=0`, `grep_mode_exit=0`, `disclosure_files_exit=0`).
- Required verification gates: `verification-gates.log` (`final_exit=0`).
- Adversarial probes: `adversarial-classes.md`.
- Dirty worktree snapshot: `dirty-worktree-snapshot.md`.
