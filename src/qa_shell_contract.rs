use crate::util::write_text;
use crate::workstation;
use std::path::{Path, PathBuf};

const REQUIRED_TOKENS: &[(&str, &str)] = &[
    ("view", "\"view\":\"workstation-status\""),
    ("engine source of truth", "\"engine_source_of_truth\":true"),
    (
        "GUI durable state disabled",
        "\"gui_durable_state_allowed\":false",
    ),
    (
        "bounded inventory transport",
        "\"transport\":\"sqlite-bounded-query\"",
    ),
    (
        "full JSON load disabled",
        "\"full_json_load_allowed\":false",
    ),
    (
        "separate validation states",
        "\"ffprobe_and_playback_are_separate_states\":true",
    ),
    ("playback confirmation command", "\"confirm-playback\""),
    (
        "large-case JSON ban",
        "\"large_case_full_json_load_allowed\":false",
    ),
    (
        "engine-only durable mutation",
        "\"durable_mutation\":\"engine-command-only\"",
    ),
    ("Windows prerequisite status", "\"windows_prerequisites\":{"),
    (
        "source of durable state",
        "\"state_owner\":\"rust-engine-sqlite-audit\"",
    ),
];

pub(crate) fn workstation_shell_contract_check(
    case_dir: &Path,
    output_dir: &Path,
) -> Result<PathBuf, String> {
    let status_json = workstation::workstation_status_json(case_dir)?;
    validate_workstation_shell_contract(&status_json)?;
    let output_path = output_dir.join("workstation-status.json");
    write_text(&output_path, &status_json)
        .map_err(|err| format!("failed to write workstation status evidence: {err}"))?;
    Ok(output_path)
}

fn validate_workstation_shell_contract(status_json: &str) -> Result<(), String> {
    for (label, token) in REQUIRED_TOKENS {
        if !status_json.contains(token) {
            return Err(format!(
                "workstation shell contract failed: missing {label} token {token}"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_workstation_shell_contract;

    #[test]
    fn workstation_contract_accepts_required_tokens() {
        let status = concat!(
            "{\"view\":\"workstation-status\",",
            "\"engine_source_of_truth\":true,",
            "\"gui_durable_state_allowed\":false,",
            "\"transport\":\"sqlite-bounded-query\",",
            "\"full_json_load_allowed\":false,",
            "\"ffprobe_and_playback_are_separate_states\":true,",
            "\"engine_commands\":[\"confirm-playback\"],",
            "\"windows_prerequisites\":{},",
            "\"winui_contract\":{",
            "\"large_case_full_json_load_allowed\":false,",
            "\"durable_mutation\":\"engine-command-only\",",
            "\"state_owner\":\"rust-engine-sqlite-audit\"}}"
        );

        assert!(validate_workstation_shell_contract(status).is_ok());
    }

    #[test]
    fn workstation_contract_rejects_full_json_load() {
        let status = concat!(
            "{\"view\":\"workstation-status\",",
            "\"engine_source_of_truth\":true,",
            "\"gui_durable_state_allowed\":false,",
            "\"transport\":\"sqlite-bounded-query\",",
            "\"full_json_load_allowed\":true,",
            "\"ffprobe_and_playback_are_separate_states\":true,",
            "\"engine_commands\":[\"confirm-playback\"],",
            "\"windows_prerequisites\":{},",
            "\"winui_contract\":{",
            "\"large_case_full_json_load_allowed\":false,",
            "\"durable_mutation\":\"engine-command-only\",",
            "\"state_owner\":\"rust-engine-sqlite-audit\"}}"
        );
        let err = validate_workstation_shell_contract(status).unwrap_err();

        assert!(err.contains("full JSON load disabled"));
    }
}
