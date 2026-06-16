use clap::Parser;
use clap::error::ErrorKind;

mod case_cmd;
mod commands;
mod e01_cmd;
pub mod handlers;
mod inventory_cmd;
mod media_cmd;
mod qa_cmd;
mod tsk_cmd;
use case_cmd::{
    InitCaseCliInput, RegisterSourceCliInput, run_init_case, run_register_source, run_scan_folder,
};
use commands::{Cli, Commands};
use e01_cmd::{ImportE01CliInput, run_import_e01, run_inspect_e01};
use handlers::*;
use inventory_cmd::run_inventory_command;
use media_cmd::{
    CarveCliInput, ExportVideoCliInput, run_carve_file, run_export_video, run_make_proxy,
    run_make_thumbnail, run_validate_artifact,
};
use qa_cmd::run_qa;
use tsk_cmd::{RecoverInodeCliInput, run_inspect_image, run_recover_inode};

pub fn run(args: Vec<String>) -> Result<(), String> {
    let cli = match Cli::try_parse_from(args) {
        Ok(c) => c,
        Err(e) => {
            let kind = e.kind();
            e.print()
                .map_err(|err| format!("failed to print command line help: {err}"))?;
            return if matches!(kind, ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
                Ok(())
            } else {
                Err("invalid command line".to_string())
            };
        }
    };

    match cli.command {
        Commands::InitCase {
            case_dir,
            title,
            operator,
            device_id,
            device_serial,
            write_protect,
            acquisition_tool,
            evidence_hash,
            notes,
        } => run_init_case(
            &case_dir,
            InitCaseCliInput {
                title,
                operator,
                device_id,
                device_serial,
                write_protect,
                acquisition_tool,
                evidence_hash,
                notes,
            },
        ),
        Commands::ScanFolder {
            case_dir,
            source_dir,
            hash,
            no_ffprobe,
            max_depth,
        } => run_scan_folder(&case_dir, &source_dir, hash, no_ffprobe, max_depth),
        Commands::RegisterSource {
            case_dir,
            path,
            kind,
            source_id,
            write_protect,
            acquisition_tool,
            evidence_hash,
            notes,
        } => run_register_source(
            &case_dir,
            &path,
            RegisterSourceCliInput {
                kind,
                source_id,
                write_protect,
                acquisition_tool,
                evidence_hash,
                notes,
            },
        ),
        Commands::InspectE01 {
            case_dir,
            e01_file,
            hash_e01,
            ewfinfo,
        } => run_inspect_e01(&case_dir, &e01_file, hash_e01, ewfinfo),
        Commands::ImportE01 {
            case_dir,
            e01_file,
            output,
            max_bytes,
            skip_verify,
            hash_e01,
            ewfinfo,
            ewfverify,
            ewfexport,
        } => run_import_e01(
            &case_dir,
            &e01_file,
            ImportE01CliInput {
                output,
                max_bytes,
                skip_verify,
                hash_e01,
                ewfinfo,
                ewfverify,
                ewfexport,
            },
        ),
        Commands::MakeReview { case_dir } => make_review(&case_dir),
        Commands::ListParsers => {
            println!("{}", crate::detector::parser_catalog_json());
            Ok(())
        }
        Commands::MakeReport { case_dir } => make_report(&case_dir),
        Commands::PackageCase { case_dir, output } => {
            let options = PackageOptions { output_dir: output };
            package_case(&case_dir, options)
        }
        Commands::ExportVideo {
            case_dir,
            selector,
            format,
            start,
            duration,
            output,
        } => run_export_video(
            &case_dir,
            &selector,
            ExportVideoCliInput {
                format,
                start,
                duration,
                output,
            },
        ),
        Commands::MakeProxy {
            case_dir,
            selector,
            max_width,
            output,
        } => run_make_proxy(&case_dir, &selector, max_width, output),
        Commands::MakeThumbnail {
            case_dir,
            selector,
            time,
            output,
        } => run_make_thumbnail(&case_dir, &selector, time, output),
        Commands::CarveFile {
            case_dir,
            source_file,
            max_bytes,
            max_candidates,
        } => run_carve_file(
            &case_dir,
            &source_file,
            CarveCliInput {
                max_bytes,
                max_candidates,
            },
        ),
        Commands::InspectImage {
            case_dir,
            image_file,
            partition_offset,
            max_entries,
            mmls,
            fls,
        } => run_inspect_image(
            &case_dir,
            &image_file,
            partition_offset,
            max_entries,
            mmls,
            fls,
        ),
        Commands::RecoverInode {
            case_dir,
            image_file,
            inode,
            partition_offset,
            output,
            recover_deleted,
            include_slack,
            skip_sparse_holes,
            icat,
        } => run_recover_inode(
            &case_dir,
            &image_file,
            RecoverInodeCliInput {
                partition_offset,
                inode,
                output,
                recover_deleted,
                include_slack,
                skip_sparse_holes,
                icat,
            },
        ),
        Commands::ValidateArtifact {
            case_dir,
            selector,
            ffprobe,
        } => run_validate_artifact(&case_dir, &selector, ffprobe),
        Commands::VerifyAudit { log_path } => verify_audit(&log_path),
        Commands::MarkInterruptedJobs { case_dir, reason } => {
            mark_interrupted_jobs(&case_dir, &reason)
        }
        Commands::BenchmarkDb { output_dir, rows } => {
            let options = BenchmarkOptions { rows };
            benchmark_db(&output_dir, options)
        }
        Commands::Inspect { case_dir } => inspect(&case_dir),
        command @ (Commands::Inventory { .. } | Commands::InventoryBulkPreview { .. }) => {
            run_inventory_command(command)
        }
        Commands::Qa { command } => run_qa(command),
    }
}
