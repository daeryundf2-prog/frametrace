# T6 Gate Review Reference Correction

## Scope

Evidence-only correction for T6, "Route all ffmpeg execution through external tool policy." No product code was edited.

## Stale Reference

The historical `doneclaim.json` contains:

- `review_rechecks[gate-reviewer].status`: `pass`
- `review_rechecks[gate-reviewer].artifact`: `.omo/evidence/frametrace-production-hardening-review-plan-task-06-tool-policy-gate-review.md`

That referenced artifact is not a passing gate review. Its content starts with `recommendation: REJECT` and lists blockers about missing T6-specific code-review/slop coverage. Therefore it must not be used as PASS evidence.

The later reverify artifact `.omo/evidence/frametrace-production-hardening-review-plan-task-06-tool-policy-reverify-gate-review.md` correctly identifies this stale PASS reference as a blocker.

## Current Evidence State

The stale rejecting gate artifact is retained for audit history and is now classified as superseded/stale in `doneclaim-final.json`.

Current passing support artifacts include:

- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy-recheck-code-review.md`
- `.omo/evidence/frametrace-t6-tool-policy-security-audit-gate-review.md`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-tool-policy-api-forge-final.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-direct-ffmpeg-ffprobe-command-sites.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-tool-policy-tests-final.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-cli-ffmpeg-policy-final.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-cli-ffprobe-policy-final.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-fmt-check-final.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-clippy-final.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-full-cargo-test-final.log`
- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-git-diff-check-final.log`

This correction does not fake a fresh independent final gate approval. A separate gate reviewer should provide final confirmation after reading `doneclaim-final.json`.

## Correction Artifact

The corrected DoneClaim is:

- `.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/doneclaim-final.json`
