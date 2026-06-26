# T4 audit-chain required notepad

Tier: HEAVY
Justification: report-defense gate semantics change across audit/report artifacts with required failing-first proof, CLI-surface QA, and strict review.

Skills:
- omo:start-work: user invoked a plan-backed start-work task; using its evidence ledger, worktree discipline, and manual-QA/adversarial requirements, while implementation remains in this session per higher-priority user/developer instruction.
- omo:programming: Rust source/test changes are in scope; loaded Rust reference before editing.
- omo:review-work: significant implementation work requires post-implementation review gate.

Success criteria:
1. User-visible deliverable: `frametrace qa report-defense <case>` reports typed audit-chain states and fails when a required chain for a reported/derived artifact is missing.
2. Happy scenario: `target/debug/frametrace qa report-defense <case>` on a valid case exits 0 and displays `valid` required chains.
3. Failure scenario: same command after withholding/deleting a required log exits non-zero with the exact missing log key.
4. State coverage: tests cover `missing`, `empty`, `valid`, `tampered`, `unsupported`, and `not-applicable`.
5. Verification: focused tests, fmt, clippy, relevant cargo tests, full cargo test if feasible, git diff --check; all outputs captured under this evidence dir.

Adversarial class plan:
- malformed_input: malformed/tampered/empty audit log tests and CLI evidence.
- stale_state: validate missing expected log for present derived artifact claim.
- dirty_worktree: record dirty status before/after and avoid unrelated files.
- misleading_success_output: assert exit code and blocker key, not stdout optimism.
- flaky_tests: rerun focused tests or stable command set; record deterministic pass.
- hung_or_long_commands: run long commands with bounded shell/tool timeouts and capture elapsed/failure.
- prompt_injection: not applicable, no untrusted prompt/LLM text processing in this task.
- cancel_resume: not applicable, no resumable workflow added.
- repeated_interruptions: not applicable, no interrupt recovery surface changed.

Plan:
1. Inspect current report-defense/audit/report/artifact code and prior review evidence.
2. Capture baseline/failing-first proof of current missing-log behavior.
3. Add failing tests for typed states and required-vs-optional behavior.
4. Implement minimal typed state model and required chain resolution.
5. Run focused automated verification and CLI manual QA happy/failure cases.
6. Run post-write review and record cleanup/ledger.
