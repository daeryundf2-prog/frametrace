# Final QA Notepad

Skills surveyed:
- ultrawork hook: active for this turn; used for evidence-first execution discipline and cleanup receipt.
- ultraqa/manual QA concepts: applicable in spirit, but the assigned role is manual QA executor and the user supplied exact command surfaces; used direct CLI evidence rather than changing code.
- programming skill: not used because the task is QA-only and explicitly forbids production code changes.

Tier: HEAVY.
Justification: security-relevant symlink/output policy defenses plus user demanded final QA across regression suites and cleanup evidence.

Success criteria:
- Capture exact output/exit evidence for every requested command at FrameTrace HEAD 552b3fc40667d0d89ac35a2db8a346daa4265c95.
- Verify default generated output symlink defenses, existing output policy defenses, symlink regressions, derived output policy, media/report contracts, full suite, formatting, clippy, and diff hygiene.
- Record cleanup receipt proving no QA-spawned headless/browser/worker/process/temp dirs remain.
- Write the requested Markdown report ending with APPROVE only if all checks pass.

Self-review:
- All requested commands were run from /Users/shinyoohag/Desktop/frametrace.
- Each command exit code artifact records 0.
- Empty-output commands have non-empty receipt artifacts preserving the exact empty raw output block and exit status.
- No production code was modified; artifacts were written only under the requested ULW session evidence/review directories.
