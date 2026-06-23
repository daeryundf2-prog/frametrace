use crate::audit;
use crate::util::json_escape;
use std::path::PathBuf;

const DEFAULT_MAX_ENTRIES: usize = 20_000;

#[derive(Debug, Clone)]
pub struct TskInspectOptions {
    pub partition_offset: Option<u64>,
    pub max_entries: usize,
    pub mmls_bin: String,
    pub fls_bin: String,
}

impl Default for TskInspectOptions {
    fn default() -> Self {
        Self {
            partition_offset: None,
            max_entries: DEFAULT_MAX_ENTRIES,
            mmls_bin: "mmls".to_string(),
            fls_bin: "fls".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TskRecoverOptions {
    pub partition_offset: u64,
    pub inode: String,
    pub output_path: Option<PathBuf>,
    pub recover_deleted: bool,
    pub include_slack: bool,
    pub skip_sparse_holes: bool,
    pub icat_bin: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MmlsPartition {
    pub slot: String,
    pub start: u64,
    pub end: u64,
    pub length: u64,
    pub description: String,
    pub allocated: bool,
}

impl MmlsPartition {
    pub(super) fn to_json(&self) -> String {
        format!(
            "{{\"slot\":\"{}\",\"start\":{},\"end\":{},\"length\":{},\"description\":\"{}\",\"allocated\":{}}}",
            json_escape(&self.slot),
            self.start,
            self.end,
            self.length,
            json_escape(&self.description),
            self.allocated
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlsEntry {
    pub raw_line: String,
    pub file_type: Option<String>,
    pub inode: Option<String>,
    pub path: Option<String>,
    pub deleted: bool,
    pub video_candidate: bool,
}

impl FlsEntry {
    pub(super) fn to_json(&self) -> String {
        format!(
            "{{\"raw_line\":\"{}\",\"file_type\":{},\"inode\":{},\"path\":{},\"deleted\":{},\"video_candidate\":{}}}",
            json_escape(&self.raw_line),
            audit::optional_string(self.file_type.as_deref()),
            audit::optional_string(self.inode.as_deref()),
            audit::optional_string(self.path.as_deref()),
            self.deleted,
            self.video_candidate
        )
    }
}

#[derive(Debug, Clone)]
pub struct TskInspectResult {
    pub image_path: PathBuf,
    pub inspected_unix: u64,
    pub partition_offset: u64,
    pub partitions: Vec<MmlsPartition>,
    pub entries: Vec<FlsEntry>,
    pub warnings: Vec<String>,
    pub mmls_log_path: PathBuf,
    pub fls_log_path: PathBuf,
    pub entries_jsonl_path: PathBuf,
    pub summary_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct TskRecoverResult {
    pub image_path: PathBuf,
    pub output_path: PathBuf,
    pub recovered_unix: u64,
    pub partition_offset: u64,
    pub inode: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub validation_status: String,
}
