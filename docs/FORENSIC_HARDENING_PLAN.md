# FrameTrace Forensic Hardening Program

This document is the canonical execution program for hardening FrameTrace into a report-defensible forensic engineering tool. It replaces informal roadmap ordering with gated phases, measurable validation, ownership, migration safety rules, legal/review blockers, and release evidence requirements.

No implementation phase may start until its entry criteria are satisfied. No phase may close until its exit criteria and completion evidence are recorded.

## 1. Program Objective

FrameTrace must produce repeatable, explainable, measurable, and packageable analysis results for large video and artifact evidence cases.

Program priorities:

1. Preserve evidence integrity and provenance.
2. Prevent source/output contamination.
3. Make database changes migration-safe.
4. Quantify accuracy, reproducibility, performance, and UI behavior.
5. Produce report-defensible outputs with explicit limitations.
6. Scale to large evidence sets without memory or UI failure.
7. Defer new recovery features until governance, corpus, and ground truth are ready.

## 2. Canonical Execution Sequence

This is the only authoritative phase order. Commit strategy, deliverables, reviews, and release gates must reference this sequence.

| Phase | Name | Primary Owner | Phase Type |
| --- | --- | --- | --- |
| 0 | Baseline | Engineering Lead | Stabilization |
| 1 | Static Cleanup | Engineering Lead | Cleanup |
| 2 | Security Review | Security Owner | Review |
| 3 | DB Audit | DB Owner | Schema review |
| 4 | Migration Hardening | DB Owner | Schema implementation |
| 5 | Recovery MVP Governance | Forensic Lead | Design gate |
| 6 | Accuracy Validation | QA Owner | Validation |
| 7 | Reproducibility Validation | QA Owner | Validation |
| 8 | Report Defensibility | Forensic Lead | Report/review |
| 9 | Large Scale Survival | Performance Owner | Scale validation |
| 10 | Release Readiness | Release Manager | Release gate |
| 11 | Feature Expansion | Product Owner | Backlog execution |

Ordering rules:

1. Phase 0 must complete before any cleanup or implementation change.
2. Phase 1 must complete before schema pruning or feature expansion.
3. Phase 2 must complete before release-facing or file-opening workflows are expanded.
4. Phase 3 must complete before Phase 4 migration work.
5. Phase 4 must complete before any schema deletion is released.
6. Phase 5 must complete before new recovery implementation begins.
7. Phase 6 and Phase 7 must complete before report-defensible claims are expanded.
8. Phase 8 must complete before release candidate packaging.
9. Phase 9 must complete before large-case support is advertised.
10. Phase 10 must complete before any tagged release.
11. Phase 11 may start only after Phase 10 approves the hardened base or explicitly marks a feature as BACKLOG/DESIGN CANDIDATE.

## 3. Program Roles

| Role | Responsibility |
| --- | --- |
| Engineering Lead | Owns branch hygiene, cleanup sequencing, code review closure, and final technical integration. |
| DB Owner | Owns schema inventory, migrations, fixtures, index changes, and rollback evidence. |
| Forensic Lead | Owns corpus relevance, provenance requirements, report-defensible language, recovery boundaries, and analyst workflows. |
| QA Owner | Owns test plans, fixture execution, corpus validation, reproducibility checks, and failure tracking. |
| Security Owner | Owns static security review, dependency review, unsafe file handling review, and privacy leakage review. |
| Performance Owner | Owns scale targets, performance harnesses, memory/latency measurements, and large-case pass/fail results. |
| UX Owner | Owns viewer latency, large-list behavior, operator workflow review, and report readability checks. |
| Legal Reviewer | Reviews report-defensible wording and release claims. This is a review role, not a guarantee of admissibility. |
| Release Manager | Owns release blockers, evidence bundle, release notes, and go/no-go decision record. |
| Product Owner | Owns post-hardening feature priorities and backlog boundaries. |

## 4. Global Non-Negotiable Rules

1. No schema deletion without documented evidence that the table/column/index is unused or safely migrated.
2. Treat all removal candidates as `UNUSED CANDIDATE` until verified by usage matrix, tests, and migration fixture.
3. No existing database compatibility break without migration code, backup procedure, fixture test, and rollback plan.
4. Record failed, skipped, missing, partial, unsupported, and unverifiable outputs.
5. Use SQLite as the source of truth for large indexes. JSON/JSONL/TSV are compatibility/export artifacts.
6. Reports must preserve source identity, derived artifact relationships, tool versions, command options, hashes, limitations, and validation status.
7. Use the term `report-defensible`; do not use phrases that claim guaranteed legal readiness, legal grading, or legal proof.
8. No release may bypass release blockers in Section 12.
9. No new recovery implementation may begin before Phase 5 exit criteria are satisfied.
10. Large-scale support must be backed by Phase 9 results, not assumptions.

## 5. Phase Execution Template

Every phase below uses the required template:

- Purpose
- Scope
- Inputs
- Entry Criteria
- Tasks
- Deliverables
- Validation
- Exit Criteria
- Risks
- Blockers
- Owner

## 6. Phase Plans

### Phase 0: Baseline

Purpose:

- Freeze the current repo, toolchain, tests, and known limitations before cleanup or hardening work starts.

Scope:

- Documentation-only and diagnostic-only work.
- No source code behavior changes.

Inputs:

- Current repository state.
- Current Rust toolchain.
- Current GUI/HTML/JS assets.
- Current README and existing planning documents.

Entry Criteria:

- Working tree state recorded.
- Target branch selected.
- No destructive Git operation pending.

Tasks:

1. Create or select branch `codex/frametrace-forensic-hardening`.
2. Record commit SHA, OS, Rust version, Cargo version, Node version when available, and external tool versions when available.
3. Run:
   - `cargo fmt --check`
   - `cargo check --all-targets`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo test`
   - `node --check gui/evidence-viewer/app.js`
4. Record failures as baseline findings, not implementation defects yet.
5. Create the initial static-analysis report.

Deliverables:

- `docs/static-analysis.md`
- Baseline command transcript or summarized results.
- Baseline issue list with severity and owner.

Validation:

- Baseline commands executed and outputs reviewed.
- Failures have issue IDs or backlog entries.

Exit Criteria:

- Baseline report exists.
- All baseline failures are either fixed before exit or explicitly tracked.
- Release Manager confirms the baseline is reproducible.

Risks:

- Hidden local dependencies may make baseline non-reproducible.
- Existing uncommitted files may be mistaken for hardening work.

Blockers:

- Repository cannot build.
- Required toolchain cannot be installed.
- Working tree contains unrelated user changes that obscure baseline.

Owner:

- Engineering Lead

### Phase 1: Static Cleanup

Purpose:

- Reduce code surface and ambiguity before security, schema, and forensic changes.

Scope:

- Dead-code removal.
- Visibility reduction.
- Duplicate helper consolidation.
- Cleanup-only refactors protected by tests.

Inputs:

- Phase 0 baseline report.
- Symbol inventory.
- Existing tests.

Entry Criteria:

- Phase 0 complete.
- Baseline failures triaged.
- Cleanup plan reviewed by Engineering Lead.

Tasks:

1. Inventory all `pub fn`, `pub struct`, and `pub enum` items.
2. Map each symbol to CLI, tests, report, viewer, DB layer, or external compatibility.
3. Mark each unused item as:
   - `DELETE`
   - `KEEP: COMPATIBILITY`
   - `KEEP: TEST SUPPORT`
   - `KEEP: FUTURE PUBLIC API`
   - `INVESTIGATE`
4. Delete only items marked `DELETE`.
5. Reduce unnecessary `pub` to `pub(crate)` or private.
6. Consolidate duplicated JSON/path/DB helper code only after tests cover existing behavior.
7. Run static checks after each cleanup commit.

Deliverables:

- `docs/cleanup-review.md`
- Symbol usage matrix.
- Cleanup commit list.

Validation:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- CLI smoke tests

Exit Criteria:

- No cleanup commit changes documented behavior unless explicitly approved.
- Cleanup report explains each deletion and each retained dead-code candidate.
- All validation commands pass.

Risks:

- Removing compatibility code that is not referenced by current tests.
- Cleanup commits accidentally changing behavior.

Blockers:

- Missing usage evidence for a deletion candidate.
- Test gap around a helper targeted for consolidation.

Owner:

- Engineering Lead

### Phase 2: Security Review

Purpose:

- Identify security and privacy risks before expanding evidence ingestion, report generation, or file viewing.

Scope:

- Local file handling.
- HTML/JS viewer safety.
- Path traversal.
- External command invocation.
- Report content escaping.
- Dependency review.
- Privacy leakage risks.

Inputs:

- Phase 1 cleaned codebase.
- Current dependency list.
- Viewer/report files.
- CLI command list.

Entry Criteria:

- Phase 1 complete.
- Security Owner assigned.
- Static checks passing.

Tasks:

1. Review all paths passed to external tools.
2. Review generated HTML and embedded JSON escaping.
3. Review file URL generation and local file opening behavior.
4. Review package output path rules and overwrite behavior.
5. Review report/export for private path leakage.
6. Review dependency surface with `cargo tree`.
7. Classify findings as P0/P1/P2/P3.
8. Add fixes or backlog entries according to severity.

Deliverables:

- `docs/security-review.md`
- Dependency inventory.
- Security finding register.

Validation:

- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- Manual HTML/report escaping review
- Dependency review completed

Exit Criteria:

- No unresolved P0/P1 security finding.
- P2 findings have owners and target phases.
- Privacy leakage risks documented.

Risks:

- HTML reports may expose local paths unnecessarily.
- External command arguments may behave differently on Windows.
- Generated viewer may render untrusted strings.

Blockers:

- Any evidence-corrupting or arbitrary-file-write finding.
- Any unescaped report content that can execute script.

Owner:

- Security Owner

### Phase 3: DB Audit

Purpose:

- Build a complete schema, query, and index usage record before migration or deletion.

Scope:

- SQLite tables, columns, indexes, and query patterns.
- No schema changes yet.

Inputs:

- Current schema.
- Current SQL statements.
- Existing case DB examples if available.
- Phase 1 and Phase 2 findings.

Entry Criteria:

- Phase 2 complete.
- DB Owner assigned.
- Current schema version identified.

Tasks:

1. Inventory tables:
   - `schema_meta`
   - `scan_runs`
   - `videos`
   - `evidence_sources`
   - `jobs`
   - `job_events`
2. Inventory every column and classify it:
   - `REQUIRED_CORE`
   - `REQUIRED_PROVENANCE`
   - `REQUIRED_REPORT`
   - `REQUIRED_PACKAGE`
   - `DERIVED_REDUNDANT`
   - `UNUSED_CANDIDATE`
3. Map every table and column to read/write code paths.
4. Map every index to a query or planned query.
5. Run `EXPLAIN QUERY PLAN` for representative search, timeline, validation, dashboard, and package queries.
6. Produce deletion candidates but do not delete anything in this phase.

Deliverables:

- `docs/schema.md`
- Table/column usage matrix.
- Index usage matrix.
- Query-plan evidence.

Validation:

- Every table has at least one documented owner or deletion candidate status.
- Every column has a classification.
- Every deletion candidate has evidence and a migration requirement.

Exit Criteria:

- DB Owner approves the schema audit.
- Engineering Lead confirms no schema changes occurred in this phase.
- All removals remain candidates only.

Risks:

- A column may be used by generated reports or exported artifacts rather than direct SQL.
- Current tests may not cover all DB compatibility needs.

Blockers:

- Unknown purpose for any table or column.
- No fixture strategy for existing case DBs.

Owner:

- DB Owner

### Phase 4: Migration Hardening

Purpose:

- Make schema evolution safe, testable, reversible, and auditable.

Scope:

- Migration framework.
- v1 fixture database.
- Backup and rollback procedure.
- Index additions/removals.
- Schema deletions only after Phase 3 evidence.

Inputs:

- Phase 3 schema audit.
- v1 fixture database.
- Migration contract in Section 10.
- Existing tests.

Entry Criteria:

- Phase 3 complete.
- v1 fixture DB created or source fixture recipe approved.
- DB backup procedure approved.
- Rollback procedure drafted.

Tasks:

1. Implement or document migration naming and execution order.
2. Add migration tests using fixture databases.
3. Add backup before migration.
4. Add rollback procedure for failed migration.
5. Add indexes only when query-plan evidence supports them.
6. Remove tables/columns only after:
   - Usage matrix proves candidate status.
   - Migration preserves required data.
   - Fixture migration passes.
   - Rollback is tested.
7. Record all schema changes in `docs/schema.md`.

Deliverables:

- Migration implementation or documented migration procedure.
- `tests/fixtures/db/v1/` fixture or fixture-generation script.
- Migration verification report.
- Updated `docs/schema.md`.

Validation:

- v1 fixture opens and migrates.
- Row counts match expected values.
- Required fields survive migration.
- Query-plan tests pass for new indexes.
- Failed migration leaves backup intact.

Exit Criteria:

- Migration tests pass.
- Rollback procedure verified.
- No unverified schema deletion remains.

Risks:

- Migration failure may corrupt case DB.
- Index additions may slow writes.
- Schema pruning may remove future report/provenance data.

Blockers:

- No v1 fixture.
- No tested rollback path.
- Any destructive migration without evidence.

Owner:

- DB Owner

### Phase 5: Recovery MVP Governance

Purpose:

- Govern recovery work before new recovery implementation begins.

Scope:

- Requirements, test specification, corpus, ground truth, and recovery boundaries.
- No new recovery feature implementation unless exit criteria are met.

Inputs:

- Existing recovery capabilities.
- Phase 4 database contract.
- Validation corpus plan.
- Product recovery requirements.

Entry Criteria:

- Phase 4 complete.
- Forensic Lead assigned.
- Product Owner confirms recovery scope.

Tasks:

1. Write Recovery PRD.
2. Write Recovery Test Specification.
3. Prepare recovery corpus and ground truth.
4. Define supported and unsupported recovery types.
5. Define validation labels:
   - `validated`
   - `candidate-unvalidated`
   - `unsupported`
   - `partial`
   - `failed`
6. Mark all new recovery features before approval as `BACKLOG` or `DESIGN CANDIDATE`.
7. Review existing recovery features for labeling, provenance, and report wording.

Deliverables:

- `docs/recovery-prd.md`
- `docs/recovery-test-spec.md`
- Recovery corpus manifest
- Recovery boundary update
- Backlog/design-candidate list

Validation:

- PRD approved.
- Test specification approved.
- Corpus prepared.
- Ground truth dataset available.
- Existing recovery outputs use correct labels.

Exit Criteria:

- Forensic Lead approves scope.
- QA Owner approves test specification.
- Product Owner approves MVP boundary.
- No new recovery work remains unclassified.

Risks:

- Recovery output may be over-claimed.
- Ground truth may be incomplete.
- Proprietary formats may not be independently verifiable.

Blockers:

- Missing PRD.
- Missing test specification.
- Missing corpus.
- Missing ground truth.

Owner:

- Forensic Lead

### Phase 6: Accuracy Validation

Purpose:

- Quantify detection and classification behavior against a defined corpus.

Scope:

- Corpus A-F from Section 9.
- Video detection.
- Artifact detection.
- Recovery labeling.
- False positives and false negatives.

Inputs:

- Approved validation corpus.
- Ground truth manifests.
- Phase 5 recovery governance outputs.

Entry Criteria:

- Phase 5 complete.
- Corpus manifests available.
- QA Owner assigned.
- Metric targets from Section 8 approved.

Tasks:

1. Implement or run `frametrace qa accuracy`.
2. Compare outputs to ground truth.
3. Compute precision, recall, false positive rate, and false negative rate.
4. Record unsupported samples separately from misses.
5. Produce machine-readable and human-readable reports.
6. File defects for failed metrics.

Deliverables:

- `accuracy-report.json`
- `accuracy-report.html`
- Accuracy defect list
- Corpus execution logs

Validation:

- All corpus entries processed or explicitly marked unsupported.
- Metrics computed from ground truth.
- Pass/fail status assigned for each corpus.

Exit Criteria:

- Accuracy matrix passes Section 8 targets or release is blocked.
- All known misses have owner, severity, and target phase.

Risks:

- Corpus may not represent field data.
- Ground truth may be wrong.
- Unsupported cases may be hidden as passes.

Blockers:

- Missing ground truth.
- Any P0 false negative in required corpus.
- Any metric without pass/fail result.

Owner:

- QA Owner

### Phase 7: Reproducibility Validation

Purpose:

- Prove repeated analysis produces equivalent core outputs.

Scope:

- Same-host reruns.
- Cross-platform comparison when Windows and macOS runners are available.
- Deterministic output comparison.

Inputs:

- Phase 6 corpus and outputs.
- Normalized diff allowlist.
- Tool version records.

Entry Criteria:

- Phase 6 complete.
- Reproducibility comparator defined.
- Allowed variable fields documented.

Tasks:

1. Run each required corpus at least three times.
2. Normalize allowed variable fields.
3. Compare DB rows, hashes, source paths, artifact counts, validation status, and report summary values.
4. Compare package manifests.
5. Compare macOS and Windows outputs when both are available.
6. File defects for drift.

Deliverables:

- `reproducibility-report.json`
- `reproducibility-report.html`
- Diff allowlist
- Drift defect list

Validation:

- Deterministic fields match the targets in Section 8.
- Allowed diffs are documented and bounded.
- Any drift has root-cause owner.

Exit Criteria:

- Reproducibility matrix passes.
- No unexplained deterministic drift remains.

Risks:

- Timestamps or path normalization may create noisy diffs.
- External tools may produce version-dependent metadata.

Blockers:

- Unexplained DB row drift.
- Hash drift for unchanged source inputs.
- Report summary drift outside allowed limits.

Owner:

- QA Owner

### Phase 8: Report Defensibility

Purpose:

- Ensure reports are complete, transparent, reviewable, and report-defensible.

Scope:

- HTML report.
- Evidence package manifest.
- Audit logs.
- Export summaries.
- Report language.

Inputs:

- Phase 6 and Phase 7 reports.
- Legal/review policy.
- Forensic report checklist.

Entry Criteria:

- Phase 7 complete.
- Legal Reviewer assigned.
- Forensic Lead assigned.

Tasks:

1. Implement or run `frametrace qa report-defense`.
2. Verify reports include:
   - Case metadata
   - Operator metadata when provided
   - FrameTrace version
   - OS and external tool versions
   - Command options
   - Source path and evidence hash when available
   - Output artifact hashes
   - Analysis start and finish times
   - Failed, skipped, missing, partial, unsupported, and unverifiable items
   - Source/recovered/carved/proxy/thumbnail/clip/report relationships
   - E01 partition, inode, offset, or carving offset where available
   - Audit-chain verification status
   - Known limitations
3. Replace disallowed legal claims with report-defensible wording.
4. Run technical review, operator review, and legal review.

Deliverables:

- `report-defense-checklist.md`
- `report-defense-report.json`
- Legal review notes
- Operator review notes

Validation:

- Checklist items pass/fail.
- Disallowed wording scan passes.
- Reviewers sign off or block release.

Exit Criteria:

- Technical Review approved.
- Operator Review approved.
- Legal Review approved or explicitly scopes release language.
- No disallowed legal wording remains.

Risks:

- Reports may overstate validation.
- Known limitations may be incomplete.
- Operator-facing language may be confusing.

Blockers:

- Any disallowed legal wording in release docs or reports.
- Missing failure/limitation disclosure.
- Missing artifact provenance.

Owner:

- Forensic Lead

### Phase 9: Large Scale Survival

Purpose:

- Demonstrate that large cases do not exhaust memory, freeze UI, or create unbounded runtimes.

Scope:

- SQLite scale.
- Viewer scale.
- Search/filter latency.
- Large file inventory layout and virtualized review workflow.
- Package creation.
- Resume/retry behavior.
- Large raw/E01-derived datasets.

Inputs:

- Phase 8 approved report structure.
- Performance corpus.
- Measurement harness.

Entry Criteria:

- Phase 8 complete.
- Performance Owner assigned.
- Target hardware profile documented.
- Dataset tier selected.

Tasks:

1. Run 10,000-file CI-friendly scale test.
2. Run 100,000-file manual/nightly test.
3. Run 1,000,000-row synthetic SQLite test.
4. Run large raw/E01-derived dataset test when available.
5. Measure RSS, CPU, throughput, query latency, viewer P95/P99 latency, package time, and completion success.
6. Verify pagination/virtualization for large viewer datasets.
7. Verify the large file inventory requirements in `docs/EVIDENCE_VIEWER_GUI.md`: dense rows, grouped source drill-down, stable sort, composable filters, bulk preview, row-state preservation, and engine-backed search.
8. Execute the implementation sequence in `docs/GUI_LARGE_INVENTORY_EXECUTION_PLAN.md` and attach its QA evidence.
9. File defects for metrics outside thresholds.

Deliverables:

- `performance-report.md`
- `performance-report.json`
- Query-plan evidence
- `docs/GUI_LARGE_INVENTORY_EXECUTION_PLAN.md`
- `docs/gui-large-inventory-traceability.md`
- Large file inventory QA report
- Large-case failure log

Validation:

- Metrics from Section 8 are pass/fail.
- Memory use remains within target.
- Viewer does not load full large JSON indexes into memory.
- Viewer satisfies the large file inventory acceptance criteria in `docs/EVIDENCE_VIEWER_GUI.md`.
- Every `docs/gui-large-inventory-traceability.md` requirement is passed or explicitly release-scoped down.
- Large-case failure mode is recoverable or release-blocked.

Exit Criteria:

- Large-scale matrix passes or release scope is reduced.
- Performance Owner approves results.
- Release Manager records supported scale claims.

Risks:

- Synthetic rows may not represent real media scan cost.
- Large evidence may reveal OS-specific file-system behavior.
- UI may pass small tests and fail large cases.

Blockers:

- Out-of-memory behavior.
- Viewer freeze beyond thresholds.
- Unbounded package creation.
- Unsupported scale claim without evidence.

Owner:

- Performance Owner

### Phase 10: Release Readiness

Purpose:

- Decide whether the hardened tool can be released with the claimed scope.

Scope:

- Release blockers.
- Evidence bundle.
- Release notes.
- Known limitations.
- Go/no-go decision.

Inputs:

- Completed Phase 0-9 deliverables.
- Release blocker checklist.
- Risk register.

Entry Criteria:

- Phase 9 complete.
- Release Manager assigned.
- All release blockers have an owner.

Tasks:

1. Verify release blockers in Section 12.
2. Assemble release evidence bundle.
3. Confirm no P0/P1 issue remains open.
4. Confirm P2 issues are documented and accepted.
5. Confirm supported scope and unsupported scope are explicit.
6. Produce release decision record.

Deliverables:

- Release readiness checklist
- Release notes
- Known limitations
- Go/no-go decision record

Validation:

- All blocker checkboxes resolved.
- Review signoffs recorded.
- Release scope matches validation evidence.

Exit Criteria:

- Release Manager marks release `GO`, `NO-GO`, or `GO WITH SCOPE REDUCTION`.
- No mandatory blocker is bypassed.

Risks:

- Business pressure may push release before validation is complete.
- Unsupported features may be implied by marketing or docs.

Blockers:

- Any unchecked mandatory release blocker.
- Any unresolved P0/P1 defect.
- Missing legal/operator review.

Owner:

- Release Manager

### Phase 11: Feature Expansion

Purpose:

- Add new features only after the hardened base is validated and release scope is clear.

Scope:

- E01/raw quick-analysis workflow.
- Deleted-file and recovery candidate triage UI.
- Large evidence viewer with virtual lists.
- Search, filtering, and timeline views.
- HTML, CSV, JSON, ZIP, and later PDF exports.
- Case dashboard.
- Windows production shell.
- Future app-forensics plugin architecture.

Inputs:

- Phase 10 release decision.
- Product backlog.
- Risk register.
- Corpus and performance results.

Entry Criteria:

- Phase 10 complete.
- Product Owner prioritizes feature backlog.
- Feature PRD and test spec exist for each major feature.

Tasks:

1. Classify features as:
   - `READY`
   - `BACKLOG`
   - `DESIGN CANDIDATE`
   - `BLOCKED`
2. Require PRD and test spec for each `READY` feature.
3. Add feature-specific corpus or fixtures before implementation.
4. Add report-defensible wording for user-facing claims.
5. Re-run applicable validation gates before release.

Deliverables:

- Feature PRDs
- Feature test specifications
- Updated backlog
- Updated release scope

Validation:

- Every feature has testable acceptance criteria.
- Any recovery feature satisfies Phase 5 governance.
- Any scale-sensitive feature satisfies Phase 9 targets.

Exit Criteria:

- Feature ships only with tests, docs, and validation evidence.
- Backlog remains separated from implemented claims.

Risks:

- Feature expansion may undermine hardened base.
- Recovery features may outpace corpus and ground truth.

Blockers:

- Missing PRD.
- Missing test spec.
- Missing corpus for forensic feature.
- Unsupported legal/report claim.

Owner:

- Product Owner

## 7. Recovery Governance

Recovery implementation must not begin until all of the following are complete:

1. Recovery PRD approved.
2. Recovery Test Specification approved.
3. Validation corpus prepared.
4. Ground truth dataset available.
5. Recovery labels and report language approved.
6. Existing recovery limitations documented.

Before these gates are complete:

- New deleted-file recovery features are `BACKLOG`.
- New carving formats are `DESIGN CANDIDATE`.
- New bulk recovery workflows are `BACKLOG`.
- New proprietary parser recovery claims are `DESIGN CANDIDATE`.
- Existing recovery functionality may receive bug fixes, provenance fixes, labeling fixes, and report-defensibility fixes only.

Recovery approval checklist:

- [ ] PRD approved by Product Owner and Forensic Lead
- [ ] Test spec approved by QA Owner
- [ ] Corpus manifest approved by Forensic Lead
- [ ] Ground truth approved by QA Owner
- [ ] Report labels approved by Legal Reviewer
- [ ] Unsupported cases documented

## 8. Quantitative Validation Matrix

All metrics are pass/fail. If a metric cannot be measured, the phase fails until the metric is either measured or formally removed by the Release Manager with written scope reduction.

### Accuracy Metrics

| Metric | Target | Pass/Fail Rule | Owner |
| --- | --- | --- | --- |
| Precision | >= 0.98 on required corpus items | Pass if true positives / predicted positives >= 0.98 | QA Owner |
| Recall | >= 0.98 on required corpus items | Pass if true positives / ground-truth positives >= 0.98 | QA Owner |
| False positive threshold | <= 2% of predicted positives | Pass if false positives / predicted positives <= 0.02 | QA Owner |
| False negative threshold | <= 2% of ground-truth positives | Pass if false negatives / ground-truth positives <= 0.02 | QA Owner |
| P0 false negatives | 0 for required evidence types | Pass only if no required corpus P0 item is missed | Forensic Lead |
| Unsupported item disclosure | 100% disclosed | Pass if every unsupported item is labeled unsupported with reason | Forensic Lead |

### Reproducibility Metrics

| Metric | Target | Pass/Fail Rule | Owner |
| --- | --- | --- | --- |
| Deterministic rerun equivalence | 100% for normalized core outputs | Pass if normalized DB rows, source IDs, hashes, validation statuses, and report summaries match | QA Owner |
| Allowed output diff | <= 1% of fields and only allowlisted fields | Pass if non-allowlisted fields have 0 diff and allowlisted fields <= 1% | QA Owner |
| Package manifest deterministic fields | 100% match | Pass if normalized package manifests match | QA Owner |
| Cross-platform core result equivalence | >= 99.5% when both OS runs are available | Pass if normalized core outputs differ by <= 0.5% and differences are explained | QA Owner |

### Performance Metrics

| Metric | Target | Pass/Fail Rule | Owner |
| --- | --- | --- | --- |
| Max RSS memory, 10k files | <= 1.0 GiB | Pass if peak resident memory <= target | Performance Owner |
| Max RSS memory, 100k files | <= 2.5 GiB | Pass if peak resident memory <= target | Performance Owner |
| Max RSS memory, 1M rows synthetic | <= 4.0 GiB | Pass if peak resident memory <= target | Performance Owner |
| Max CPU utilization | <= 95% average over 5 minutes unless user requested maximum throughput | Pass if average CPU <= target | Performance Owner |
| Scan throughput, no hash/no ffprobe | >= 5,000 files/minute on target hardware | Pass if throughput >= target | Performance Owner |
| SQLite insert throughput | >= 50,000 rows/minute on target hardware | Pass if throughput >= target | DB Owner |
| SQLite indexed query latency | <= 2 seconds for 1M rows | Pass if representative indexed queries return within target | DB Owner |

### UI Metrics

| Metric | Target | Pass/Fail Rule | Owner |
| --- | --- | --- | --- |
| Viewer initial render, 10k rows | P95 <= 2 seconds | Pass if P95 initial render <= target | UX Owner |
| Viewer search latency, 100k rows | P95 <= 1 second | Pass if P95 search response <= target | UX Owner |
| Viewer search latency, 1M rows | P99 <= 3 seconds with SQLite-backed search | Pass if P99 search response <= target | UX Owner |
| List scroll frame stability | P95 frame time <= 50 ms for virtualized list | Pass if measured P95 <= target | UX Owner |
| Visible inventory density, 1440 px | >= 12 visible rows with default layout | Pass if default layout shows at least 12 rows without hiding viewer/inspector | UX Owner |
| Visible inventory density, inventory-focused mode | >= 30 visible rows | Pass if focused mode shows at least 30 rows and preserves selection/filter state | UX Owner |
| UI freeze threshold | 0 freezes > 5 seconds | Pass if no interaction block exceeds threshold | UX Owner |

### Large Scale Metrics

| Metric | Target | Pass/Fail Rule | Owner |
| --- | --- | --- | --- |
| Target evidence size | >= 1 TiB raw/E01-derived dataset tested before 1 TiB support claim | Pass if dataset completes or claim is reduced | Performance Owner |
| Target file count | 100,000 real or fixture files and 1,000,000 synthetic DB rows | Pass if both tiers complete | Performance Owner |
| Completion success rate | >= 99% for required corpus jobs | Pass if completed jobs / required jobs >= 0.99 | QA Owner |
| Resume/retry success | >= 95% of induced interruptions resume or fail safely | Pass if resumed-or-safe-failed jobs / interrupted jobs >= 0.95 | QA Owner |
| Package generation success | 100% for complete cases; 0 silent incomplete packages | Pass if complete cases package and incomplete cases fail or disclose missing optional files | Release Manager |

## 9. Forensic Corpus Definition

Every corpus must have a manifest, immutable ground truth, expected outputs, and pass criteria. Corpus contents should be stored under a documented fixture location or external evidence vault. Large or sensitive corpora may be referenced by manifest only.

### Corpus A: Deleted File Recovery

Purpose:

- Validate deleted-file listing, inode recovery, carving labels, offsets, and validation status.

Ground truth source:

- Prepared raw/E01 images with known deleted files, known inode IDs, known offsets, known hashes, and known recovery status.

Expected outputs:

- Deleted entries listed.
- Selected recoveries exported.
- Recovered artifacts labeled `candidate-unvalidated` until validation.
- Source image, partition offset, inode or carving offset, output hash, and validation result recorded.

Pass criteria:

- Required deleted files detected with recall >= 0.98.
- No recovered artifact lacks provenance fields.
- Unsupported recovery cases are explicitly labeled unsupported.

### Corpus B: Browser Artifacts

Purpose:

- Validate future browser-artifact ingestion boundaries without claiming unsupported parsing.

Ground truth source:

- Prepared browser profile fixtures with known downloads, history entries, cache media, and timestamps.

Expected outputs:

- Current phase: artifacts are BACKLOG/DESIGN CANDIDATE unless parser PRD exists.
- When implemented: downloads/history/cache media are detected with source path, timestamp, and profile context.

Pass criteria:

- Before implementation: all browser parsing is marked BACKLOG or DESIGN CANDIDATE.
- After implementation approval: precision and recall meet Section 8 targets against fixture ground truth.

### Corpus C: Windows Event Logs

Purpose:

- Validate future Windows timeline context ingestion boundaries.

Ground truth source:

- Prepared EVTX fixtures with known device attach, login, file access, and application execution events.

Expected outputs:

- Current phase: event-log parsing is BACKLOG/DESIGN CANDIDATE unless PRD exists.
- When implemented: parsed events include event ID, timestamp, source log, host, and normalized timeline fields.

Pass criteria:

- Before implementation: no release claim says Windows Event Logs are parsed.
- After implementation approval: required events meet Section 8 precision/recall targets.

### Corpus D: Timeline Reconstruction

Purpose:

- Validate cross-source timeline ordering and disclosure of timestamp conflicts.

Ground truth source:

- Mixed fixture with known file modified times, metadata times, recovery times, and external event times.

Expected outputs:

- Timeline entries ordered deterministically.
- Conflicting timestamps flagged.
- Missing timestamps grouped separately.
- Timezone assumptions recorded.

Pass criteria:

- Required timeline entries present with recall >= 0.98.
- Timestamp conflicts are disclosed.
- Normalized repeated runs produce deterministic timeline output.

### Corpus E: Large Evidence Dataset

Purpose:

- Validate large-case performance, memory, query, package, and viewer behavior.

Ground truth source:

- Synthetic 1,000,000-row DB fixture plus large folder/raw/E01-derived fixture with known counts and hashes.

Expected outputs:

- Complete scan/index or documented safe failure.
- SQLite-backed filtering and pagination.
- Viewer does not load entire large JSON index.
- Performance report generated.

Pass criteria:

- Section 8 performance, UI, and large-scale metrics pass.
- Completion success rate >= target.
- No out-of-memory failure.

### Corpus F: Mixed Real-World Case Dataset

Purpose:

- Validate realistic mixed evidence behavior across video, recovery candidates, logs, reports, package generation, and operator review.

Ground truth source:

- Sanitized real-world or representative mixed case with approved ground truth manifest and privacy review.

Expected outputs:

- Case dashboard summary.
- Video index.
- Recovery candidate list.
- Validation status.
- Report-defensible case report.
- Evidence package manifest.

Pass criteria:

- Required outputs generated.
- Privacy leakage review passes.
- Report-defensibility checklist passes.
- Known limitations disclosed.

## 10. Database Migration Contract

### Schema Version Rules

1. `SCHEMA_VERSION` must increase for any released schema change that changes tables, columns, constraints, indexes that affect behavior, or migration expectations.
2. Additive non-breaking indexes may share a patch migration but must still be recorded.
3. Destructive changes require a major migration entry and fixture validation.
4. Unknown or future schema versions must fail closed with a clear error.

### Migration Naming Convention

Use ordered, descriptive names:

- `migration_v001_to_v002_<short_reason>`
- `migration_v002_to_v003_<short_reason>`

Examples:

- `migration_v001_to_v002_add_timeline_indexes`
- `migration_v002_to_v003_remove_verified_unused_columns`

### Ownership

- DB Owner owns design and implementation.
- QA Owner owns fixture validation.
- Release Manager owns release approval.

### Rollback Strategy

1. Before migration, create a copy of the case DB.
2. If migration fails, do not modify the original DB further.
3. Restore from backup or instruct operator to use backup path.
4. Record migration failure in logs when possible.
5. Never silently continue on partial migration.

### Backup Naming

Backup files must use:

```text
case.db.backup-v{from_version}-to-v{to_version}-{unix_timestamp}
```

Example:

```text
case.db.backup-v1-to-v2-1770000000
```

### Fixture Locations

Recommended locations:

```text
tests/fixtures/db/v1/case.db
tests/fixtures/db/v1/manifest.json
tests/fixtures/db/v2/expected_manifest.json
```

If fixture DBs are too large, store a generation script:

```text
tests/fixtures/db/generate_v1_fixture.rs
```

### Migration Verification Procedure

1. Open v1 fixture read-only and record row counts.
2. Copy fixture to temp directory.
3. Run migration.
4. Verify target schema version.
5. Verify row counts.
6. Verify required columns and data values.
7. Verify required indexes exist.
8. Run representative queries.
9. Run integrity check.
10. Verify backup exists.
11. Simulate failure path when possible and verify original DB is recoverable.

### Post-Migration Validation Checklist

- [ ] `schema_meta` has expected version
- [ ] SQLite integrity check passes
- [ ] Required tables exist
- [ ] Removed tables/columns have deletion evidence
- [ ] Required row counts match
- [ ] Required hashes and source paths survive
- [ ] Query-plan checks pass
- [ ] Backup file exists
- [ ] Rollback instructions tested
- [ ] Migration result documented in release evidence

## 11. Legal And Report-Defensibility Policy

Allowed language:

- `report-defensible`
- `reproducible analysis record`
- `validated against the defined QA corpus`
- `candidate-unvalidated`
- `unsupported`
- `known limitation`

Disallowed language:

- Any phrase that claims guaranteed legal readiness
- Any phrase that claims legal superiority of validation
- Any phrase that claims legal proof
- Any claim that legal admissibility is guaranteed
- Any claim that unsupported formats are fully recovered

Report policy:

1. Reports must state what was analyzed.
2. Reports must state what was not analyzed.
3. Reports must state what failed, was skipped, was partial, or was unsupported.
4. Reports must include provenance for source and derived artifacts.
5. Reports must include tool versions and options.
6. Reports must not hide validation failures.

## 12. Release Blockers

No release may bypass these blockers.

- [ ] Technical Review
- [ ] Security Review
- [ ] Accuracy Validation
- [ ] Reproducibility Validation
- [ ] Migration Validation
- [ ] Operator Review
- [ ] Legal Review

Blocking rules:

1. Any unchecked blocker means `NO-GO`.
2. Any unresolved P0/P1 issue means `NO-GO`.
3. Any missing migration fixture for a schema change means `NO-GO`.
4. Any unsupported feature advertised as supported means `NO-GO`.
5. Any disallowed legal wording means `NO-GO`.
6. Any silent incomplete package behavior means `NO-GO`.
7. Any evidence-corrupting behavior means `NO-GO`.

## 13. Deliverable Inventory

| Phase | Deliverable | Owner | Validation Required | Completion Evidence |
| --- | --- | --- | --- | --- |
| 0 | `docs/static-analysis.md` | Engineering Lead | Baseline commands run | Command results and issue list |
| 1 | `docs/cleanup-review.md` | Engineering Lead | Static checks and tests pass | Symbol matrix and cleanup commits |
| 2 | `docs/security-review.md` | Security Owner | Security findings triaged | Finding register and signoff |
| 3 | `docs/schema.md` | DB Owner | Table/column/index matrix complete | Query-plan evidence |
| 4 | Migration tests and backup procedure | DB Owner | Fixture migration and rollback pass | Migration report |
| 5 | `docs/recovery-prd.md` | Forensic Lead | PRD/test spec/corpus approved | Approved governance checklist |
| 5 | `docs/recovery-test-spec.md` | QA Owner | Ground truth available | Corpus manifest |
| 6 | `accuracy-report.json/html` | QA Owner | Accuracy metrics pass | Corpus result report |
| 7 | `reproducibility-report.json/html` | QA Owner | Deterministic diff metrics pass | Rerun comparison report |
| 8 | `report-defense-checklist.md` | Forensic Lead | Technical/operator/legal review | Signed review notes |
| 9 | `performance-report.md/json` | Performance Owner | Scale metrics pass | Performance logs and charts |
| 10 | Release readiness checklist | Release Manager | All release blockers checked | Go/no-go record |
| 11 | Feature PRDs and test specs | Product Owner | Feature-specific gates pass | Feature acceptance evidence |

## 14. Risk Register

| Risk | Description | Probability | Impact | Mitigation | Owner |
| --- | --- | --- | --- | --- | --- |
| False positives | Tool identifies non-evidence as evidence. | Medium | High | Ground-truth corpus, precision target, operator review. | QA Owner |
| False negatives | Tool misses required evidence. | Medium | Critical | Recall target, P0 false-negative rule, corpus expansion. | Forensic Lead |
| Migration failure | Schema change corrupts or blocks existing case DB. | Medium | Critical | Backup, fixture migration, rollback test, fail-closed version check. | DB Owner |
| Dataset contamination | Generated outputs are re-scanned as source evidence. | Medium | Critical | Case-dir exclusion, contamination regression tests, warnings. | Engineering Lead |
| Reproducibility drift | Repeated runs produce unexplained differences. | Medium | High | Normalized diff, rerun tests, tool-version recording. | QA Owner |
| Evidence corruption | Tool writes into source evidence path. | Low | Critical | Source/output path separation, write-protection warnings, security review. | Security Owner |
| Privacy leakage | Reports expose sensitive local paths or unrelated user data. | Medium | High | Privacy review, redaction options, report-defensibility checklist. | Security Owner |
| Legal review failure | Report wording overstates capability or admissibility. | Medium | High | Disallowed wording scan, Legal Review blocker. | Legal Reviewer |
| Large-case memory failure | Viewer or report loads too much data. | High | High | SQLite-backed pagination, virtual list, memory thresholds. | Performance Owner |
| External tool drift | ffmpeg/libewf/tsk versions change output. | Medium | Medium | Tool version recording, normalized comparisons, version-specific notes. | Engineering Lead |
| Ground truth error | Corpus expected results are wrong. | Low | High | Dual review by QA and Forensic Lead, manifest hashes. | QA Owner |
| Unsupported feature claim | Documentation implies features that are only backlog/design candidates. | Medium | High | Release scope review and blocker checklist. | Product Owner |

## 15. Audit Findings Against Previous Plan

### A. Missing Items

The previous plan was missing:

1. Formal entry criteria per phase.
2. Formal exit criteria per phase.
3. Phase owners.
4. Quantitative pass/fail metrics.
5. Release blockers.
6. Recovery governance gate.
7. Database migration contract.
8. Rollback procedure.
9. Forensic corpus definitions.
10. Risk register.
11. Deliverable inventory.
12. Security review phase.
13. Release readiness phase.

### B. Weak Areas

1. Phase ordering was mostly implied, not canonical.
2. Accuracy and reproducibility were directionally described but not measurable.
3. Large-scale behavior lacked memory, CPU, latency, and completion thresholds.
4. DB deletion rules existed but lacked fixture, backup, rollback, and ownership rules.
5. Legal/report language was cautious but not enforceable through blockers.

### C. Remaining Risks

1. Metric targets may need tuning after first real benchmark runs.
2. Corpus A-F must still be built and approved.
3. Windows-specific behavior still requires real Windows validation.
4. Browser artifacts and Windows Event Logs remain design candidates until PRDs exist.
5. New recovery work must remain blocked until Phase 5 is complete.

### D. Readiness Score

Plan readiness score: 92 / 100.

Rationale:

- Mandatory planning sections now exist.
- Execution ordering is canonical.
- Phase template is complete.
- Release blockers are explicit.
- Remaining score gap is due to corpus, fixture, and benchmark data not yet existing.

### E. Recommendation

Classification: Execution Ready.

Recommendation:

- Start with Phase 0 only.
- Do not start implementation, migration, or recovery expansion until the relevant phase gates are satisfied.
- Treat Phase 5 as the hard stop for all new recovery work until PRD, test specification, corpus, and ground truth are approved.
