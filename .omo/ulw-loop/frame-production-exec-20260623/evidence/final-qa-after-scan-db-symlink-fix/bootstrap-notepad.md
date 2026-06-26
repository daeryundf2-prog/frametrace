# Bootstrap

Goal: execute hands-on QA for FrameTrace HEAD f589dea after the scan-folder symlink escape fix, capture command evidence, cleanup receipt, and final Markdown QA report.

Skills surveyed:
- ultraqa: applicable because this is adversarial real-surface QA with cleanup and evidence requirements. Used as the QA workflow guide, with the user constraint that no product fixes are allowed.
- manual QA executor role: applicable and binding for the final `manualQa` matrix and artifact-backed PASS/FAIL evidence.

Tier: HEAVY. Justification: security-sensitive symlink/path-policy regression QA plus user-mandated full verification and cleanup evidence.

Success criteria:
- The target repo is on branch `codex/frametrace-forensic-hardening` and `git rev-parse HEAD` returns the requested HEAD `f589dea...`.
- Each required command is run on the CLI surface with exact output captured to a non-empty artifact and PASS/FAIL determined from exit status plus output.
- Symlink/output path, inventory/query, media/report DB, and full-suite regressions are covered by the requested focused tests and full suite.
- Cleanup receipt proves no FrameTrace QA temp dirs, long-running QA processes, browsers, or workers remain from this QA.
- Final report contains `manualQa.surfaceEvidence`, `manualQa.adversarialCases`, and `manualQa.artifactRefs`; final line is exactly `APPROVE` only if every required check passes and cleanup is complete, otherwise exactly `BLOCKED`.

Safety bounds:
- Do not change production code.
- Write only evidence/report files under `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/`.
