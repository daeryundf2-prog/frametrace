#[path = "qa_accuracy.rs"]
mod qa_accuracy;
#[path = "qa_release.rs"]
mod qa_release;
#[path = "qa_release_decision.rs"]
mod qa_release_decision;
#[path = "qa_release_gates.rs"]
mod qa_release_gates;
#[path = "qa_release_manifest.rs"]
mod qa_release_manifest;
#[cfg(test)]
#[path = "qa_release_manifest_tests.rs"]
mod qa_release_manifest_tests;
#[path = "qa_report_defense.rs"]
mod qa_report_defense;
#[path = "qa_repro.rs"]
mod qa_repro;
#[path = "qa_repro_json.rs"]
mod qa_repro_json;
#[path = "qa_shell_contract.rs"]
mod qa_shell_contract;
#[cfg(test)]
#[path = "qa_test_fixtures.rs"]
mod qa_test_fixtures;
#[cfg(test)]
#[path = "qa_tests/mod.rs"]
mod qa_tests;

pub use crate::performance_qa::performance_report;
#[cfg(test)]
pub(crate) use crate::performance_qa::performance_report_for_test;
pub use qa_accuracy::accuracy_report;
pub use qa_release::release_readiness_report;
pub(crate) use qa_release_gates::REVIEW_GATES;
#[cfg(test)]
pub(crate) use qa_release_manifest::read_review_manifest;
pub use qa_report_defense::privacy_review_check;
pub use qa_report_defense::report_defense_check;
pub use qa_repro::reproducibility_report;
pub(crate) use qa_shell_contract::workstation_shell_contract_check;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct QaReport {
    pub report_path: PathBuf,
    pub passed: bool,
}

#[derive(Debug, Clone)]
pub struct ReleaseReadinessOptions {
    pub corpus_manifest: Option<PathBuf>,
    pub comparison_case_dir: Option<PathBuf>,
    pub review_manifest: Option<PathBuf>,
    pub performance_output_dir: Option<PathBuf>,
    pub performance_rows: usize,
}
