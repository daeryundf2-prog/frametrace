# Release Readiness

Overall: **BLOCKED**

| Check | Status | Evidence |
| --- | --- | --- |
| report_defense | FAIL | `report defensibility QA failed: missing 6 required artifacts, 0 disallowed claim(s), 0 audit chain failure(s), 0 active job(s)` |
| workstation_shell_contract | FAIL | `failed to read case manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/case/case.json: No such file or directory (os error 2)` |
| windows_prerequisites | FAIL | `windows_prerequisites failed: unsupported-host, missing-tool:dotnet, missing-winui-project, missing-winui-build-receipt; see .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/out-text/windows-prerequisites.json` |
| technical_review | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| security_review | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| privacy_review | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| supply_chain_review | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| accuracy_validation | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| reproducibility_validation | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| performance_validation | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| migration_validation | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| operator_review | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| report_defensibility_review | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| legal_wording_review | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| installer_package_validation | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| windows_workstation_validation | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| known_limitations_review | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| release_notes_review | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| support_triage_policy | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| hotfix_policy | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| incident_response_plan | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| corpus_governance | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| feature_intake_governance | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| post_ga_monitoring | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| external_review_readiness | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| regression_schedule | FAIL | `release review manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/text-only-review.txt must be a typed JSON review manifest: expected ident at line 1 column 2` |
| accuracy | BLOCKED | `missing --corpus-manifest` |
| reproducibility | BLOCKED | `missing --comparison-case` |
| performance | PASS | `.omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/out-text/performance/performance-report.json` |
