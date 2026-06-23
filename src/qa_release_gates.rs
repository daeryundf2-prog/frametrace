pub(crate) const REVIEW_GATES: &[(&str, &str)] = &[
    ("technical_review", "Technical Review"),
    ("security_review", "Security Review"),
    ("privacy_review", "Privacy Review"),
    ("supply_chain_review", "Supply-chain Review"),
    ("accuracy_validation", "Accuracy Validation"),
    ("reproducibility_validation", "Reproducibility Validation"),
    ("performance_validation", "Performance Validation"),
    ("migration_validation", "Migration Validation"),
    ("operator_review", "Operator Review"),
    ("report_defensibility_review", "Report-defensibility Review"),
    ("legal_wording_review", "Legal Wording Review"),
    (
        "installer_package_validation",
        "Installer/Package Validation",
    ),
    (
        "windows_workstation_validation",
        "Windows Workstation Validation",
    ),
    ("known_limitations_review", "Known Limitations Review"),
    ("release_notes_review", "Release Notes Review"),
    ("support_triage_policy", "Support/Triage Policy"),
    ("hotfix_policy", "Hotfix Policy"),
    ("incident_response_plan", "Incident Response Plan"),
    ("corpus_governance", "Corpus Governance"),
    ("feature_intake_governance", "Feature Intake Governance"),
    ("post_ga_monitoring", "Post-GA Monitoring"),
    ("external_review_readiness", "External Review Readiness"),
    ("regression_schedule", "Regression Schedule"),
];

#[cfg(test)]
mod tests {
    use super::REVIEW_GATES;
    use std::collections::HashSet;

    #[test]
    fn global_release_gate_keys_are_unique_and_complete() {
        let keys = REVIEW_GATES
            .iter()
            .map(|(key, _)| *key)
            .collect::<HashSet<_>>();

        assert_eq!(keys.len(), REVIEW_GATES.len());
        assert!(keys.contains("privacy_review"));
        assert!(keys.contains("incident_response_plan"));
        assert!(keys.contains("post_ga_monitoring"));
        assert!(keys.contains("external_review_readiness"));
        assert!(keys.contains("regression_schedule"));
    }
}
