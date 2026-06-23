mod commands;
mod output_policy;

use crate::audit;
use crate::tool_policy::require_case_output_path;
use crate::util::{json_escape, now_unix, unique_path, write_text};
use commands::{
    default_raw_filename, ewf_command_version, ewfexport_args, ewfexport_target_for_output,
    expected_ewfexport_output, resolve_ewfexport_output, run_capture, run_status,
};
use output_policy::{append_e01_audit_at, e01_audit_log_path, require_e01_output_path};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct E01Options {
    pub output_path: Option<PathBuf>,
    pub max_bytes: Option<u64>,
    pub skip_verify: bool,
    pub hash_e01: bool,
    pub ewfinfo_bin: String,
    pub ewfverify_bin: String,
    pub ewfexport_bin: String,
}

impl Default for E01Options {
    fn default() -> Self {
        Self {
            output_path: None,
            max_bytes: None,
            skip_verify: false,
            hash_e01: false,
            ewfinfo_bin: "ewfinfo".to_string(),
            ewfverify_bin: "ewfverify".to_string(),
            ewfexport_bin: "ewfexport".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct E01ImportResult {
    pub e01_path: PathBuf,
    pub raw_output_path: PathBuf,
    pub ewfinfo_log_path: PathBuf,
    pub ewfverify_log_path: Option<PathBuf>,
    pub ewfexport_log_path: PathBuf,
    pub raw_sha256: String,
    pub e01_sha256: Option<String>,
}

pub fn inspect_e01(case_dir: &Path, e01_path: &Path, options: &E01Options) -> Result<(), String> {
    let e01_path = canonical_e01_path(e01_path)?;
    let inspected_unix = now_unix()?;
    let info_log_path = unique_path(
        &case_dir
            .join("evidence/logs")
            .join(format!("e01-info-{inspected_unix}.txt")),
    );
    require_e01_output_path(case_dir, &e01_path, &info_log_path, "E01 info log")?;
    let audit_log_path = e01_audit_log_path(case_dir, &e01_path)?;
    let info = run_capture(
        &options.ewfinfo_bin,
        &["ewfinfo"],
        &["-f", "text", &audit::path_string(&e01_path)],
    )?;
    write_text(&info_log_path, &info.stdout)
        .map_err(|err| format!("failed to write E01 info log: {err}"))?;

    let e01_sha256 = if options.hash_e01 {
        Some(audit::digest_file(&e01_path)?)
    } else {
        None
    };
    append_e01_audit_at(
        &audit_log_path,
        &format!(
            "{{\"schema_version\":1,\"event\":\"inspect-e01\",\"inspected_unix\":{},\"e01_path\":\"{}\",\"e01_sha256\":{},\"ewfinfo_version\":\"{}\",\"ewfinfo_log_path\":\"{}\"}}",
            inspected_unix,
            json_escape(&e01_path.to_string_lossy()),
            audit::optional_string(e01_sha256.as_deref()),
            json_escape(&ewf_command_version(&options.ewfinfo_bin, &["ewfinfo"])),
            json_escape(&info_log_path.to_string_lossy())
        ),
    )
}

pub fn import_e01(
    case_dir: &Path,
    e01_path: &Path,
    options: &E01Options,
) -> Result<E01ImportResult, String> {
    let e01_path = canonical_e01_path(e01_path)?;
    let imported_unix = now_unix()?;
    let info_log_path = unique_path(
        &case_dir
            .join("evidence/logs")
            .join(format!("e01-info-{imported_unix}.txt")),
    );
    require_e01_output_path(case_dir, &e01_path, &info_log_path, "E01 info log")?;
    let verify_log_path = if options.skip_verify {
        None
    } else {
        let path = unique_path(
            &case_dir
                .join("evidence/logs")
                .join(format!("e01-verify-{imported_unix}.txt")),
        );
        require_e01_output_path(case_dir, &e01_path, &path, "E01 verify log")?;
        Some(path)
    };
    let export_log_path = unique_path(
        &case_dir
            .join("evidence/logs")
            .join(format!("e01-export-{imported_unix}.txt")),
    );
    require_e01_output_path(case_dir, &e01_path, &export_log_path, "E01 export log")?;
    let audit_log_path = e01_audit_log_path(case_dir, &e01_path)?;

    let info = run_capture(
        &options.ewfinfo_bin,
        &["ewfinfo"],
        &["-f", "text", &audit::path_string(&e01_path)],
    )?;
    write_text(&info_log_path, &info.stdout)
        .map_err(|err| format!("failed to write E01 info log: {err}"))?;

    if let Some(path) = &verify_log_path {
        let args = vec![
            "-q".to_string(),
            "-d".to_string(),
            "sha256".to_string(),
            "-l".to_string(),
            audit::path_string(path),
            audit::path_string(&e01_path),
        ];
        run_status(&options.ewfverify_bin, &["ewfverify"], &args)?;
    }

    let requested_raw_path = options.output_path.clone().unwrap_or_else(|| {
        case_dir
            .join("evidence/images")
            .join(default_raw_filename(&e01_path))
    });
    require_case_output_path(case_dir, &requested_raw_path, "E01 raw")?;
    if requested_raw_path.exists() {
        return Err(format!(
            "output already exists: {} (choose a new --output path)",
            requested_raw_path.display()
        ));
    }
    if let Some(parent) = requested_raw_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create E01 output directory: {err}"))?;
    }
    let export_target = ewfexport_target_for_output(&requested_raw_path);
    let generated_raw_path = expected_ewfexport_output(&export_target);
    if generated_raw_path != requested_raw_path && generated_raw_path.exists() {
        return Err(format!(
            "ewfexport target already exists: {} (choose a new --output path)",
            generated_raw_path.display()
        ));
    }

    let export_args = ewfexport_args(
        &e01_path,
        &export_target,
        options.max_bytes,
        &export_log_path,
    );
    run_status(&options.ewfexport_bin, &["ewfexport"], &export_args)?;
    let generated_raw_path = resolve_ewfexport_output(&generated_raw_path)?;
    if generated_raw_path != requested_raw_path {
        std::fs::rename(&generated_raw_path, &requested_raw_path).map_err(|err| {
            format!(
                "failed to rename E01 raw output {} to {}: {err}",
                generated_raw_path.display(),
                requested_raw_path.display()
            )
        })?;
    }
    let raw_output_path = requested_raw_path
        .canonicalize()
        .map_err(|err| format!("failed to canonicalize raw E01 output: {err}"))?;
    let raw_sha256 = audit::digest_file(&raw_output_path)?;
    let e01_sha256 = if options.hash_e01 {
        Some(audit::digest_file(&e01_path)?)
    } else {
        None
    };

    append_e01_audit_at(
        &audit_log_path,
        &format!(
            "{{\"schema_version\":1,\"event\":\"import-e01\",\"imported_unix\":{},\"e01_path\":\"{}\",\"e01_sha256\":{},\"raw_output_path\":\"{}\",\"raw_sha256\":\"{}\",\"max_bytes\":{},\"verified\":{},\"ewfinfo_version\":\"{}\",\"ewfverify_version\":\"{}\",\"ewfexport_version\":\"{}\",\"ewfinfo_log_path\":\"{}\",\"ewfverify_log_path\":{},\"ewfexport_log_path\":\"{}\",\"command\":\"{}\",\"command_args\":{}}}",
            imported_unix,
            json_escape(&e01_path.to_string_lossy()),
            audit::optional_string(e01_sha256.as_deref()),
            json_escape(&raw_output_path.to_string_lossy()),
            json_escape(&raw_sha256),
            options
                .max_bytes
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_string()),
            !options.skip_verify,
            json_escape(&ewf_command_version(&options.ewfinfo_bin, &["ewfinfo"])),
            json_escape(&ewf_command_version(&options.ewfverify_bin, &["ewfverify"])),
            json_escape(&ewf_command_version(&options.ewfexport_bin, &["ewfexport"])),
            json_escape(&info_log_path.to_string_lossy()),
            audit::optional_string(
                verify_log_path
                    .as_ref()
                    .map(|path| path.to_string_lossy())
                    .as_deref()
            ),
            json_escape(&export_log_path.to_string_lossy()),
            json_escape(&options.ewfexport_bin),
            audit::json_string_array(&export_args)
        ),
    )?;

    Ok(E01ImportResult {
        e01_path,
        raw_output_path,
        ewfinfo_log_path: info_log_path,
        ewfverify_log_path: verify_log_path,
        ewfexport_log_path: export_log_path,
        raw_sha256,
        e01_sha256,
    })
}

fn canonical_e01_path(path: &Path) -> Result<PathBuf, String> {
    let path = path
        .canonicalize()
        .map_err(|err| format!("failed to canonicalize E01 path: {err}"))?;
    if !path.is_file() {
        return Err(format!("E01 path is not a file: {}", path.display()));
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    if !extension.eq_ignore_ascii_case("e01")
        && !extension.eq_ignore_ascii_case("ex01")
        && !extension.eq_ignore_ascii_case("s01")
        && !extension.eq_ignore_ascii_case("l01")
    {
        return Err(format!(
            "unsupported EWF extension .{} (expected E01/Ex01/S01/L01 first segment)",
            extension
        ));
    }
    Ok(path)
}
