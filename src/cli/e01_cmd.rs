use crate::cli::handlers::{import_e01, inspect_e01};
use crate::e01::E01Options;
use std::path::{Path, PathBuf};

pub struct ImportE01CliInput {
    pub output: Option<PathBuf>,
    pub max_bytes: Option<u64>,
    pub skip_verify: bool,
    pub hash_e01: bool,
    pub ewfinfo: Option<String>,
    pub ewfverify: Option<String>,
    pub ewfexport: Option<String>,
}

pub fn run_inspect_e01(
    case_dir: &Path,
    e01_file: &Path,
    hash_e01: bool,
    ewfinfo: Option<String>,
) -> Result<(), String> {
    inspect_e01(
        case_dir,
        e01_file,
        E01Options {
            output_path: None,
            max_bytes: None,
            skip_verify: false,
            hash_e01,
            ewfinfo_bin: ewfinfo.unwrap_or_else(|| "ewfinfo".to_string()),
            ewfverify_bin: "ewfverify".to_string(),
            ewfexport_bin: "ewfexport".to_string(),
        },
    )
}

pub fn run_import_e01(
    case_dir: &Path,
    e01_file: &Path,
    input: ImportE01CliInput,
) -> Result<(), String> {
    import_e01(
        case_dir,
        e01_file,
        E01Options {
            output_path: input.output,
            max_bytes: input.max_bytes,
            skip_verify: input.skip_verify,
            hash_e01: input.hash_e01,
            ewfinfo_bin: input.ewfinfo.unwrap_or_else(|| "ewfinfo".to_string()),
            ewfverify_bin: input.ewfverify.unwrap_or_else(|| "ewfverify".to_string()),
            ewfexport_bin: input.ewfexport.unwrap_or_else(|| "ewfexport".to_string()),
        },
    )
}
