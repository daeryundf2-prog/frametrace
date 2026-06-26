# Hook Verification 3 - T4 Audit Chain

Verdict: PASS

Direct verification rerun:
- `cargo test --locked report_defense_ -- --nocapture`: PASS, 8 T4 report-defense tests passed. Artifact: `focused-report-defense-tests.log`.
- `target/debug/frametrace qa report-defense <happy-case>`: PASS, `exit_code=0`; checklist contains `[valid] proxy` and `required=yes`. Artifact: `cli-happy.log`.
- `target/debug/frametrace qa report-defense <missing-log-case>` after deleting `artifacts/proxies/proxy-log.jsonl`: PASS, command records `exit_code=1`; stderr contains `artifacts/proxies/proxy-log.jsonl [missing]`. Artifact: `cli-failure.log`.
- `cargo fmt --all -- --check`: PASS, `exit_code=0`. Artifact: `fmt-check.log`.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: PASS, `exit_code=0`. Artifact: `clippy.log`.
- `cargo test --locked`: PASS, `exit_code=0`. Artifact: `cargo-test-full.log`.
- `git diff --check`: PASS, `exit_code=0`. Artifact: `git-diff-check.log`.
- Evidence non-empty check: PASS. Artifact: `evidence-artifact-check.log`.

Judgment: T4 is verified by fresh binary observables. The real CLI accepts a valid required proxy audit chain and rejects a missing required proxy audit chain with exact blocker key `artifacts/proxies/proxy-log.jsonl [missing]`.
