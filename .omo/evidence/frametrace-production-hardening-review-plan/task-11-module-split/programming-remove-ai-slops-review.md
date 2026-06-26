# T11 Programming / Remove-AI-Slops Review

## Scope Reviewed

Changed T11 refactor files:

- `src/performance_qa.rs` -> `src/performance_qa/{mod.rs,compatibility.rs,survival.rs,render.rs,tests.rs}`
- `src/qa_tests.rs` -> `src/qa_tests/{mod.rs,accuracy.rs,helpers.rs,release_privacy.rs,report_defense.rs,report_defense_audit.rs,reproducibility_performance.rs}`
- `src/qa.rs` path attribute update for the new test module directory

## Rust Programming Criteria

- No `unsafe` introduced.
- No production `unwrap`/`expect` introduced. Existing test-only `unwrap` usage remains in test modules.
- Public behavior preserved: `performance_report` remains exported via `qa::performance_report`; QA test modules remain `#[cfg(test)]` only.
- No dependencies added.
- Visibility was scoped to `pub(super)` where cross-submodule access was required.
- Allocation behavior is unchanged except for moving existing formatting/rendering code into submodules.

## Remove-AI-Slops / Overfit / Test-Slop Review

- Oversized-module smell reduced for two safe targets:
  - `src/performance_qa.rs` split by responsibility: orchestration, compatibility exports, large-case survival probes, rendering, tests.
  - `src/qa_tests.rs` split by behavior cluster: accuracy, report defense, audit defense, privacy/release, reproducibility/performance, shared helpers.
- No broad abstraction layer was added. New modules are responsibility boundaries, not generic helpers.
- No one-off helper bloat was added beyond moving existing shared QA fixture helpers into `src/qa_tests/helpers.rs`.
- No tests were weakened/deleted/skipped. Full test suite now runs 156 library tests plus integration suites.
- No command text, JSON field, or report-rendering logic was intentionally changed.

## Module-Size Conclusions

All new T11-created modules are <=250 pure LOC. See `module-size-report.md` and `post-module-loc.txt`.

Remaining oversized product files are documented with `SIZE_OK deferred for T11` rationale in `module-size-report.md`. The common reason is behavior risk: report renderer text, CLI command stdout, scan compatibility JSON/TSV contracts, and release/report-defense semantics would need dedicated snapshot/API pinning before deeper extraction. T11 completed safe splits where behavior was already pinned and green.

## Behavior Drift Review

- Focused baseline and post behavior snapshot commands passed.
- Full post gates passed: fmt check, clippy `-D warnings`, full `cargo test --locked`, Node syntax check, git diff whitespace check.
- Manual CLI smoke before/after normalized contract diff is empty.

## Review Verdict

PASS for T11 scoped behavior-preserving refactor. Known residual risk is explicitly documented remaining oversized files, not hidden.
