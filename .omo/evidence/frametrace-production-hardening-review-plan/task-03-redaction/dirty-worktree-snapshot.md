# Dirty Worktree Snapshot
captured_utc=2026-06-24T08:01:36Z

## git status --short
 M Cargo.lock
 M Cargo.toml
 M docs/RECOVERY_BOUNDARIES.md
 M gui/evidence-viewer/app.js
 M src/cli/commands.rs
 M src/cli/handlers.rs
 M src/cli/media_cmd.rs
 M src/cli/mod.rs
 M src/html_report.rs
 M src/lib.rs
 M src/package.rs
 M src/qa.rs
 M src/qa_release.rs
 M src/qa_tests.rs
 M src/report.rs
 M src/review_bundle.rs
 M src/validation.rs
 M src/validation/log.rs
 M src/validation/target.rs
 M tests/cli_e01_validation_log_output_policy.rs
 M tests/cli_smoke.rs
?? .omo/boulder.json
?? .omo/drafts/
?? .omo/evidence/
?? .omo/plans/
?? .omo/start-work/
?? .omo/ulw-loop/.current-media-session
?? .omo/ulw-loop/frame-full-ga-20260617222935/
?? .omo/ulw-loop/frame-gui-20260617102845/
?? .omo/ulw-loop/frame-master-rcga-cleanup-receipt.txt
?? .omo/ulw-loop/frame-master-rcga-review-blocker.md
?? .omo/ulw-loop/frame-media-validation-20260617024104/
?? .omo/ulw-loop/frame-production-exec-20260623-brief.md
?? .omo/ulw-loop/frame-production-exec-20260623/
?? .omo/ulw-loop/frame-production-seq-20260623-brief.md
?? .omo/ulw-loop/frame-production-seq-20260623/
?? .omo/ulw-loop/frame-review-progress-20260624/
?? .omo/ulw-loop/frame-windows-prereq-gate-20260622/
?? .omo/ulw-loop/frame-winui-20260617-playback-cli.txt
?? .omo/ulw-loop/frame-winui-cleanup-receipt.txt
?? .omo/ulw-loop/frame-winui-latest-playback-case.txt
?? .omo/ulw-loop/frame-winui-playback-cli-pass.txt
?? .omo/ulw-loop/frame-winui-workstation-status-release.json
?? .omo/ulw-loop/frame-winui-workstation-status.json
?? scripts/qa/
?? src/distributable_redaction.rs
?? src/qa_release_manifest.rs
?? src/qa_release_manifest_tests.rs
?? src/validation/target_tests.rs

## git diff --name-only
Cargo.lock
Cargo.toml
docs/RECOVERY_BOUNDARIES.md
gui/evidence-viewer/app.js
src/cli/commands.rs
src/cli/handlers.rs
src/cli/media_cmd.rs
src/cli/mod.rs
src/html_report.rs
src/lib.rs
src/package.rs
src/qa.rs
src/qa_release.rs
src/qa_tests.rs
src/report.rs
src/review_bundle.rs
src/validation.rs
src/validation/log.rs
src/validation/target.rs
tests/cli_e01_validation_log_output_policy.rs
tests/cli_smoke.rs

## T3 touched/owned files
src/distributable_redaction.rs
src/lib.rs
src/review_bundle.rs
src/report.rs
src/html_report.rs
src/cli/commands.rs
src/cli/mod.rs
src/cli/handlers.rs
src/package.rs
gui/evidence-viewer/app.js
docs/RECOVERY_BOUNDARIES.md
.omo/plans/t3-distributable-redaction.md
.omo/evidence/frametrace-production-hardening-review-plan/task-03-redaction/

## Known pre-existing/shared T1/T2 or adjacent dirty files not owned/reverted by T3
Cargo.toml
Cargo.lock
src/qa.rs
src/qa_release.rs
src/qa_tests.rs
tests/cli_smoke.rs
scripts/qa/
src/qa_release_manifest.rs
src/qa_release_manifest_tests.rs
src/validation.rs
src/validation/log.rs
src/validation/target.rs
src/validation/target_tests.rs
src/cli/media_cmd.rs
tests/cli_e01_validation_log_output_policy.rs
