mod audit_log;
mod commands;
mod parse;
mod types;

use crate::audit;
use crate::tool_policy::require_case_output_path;
use crate::util::{json_escape, now_unix, unique_path, write_text};
use audit_log::{
    InspectSummaryInput, append_tsk_audit_at, inspect_summary_json, tsk_audit_log_path,
};
use commands::{
    fls_args, icat_args, run_capture, run_icat_to_file, sanitize_filename, tsk_command_version,
    tsk_path_string,
};
use parse::{choose_partition_offset, parse_fls_entries, parse_mmls_partitions};
use std::fs;
use std::path::{Path, PathBuf};
pub use types::{
    FlsEntry, MmlsPartition, TskInspectOptions, TskInspectResult, TskRecoverOptions,
    TskRecoverResult,
};

pub fn inspect_image(
    case_dir: &Path,
    image_path: &Path,
    options: &TskInspectOptions,
) -> Result<TskInspectResult, String> {
    if options.max_entries == 0 {
        return Err("--max-entries must be greater than 0".to_string());
    }
    let image_path = canonical_image_path(image_path)?;
    let inspected_unix = now_unix()?;
    let mut warnings = Vec::new();

    let mmls_log_path = unique_path(
        &case_dir
            .join("evidence/logs")
            .join(format!("tsk-mmls-{inspected_unix}.txt")),
    );
    require_case_output_path(case_dir, &mmls_log_path, "mmls log")?;
    let mmls_args = vec![tsk_path_string(&image_path)];
    let mmls = run_capture(&options.mmls_bin, &["mmls"], &mmls_args);
    let partitions = match &mmls {
        Ok(output) if output.status_success => {
            write_text(&mmls_log_path, &output.combined_text())
                .map_err(|err| format!("failed to write mmls log: {err}"))?;
            parse_mmls_partitions(&output.stdout)
        }
        Ok(output) => {
            write_text(&mmls_log_path, &output.combined_text())
                .map_err(|err| format!("failed to write mmls log: {err}"))?;
            warnings.push(format!(
                "mmls did not complete successfully; using explicit/default offset: {}",
                output.stderr.trim()
            ));
            Vec::new()
        }
        Err(err) => {
            write_text(&mmls_log_path, &format!("mmls unavailable: {err}\n"))
                .map_err(|write_err| format!("failed to write mmls log: {write_err}"))?;
            warnings.push(format!(
                "mmls unavailable; using explicit/default offset: {err}"
            ));
            Vec::new()
        }
    };

    let partition_offset = choose_partition_offset(&partitions, options.partition_offset);
    let fls_log_path = unique_path(
        &case_dir
            .join("evidence/logs")
            .join(format!("tsk-fls-{inspected_unix}.txt")),
    );
    require_case_output_path(case_dir, &fls_log_path, "fls log")?;
    let fls_args = fls_args(&image_path, partition_offset);
    let fls = run_capture(&options.fls_bin, &["fls"], &fls_args)?;
    write_text(&fls_log_path, &fls.combined_text())
        .map_err(|err| format!("failed to write fls log: {err}"))?;
    if !fls.status_success {
        return Err(format!(
            "fls failed at offset {}: {}",
            partition_offset,
            fls.stderr.trim()
        ));
    }

    let mut entries = parse_fls_entries(&fls.stdout);
    if entries.len() > options.max_entries {
        warnings.push(format!(
            "filesystem listing truncated from {} to {} entries",
            entries.len(),
            options.max_entries
        ));
        entries.truncate(options.max_entries);
    }

    let entries_jsonl_path = unique_path(
        &case_dir
            .join("db/filesystem")
            .join(format!("tsk-files-{inspected_unix}.jsonl")),
    );
    require_case_output_path(case_dir, &entries_jsonl_path, "filesystem entries")?;
    let jsonl = entries
        .iter()
        .map(FlsEntry::to_json)
        .collect::<Vec<_>>()
        .join("\n");
    write_text(&entries_jsonl_path, &(jsonl + "\n"))
        .map_err(|err| format!("failed to write filesystem entries: {err}"))?;

    let summary_path = unique_path(
        &case_dir
            .join("db/filesystem")
            .join(format!("tsk-inspection-{inspected_unix}.json")),
    );
    require_case_output_path(case_dir, &summary_path, "filesystem inspection summary")?;
    let audit_log_path = tsk_audit_log_path(case_dir, Some(&image_path))?;
    let summary = inspect_summary_json(&InspectSummaryInput {
        image_path: &image_path,
        inspected_unix,
        partition_offset,
        partitions: &partitions,
        entries: &entries,
        warnings: &warnings,
        mmls_log_path: &mmls_log_path,
        fls_log_path: &fls_log_path,
        entries_jsonl_path: &entries_jsonl_path,
    });
    write_text(&summary_path, &summary)
        .map_err(|err| format!("failed to write filesystem inspection summary: {err}"))?;

    append_tsk_audit_at(
        &audit_log_path,
        &format!(
            "{{\"schema_version\":1,\"event\":\"inspect-image-filesystem\",\"inspected_unix\":{},\"image_path\":\"{}\",\"partition_offset\":{},\"partition_count\":{},\"entry_count\":{},\"deleted_count\":{},\"video_candidate_count\":{},\"mmls_version\":\"{}\",\"fls_version\":\"{}\",\"mmls_log_path\":\"{}\",\"fls_log_path\":\"{}\",\"entries_jsonl_path\":\"{}\",\"summary_path\":\"{}\",\"warnings\":{}}}",
            inspected_unix,
            json_escape(&image_path.to_string_lossy()),
            partition_offset,
            partitions.len(),
            entries.len(),
            entries.iter().filter(|entry| entry.deleted).count(),
            entries.iter().filter(|entry| entry.video_candidate).count(),
            json_escape(&tsk_command_version(&options.mmls_bin, &["mmls"])),
            json_escape(&tsk_command_version(&options.fls_bin, &["fls"])),
            json_escape(&mmls_log_path.to_string_lossy()),
            json_escape(&fls_log_path.to_string_lossy()),
            json_escape(&entries_jsonl_path.to_string_lossy()),
            json_escape(&summary_path.to_string_lossy()),
            audit::json_string_array(&warnings)
        ),
    )?;

    Ok(TskInspectResult {
        image_path,
        inspected_unix,
        partition_offset,
        partitions,
        entries,
        warnings,
        mmls_log_path,
        fls_log_path,
        entries_jsonl_path,
        summary_path,
    })
}

pub fn recover_inode(
    case_dir: &Path,
    image_path: &Path,
    options: &TskRecoverOptions,
) -> Result<TskRecoverResult, String> {
    let image_path = canonical_image_path(image_path)?;
    if options.inode.trim().is_empty() {
        return Err("inode must not be empty".to_string());
    }
    let recovered_unix = now_unix()?;
    let output_path = match &options.output_path {
        Some(path) => {
            if path.exists() {
                return Err(format!(
                    "output already exists: {} (choose a new --output path)",
                    path.display()
                ));
            }
            path.to_path_buf()
        }
        None => unique_path(
            &case_dir
                .join("artifacts/recovered/filesystem")
                .join(format!("inode_{}.bin", sanitize_filename(&options.inode))),
        ),
    };
    require_case_output_path(case_dir, &output_path, "inode recovery")?;
    let audit_log_path = tsk_audit_log_path(case_dir, Some(&image_path))?;
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create recovery output directory: {err}"))?;
    }

    let args = icat_args(&image_path, options);
    run_icat_to_file(
        &options.icat_bin,
        &args,
        &output_path,
        &options.inode,
        options.partition_offset,
    )?;

    let output_path = output_path
        .canonicalize()
        .map_err(|err| format!("failed to canonicalize recovered output: {err}"))?;
    let size_bytes = fs::metadata(&output_path)
        .map_err(|err| format!("failed to read recovered output metadata: {err}"))?
        .len();
    let sha256 = audit::digest_file(&output_path)?;
    let validation_status = "candidate-unvalidated".to_string();

    append_tsk_audit_at(
        &audit_log_path,
        &format!(
            "{{\"schema_version\":1,\"event\":\"recover-inode\",\"recovered_unix\":{},\"image_path\":\"{}\",\"partition_offset\":{},\"inode\":\"{}\",\"output_path\":\"{}\",\"size_bytes\":{},\"sha256\":\"{}\",\"validation_status\":\"{}\",\"recover_deleted\":{},\"include_slack\":{},\"skip_sparse_holes\":{},\"icat_version\":\"{}\",\"command\":\"{}\",\"command_args\":{}}}",
            recovered_unix,
            json_escape(&image_path.to_string_lossy()),
            options.partition_offset,
            json_escape(&options.inode),
            json_escape(&output_path.to_string_lossy()),
            size_bytes,
            json_escape(&sha256),
            json_escape(&validation_status),
            options.recover_deleted,
            options.include_slack,
            options.skip_sparse_holes,
            json_escape(&tsk_command_version(&options.icat_bin, &["icat"])),
            json_escape(&options.icat_bin),
            audit::json_string_array(&args)
        ),
    )?;

    Ok(TskRecoverResult {
        image_path,
        output_path,
        recovered_unix,
        partition_offset: options.partition_offset,
        inode: options.inode.clone(),
        size_bytes,
        sha256,
        validation_status,
    })
}

fn canonical_image_path(path: &Path) -> Result<PathBuf, String> {
    if !path.is_file() {
        return Err(format!("forensic image is not a file: {}", path.display()));
    }
    Ok(path.to_path_buf())
}
