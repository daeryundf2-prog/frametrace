# Release Readiness

Overall: **BLOCKED**

| Check | Status | Evidence |
| --- | --- | --- |
| report_defense | FAIL | `report defensibility QA failed: missing 6 required artifacts, 0 disallowed claim(s), 0 audit chain failure(s), 0 active job(s)` |
| workstation_shell_contract | FAIL | `failed to read case manifest /tmp/frametrace-t1-empty-case.bc1ba9/case.json: No such file or directory (os error 2)` |
| windows_prerequisites | FAIL | `windows_prerequisites failed: unsupported-host, missing-tool:dotnet, missing-winui-project, missing-winui-build-receipt; see /tmp/frametrace-t1-empty-case.bc1ba9/reports/qa-review-audit/windows-prerequisites.json` |
| technical_review | BLOCKED | `missing --review-manifest` |
| security_review | BLOCKED | `missing --review-manifest` |
| privacy_review | BLOCKED | `missing --review-manifest` |
| supply_chain_review | BLOCKED | `missing --review-manifest` |
| accuracy_validation | BLOCKED | `missing --review-manifest` |
| reproducibility_validation | BLOCKED | `missing --review-manifest` |
| performance_validation | BLOCKED | `missing --review-manifest` |
| migration_validation | BLOCKED | `missing --review-manifest` |
| operator_review | BLOCKED | `missing --review-manifest` |
| report_defensibility_review | BLOCKED | `missing --review-manifest` |
| legal_wording_review | BLOCKED | `missing --review-manifest` |
| installer_package_validation | BLOCKED | `missing --review-manifest` |
| windows_workstation_validation | BLOCKED | `missing --review-manifest` |
| known_limitations_review | BLOCKED | `missing --review-manifest` |
| release_notes_review | BLOCKED | `missing --review-manifest` |
| support_triage_policy | BLOCKED | `missing --review-manifest` |
| hotfix_policy | BLOCKED | `missing --review-manifest` |
| incident_response_plan | BLOCKED | `missing --review-manifest` |
| corpus_governance | BLOCKED | `missing --review-manifest` |
| feature_intake_governance | BLOCKED | `missing --review-manifest` |
| post_ga_monitoring | BLOCKED | `missing --review-manifest` |
| external_review_readiness | BLOCKED | `missing --review-manifest` |
| regression_schedule | BLOCKED | `missing --review-manifest` |
| accuracy | BLOCKED | `missing --corpus-manifest` |
| reproducibility | BLOCKED | `missing --comparison-case` |
| performance | PASS | `/tmp/frametrace-t1-empty-case.bc1ba9/reports/qa-review-audit/performance/performance-report.json` |
