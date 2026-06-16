use crate::cli::handlers::{inspect_image, recover_inode};
use crate::tsk::{TskInspectOptions, TskRecoverOptions};
use std::path::{Path, PathBuf};

pub struct RecoverInodeCliInput {
    pub inode: String,
    pub partition_offset: u64,
    pub output: Option<PathBuf>,
    pub recover_deleted: bool,
    pub include_slack: bool,
    pub skip_sparse_holes: bool,
    pub icat: Option<String>,
}

pub fn run_inspect_image(
    case_dir: &Path,
    image_file: &Path,
    partition_offset: Option<u64>,
    max_entries: usize,
    mmls: Option<String>,
    fls: Option<String>,
) -> Result<(), String> {
    inspect_image(
        case_dir,
        image_file,
        TskInspectOptions {
            partition_offset,
            max_entries,
            mmls_bin: mmls.unwrap_or_else(|| "mmls".to_string()),
            fls_bin: fls.unwrap_or_else(|| "fls".to_string()),
        },
    )
}

pub fn run_recover_inode(
    case_dir: &Path,
    image_file: &Path,
    input: RecoverInodeCliInput,
) -> Result<(), String> {
    recover_inode(
        case_dir,
        image_file,
        TskRecoverOptions {
            partition_offset: input.partition_offset,
            inode: input.inode,
            output_path: input.output,
            recover_deleted: input.recover_deleted,
            include_slack: input.include_slack,
            skip_sparse_holes: input.skip_sparse_holes,
            icat_bin: input.icat.unwrap_or_else(|| "icat".to_string()),
        },
    )
}
