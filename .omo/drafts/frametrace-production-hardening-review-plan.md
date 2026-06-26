---
slug: frametrace-production-hardening-review-plan
status: plan-written-awaiting-execution-approval
intent: clear
pending-action: user approval to execute .omo/plans/frametrace-production-hardening-review-plan.md via start-work/implementation mode
approach: Convert the 2026-06-24 readiness/code review findings into a strict execution plan that fixes false readiness, privacy leakage, audit/provenance gaps, large-case memory risks, maintainability debt, real corpus validation, Windows validation, WinUI shell, installer, and final release gates in dependency order.
---

# Draft: frametrace-production-hardening-review-plan

## Components (topology ledger)
<!-- Lock the SHAPE before depth. One row per top-level component that can succeed or fail independently. -->
<!-- id | outcome (one line) | status: active|deferred | evidence path -->
| C1 | false release/readiness PASS remains impossible | active | .omo/ulw-loop/frame-review-progress-20260624/evidence/progress-readiness-audit.md |
| C2 | distributable report/viewer/package output redacts local paths by default | active | .omo/ulw-loop/frame-review-progress-20260624/evidence/code-review-scan.md |
| C3 | report-defense blocks on required missing audit chains | active | .omo/ulw-loop/frame-review-progress-20260624/evidence/code-review-scan.md |
| C4 | large-case report/review/compatibility outputs use bounded or streaming paths | active | .omo/ulw-loop/frame-review-progress-20260624/evidence/remaining-steps.md |
| C5 | validation target resolution is typed, case-scoped, and audit-chain aware | active | .omo/ulw-loop/frame-review-progress-20260624/evidence/code-review-scan.md |
| C6 | ffmpeg-derived outputs go through the external-tool policy and provenance path | active | .omo/ulw-loop/frame-review-progress-20260624/evidence/code-review-scan.md |
| C7 | oversized modules are split only after behavior is pinned | active | .omo/ulw-loop/frame-review-progress-20260624/evidence/code-review-scan.md |
| C8 | real corpus accuracy/reproducibility evidence is created and release-gated | active | .omo/ulw-loop/frame-review-progress-20260624/evidence/progress-readiness-audit.md |
| C9 | Windows engine validation is proven before WinUI implementation | active | docs/WINDOWS_IMPLEMENTATION_HANDOFF.md |
| C10 | WinUI 3 shell is a client of Rust/SQLite/audit, not a second source of truth | active | docs/WINUI3_SHELL_CONTRACT.md |
| C11 | installer/package/release artifacts are blocked unless all review gates have typed evidence | active | docs/FULL_PRODUCTION_GA_READINESS_REPORT.md |

## Open assumptions (announced defaults)
<!-- Record any default you adopt instead of asking, so the user can veto it at the gate. -->
<!-- assumption | adopted default | rationale | reversible? -->
| execution priority | security/privacy, audit/provenance correctness, large-case survival, then WinUI/release | matches review severity and avoids building GUI on unsafe contracts | yes |
| test strategy | tests-after for hardening/refactor; add failing regression first inside each todo when current behavior is known unsafe | existing test suite is green; new regressions should pin each gap before fixes | yes |
| redaction default | distributable/shared outputs omit full local source paths; full paths require explicit opt-in | prevents workstation/client path leakage in review bundles and reports | yes |
| Windows gate | no WinUI work starts until local/CI Windows engine validation is green or produces a named blocker | docs state WinUI is final phase and current macOS host cannot prove Windows readiness | yes |
| large-case policy | production/browser path never embeds 100k/1M rows as full JSON; compatibility exports stream or become explicit operator actions | workstation contract already forbids full JSON browser load | yes |

## Findings (cited - path:lines)
- `src/review_bundle.rs:129-140` serializes `source_path` and `file_url` from `row.full_path`.
- `src/report.rs:255-264` renders `scan.source_path`; `src/report.rs:330-354` renders source/output paths for exports and derived artifacts.
- `gui/evidence-viewer/app.js:1114-1118` displays `record.path` directly.
- `src/qa_report_defense.rs:28-33` only blocks report-defense on tampered audit chains, not missing required chains.
- `src/audit.rs:236-246` classifies absent logs as `AuditChainState::Missing`.
- `src/cli/handlers.rs:307-314` reads `db/video_index.json` for `make-report`.
- `src/report.rs:306-323` maps all videos into a report table.
- `src/scan.rs:248-264` builds merged JSON/JSONL/TSV strings in memory.
- `src/validation/target.rs:17-33` accepts direct file paths; `src/validation/target.rs:79-115` manually extracts JSON strings from logs.
- `src/video_export.rs:100-109` and `src/artifacts.rs:102-106,164-168,226-230` run `Command::new("ffmpeg")` directly.
- `docs/WINUI3_SHELL_CONTRACT.md` requires Rust/SQLite/audit as source of truth and bounded inventory transport.
- `docs/FULL_PRODUCTION_GA_READINESS_REPORT.md` marks full production/GA as partially ready because no real WinUI shell or Windows validation receipt exists.
- `.omo/ulw-loop/frame-review-progress-20260624/evidence/progress-readiness-audit.md` scores overall readiness 50/100 and says PARTIALLY READY.

## Decisions (with rationale)
- D1: Execute in dependency order: release gate integrity, privacy, audit/provenance, large-case safety, tool provenance, refactor, corpus metrics, Windows engine, WinUI, installer, release.
- D2: Treat all release/readiness claims as fail-closed; do not allow text-only or broad `complete/done/x` style evidence for production gates.
- D3: Make redaction the default for distributable outputs; any full-path disclosure is an explicit local/operator mode and must be recorded.
- D4: Convert missing required audit chains from informational status into blockers when corresponding report/artifact/index claims exist.
- D5: Do not implement WinUI before Rust engine contracts and Windows validation are stable.
- D6: Keep HTML prototype as review/prototype output; production large-case inventory remains SQLite/engine-backed.
- D7: Split large modules after behavior locks and high-risk contract fixes, not before.

## Scope IN
- Rust CLI/core hardening for privacy, audit, validation target resolution, external tool policy, report generation, scan compatibility outputs, release gates, and QA commands.
- SQLite-backed bounded/streaming report and inventory export behavior.
- Focused tests, negative tests, performance checks, and evidence artifacts for each blocker.
- Real corpus manifest, accuracy/reproducibility validation, and report-defensible failure/skip/partial/unsupported reporting.
- Windows engine validation and WinUI 3 shell planning/implementation only after prerequisite gates pass.
- Installer/package and final release readiness gates.

## Scope OUT (Must NOT have)
- No claim that FrameTrace is production-ready, GA-ready, legally admissible, or otherwise legally guaranteed until release gates pass with artifacts.
- No writes to original evidence or source evidence paths.
- No derived output in source evidence paths.
- No full 100k/1M row JSON browser load in production GUI/review paths.
- No WinUI durable state that bypasses Rust engine, SQLite case DB, or audit logs.
- No refactor that changes behavior without pinned regression evidence.

## Open questions
None blocking. The only execution choice left is whether to start implementation from this plan now or run a separate high-accuracy plan review first.

## Approval gate
status: plan-written-awaiting-execution-approval
<!-- When exploration is exhausted and unknowns are answered, set status: awaiting-approval. -->
<!-- That durable record is the loop guard: on a later turn read it and resume at the gate instead of re-running exploration. -->
