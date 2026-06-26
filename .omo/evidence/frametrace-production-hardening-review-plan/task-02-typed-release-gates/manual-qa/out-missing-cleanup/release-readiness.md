# Release Readiness

Overall: **BLOCKED**

| Check | Status | Evidence |
| --- | --- | --- |
| report_defense | FAIL | `report defensibility QA failed: missing 6 required artifacts, 0 disallowed claim(s), 0 audit chain failure(s), 0 active job(s)` |
| workstation_shell_contract | FAIL | `failed to read case manifest .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/case/case.json: No such file or directory (os error 2)` |
| windows_prerequisites | FAIL | `windows_prerequisites failed: unsupported-host, missing-tool:dotnet, missing-winui-project, missing-winui-build-receipt; see .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/out-missing-cleanup/windows-prerequisites.json` |
| technical_review | FAIL | `review gate technical_review requires cleanup_status` |
| security_review | FAIL | `Security Review (security_review) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| privacy_review | FAIL | `Privacy Review (privacy_review) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| supply_chain_review | FAIL | `Supply-chain Review (supply_chain_review) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| accuracy_validation | FAIL | `Accuracy Validation (accuracy_validation) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| reproducibility_validation | FAIL | `Reproducibility Validation (reproducibility_validation) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| performance_validation | FAIL | `Performance Validation (performance_validation) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| migration_validation | FAIL | `Migration Validation (migration_validation) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| operator_review | FAIL | `Operator Review (operator_review) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| report_defensibility_review | FAIL | `Report-defensibility Review (report_defensibility_review) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| legal_wording_review | FAIL | `Legal Wording Review (legal_wording_review) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| installer_package_validation | FAIL | `Installer/Package Validation (installer_package_validation) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| windows_workstation_validation | FAIL | `Windows Workstation Validation (windows_workstation_validation) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| known_limitations_review | FAIL | `Known Limitations Review (known_limitations_review) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| release_notes_review | FAIL | `Release Notes Review (release_notes_review) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| support_triage_policy | FAIL | `Support/Triage Policy (support_triage_policy) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| hotfix_policy | FAIL | `Hotfix Policy (hotfix_policy) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| incident_response_plan | FAIL | `Incident Response Plan (incident_response_plan) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| corpus_governance | FAIL | `Corpus Governance (corpus_governance) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| feature_intake_governance | FAIL | `Feature Intake Governance (feature_intake_governance) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| post_ga_monitoring | FAIL | `Post-GA Monitoring (post_ga_monitoring) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| external_review_readiness | FAIL | `External Review Readiness (external_review_readiness) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| regression_schedule | FAIL | `Regression Schedule (regression_schedule) is not approved in .omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/missing-cleanup-review.json` |
| accuracy | BLOCKED | `missing --corpus-manifest` |
| reproducibility | BLOCKED | `missing --comparison-case` |
| performance | PASS | `.omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/out-missing-cleanup/performance/performance-report.json` |
