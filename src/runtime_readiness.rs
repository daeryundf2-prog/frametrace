use crate::case_db;
use crate::util::json_escape;
use crate::windows_prerequisites;
use std::path::Path;

struct FeatureGate {
    feature: &'static str,
    commands: &'static [&'static str],
    required_tools: &'static [&'static str],
}

struct DiskPreflight {
    feature: &'static str,
    output_path: &'static str,
}

const FEATURE_GATES: &[FeatureGate] = &[
    FeatureGate {
        feature: "import",
        commands: &["inspect-e01", "import-e01"],
        required_tools: &["ewfinfo", "ewfverify", "ewfexport"],
    },
    FeatureGate {
        feature: "carve",
        commands: &["carve-file"],
        required_tools: &[],
    },
    FeatureGate {
        feature: "filesystem-inspection",
        commands: &["inspect-image", "recover-inode"],
        required_tools: &["mmls", "fls", "icat"],
    },
    FeatureGate {
        feature: "validate-artifact",
        commands: &["validate-artifact"],
        required_tools: &["ffprobe"],
    },
    FeatureGate {
        feature: "proxy",
        commands: &["make-proxy", "make-thumbnail", "capture-frame"],
        required_tools: &["ffmpeg"],
    },
    FeatureGate {
        feature: "export",
        commands: &["export-video"],
        required_tools: &["ffmpeg"],
    },
    FeatureGate {
        feature: "package",
        commands: &["package-case"],
        required_tools: &[],
    },
];

const DISK_PREFLIGHTS: &[DiskPreflight] = &[
    DiskPreflight {
        feature: "import",
        output_path: "evidence/images",
    },
    DiskPreflight {
        feature: "carve",
        output_path: "artifacts/carved",
    },
    DiskPreflight {
        feature: "proxy",
        output_path: "artifacts/proxies",
    },
    DiskPreflight {
        feature: "export",
        output_path: "artifacts/clips",
    },
    DiskPreflight {
        feature: "package",
        output_path: "packages",
    },
];

pub fn runtime_readiness_json(case_dir: &Path) -> Result<String, String> {
    Ok(format!(
        "{{\"schema_version\":1,\"bounded_status\":true,\"jobs\":{},\
\"disk_preflight\":{},\"feature_gates\":{}}}",
        case_db::runtime_jobs_json(case_dir)?,
        disk_preflight_json(case_dir),
        feature_gates_json()
    ))
}

fn feature_gates_json() -> String {
    let gates = FEATURE_GATES
        .iter()
        .map(feature_gate_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{gates}]")
}

fn feature_gate_json(gate: &FeatureGate) -> String {
    let missing_tools = gate
        .required_tools
        .iter()
        .filter(|tool| !windows_prerequisites::command_available(tool))
        .copied()
        .collect::<Vec<_>>();
    let blockers = missing_tools
        .iter()
        .map(|tool| format!("missing-tool:{tool}"))
        .collect::<Vec<_>>();
    let status = if blockers.is_empty() {
        "available"
    } else {
        "blocked"
    };
    format!(
        "{{\"feature\":\"{}\",\"status\":\"{}\",\"commands\":{},\
\"required_tools\":{},\"missing_tools\":{},\"blockers\":{}}}",
        json_escape(gate.feature),
        status,
        str_array_json(gate.commands),
        str_array_json(gate.required_tools),
        str_array_json(&missing_tools),
        string_array_json(&blockers)
    )
}

fn disk_preflight_json(case_dir: &Path) -> String {
    let features = DISK_PREFLIGHTS
        .iter()
        .map(|preflight| disk_preflight_feature_json(case_dir, preflight))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"status\":\"blocked\",\"available_bytes\":null,\
\"blockers\":[\"required-bytes-unknown\"],\"features\":[{}]}}",
        features
    )
}

fn disk_preflight_feature_json(case_dir: &Path, preflight: &DiskPreflight) -> String {
    let path = case_dir.join(preflight.output_path);
    format!(
        "{{\"feature\":\"{}\",\"status\":\"blocked\",\"output_path\":\"{}\",\
\"available_bytes\":null,\"required_bytes\":null,\
\"blockers\":[\"required-bytes-unknown\",\"operator-must-confirm-free-space-before-run\"]}}",
        json_escape(preflight.feature),
        json_escape(&path.to_string_lossy())
    )
}

fn str_array_json(values: &[&str]) -> String {
    let items = values
        .iter()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn string_array_json(values: &[String]) -> String {
    let items = values
        .iter()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}
