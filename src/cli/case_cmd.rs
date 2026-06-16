use crate::cli::handlers::{
    InitCaseOptions, RegisterSourceOptions, init_case, register_source, scan_folder,
};
use crate::model::ScanOptions;
use std::path::Path;

pub struct InitCaseCliInput {
    pub title: Option<String>,
    pub operator: Option<String>,
    pub device_id: Option<String>,
    pub device_serial: Option<String>,
    pub write_protect: Option<String>,
    pub acquisition_tool: Option<String>,
    pub evidence_hash: Option<String>,
    pub notes: Option<String>,
}

pub struct RegisterSourceCliInput {
    pub kind: String,
    pub source_id: Option<String>,
    pub write_protect: Option<String>,
    pub acquisition_tool: Option<String>,
    pub evidence_hash: Option<String>,
    pub notes: Option<String>,
}

pub fn run_init_case(case_dir: &Path, input: InitCaseCliInput) -> Result<(), String> {
    init_case(
        case_dir,
        &InitCaseOptions {
            title: input.title,
            operator: input.operator,
            device_id: input.device_id,
            device_serial: input.device_serial,
            write_protect: input.write_protect,
            acquisition_tool: input.acquisition_tool,
            evidence_hash: input.evidence_hash,
            notes: input.notes,
        },
    )
}

pub fn run_scan_folder(
    case_dir: &Path,
    source_dir: &Path,
    hash: bool,
    no_ffprobe: bool,
    max_depth: Option<usize>,
) -> Result<(), String> {
    scan_folder(
        case_dir,
        source_dir,
        ScanOptions {
            hash_files: hash,
            use_ffprobe: !no_ffprobe,
            max_depth,
        },
    )
}

pub fn run_register_source(
    case_dir: &Path,
    source_path: &Path,
    input: RegisterSourceCliInput,
) -> Result<(), String> {
    register_source(
        case_dir,
        source_path,
        RegisterSourceOptions {
            kind: input.kind,
            source_id: input.source_id,
            write_protect: input.write_protect,
            acquisition_tool: input.acquisition_tool,
            evidence_hash: input.evidence_hash,
            notes: input.notes,
        },
    )
}
