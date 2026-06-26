use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "FrameTrace", version, about = "Windows local workstation concept for reviewing video evidence", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create a local forensic case folder.
    InitCase {
        case_dir: PathBuf,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        operator: Option<String>,
        #[arg(long)]
        device_id: Option<String>,
        #[arg(long)]
        device_serial: Option<String>,
        #[arg(long)]
        write_protect: Option<String>,
        #[arg(long)]
        acquisition_tool: Option<String>,
        #[arg(long)]
        evidence_hash: Option<String>,
        #[arg(long)]
        notes: Option<String>,
    },
    /// Index video candidates and likely manufacturer/parser lanes
    ScanFolder {
        case_dir: PathBuf,
        source_dir: PathBuf,
        #[arg(long)]
        hash: bool,
        #[arg(long)]
        no_ffprobe: bool,
        #[arg(long)]
        max_depth: Option<usize>,
    },
    /// Register an evidence source in the SQLite case database
    RegisterSource {
        case_dir: PathBuf,
        path: PathBuf,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        source_id: Option<String>,
        #[arg(long)]
        write_protect: Option<String>,
        #[arg(long)]
        acquisition_tool: Option<String>,
        #[arg(long)]
        evidence_hash: Option<String>,
        #[arg(long)]
        notes: Option<String>,
    },
    /// Record E01 metadata through libewf ewfinfo
    InspectE01 {
        case_dir: PathBuf,
        e01_file: PathBuf,
        #[arg(long)]
        hash_e01: bool,
        #[arg(long)]
        ewfinfo: Option<String>,
    },
    /// Verify an E01 with libewf, export it to raw image form, hash the raw output
    ImportE01 {
        case_dir: PathBuf,
        e01_file: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        max_bytes: Option<u64>,
        #[arg(long)]
        skip_verify: bool,
        #[arg(long)]
        hash_e01: bool,
        #[arg(long)]
        ewfinfo: Option<String>,
        #[arg(long)]
        ewfverify: Option<String>,
        #[arg(long)]
        ewfexport: Option<String>,
    },
    /// Generate a serverless HTML review dashboard at review/index.html
    MakeReview {
        case_dir: PathBuf,
        #[arg(long)]
        include_full_paths: bool,
    },
    /// Print the manufacturer/source parser plugin catalog as JSON
    ListParsers,
    /// Generate a case report at reports/case-report.html
    MakeReport {
        case_dir: PathBuf,
        #[arg(long)]
        include_full_paths: bool,
    },
    /// Build a checksummed report/review package directory with manifest files
    PackageCase {
        case_dir: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        include_full_paths: bool,
    },
    /// Export an indexed video or selected range as a client-deliverable MP4/AVI
    ExportVideo {
        case_dir: PathBuf,
        selector: String,
        #[arg(long)]
        format: String,
        #[arg(long)]
        start: Option<f64>,
        #[arg(long)]
        duration: Option<f64>,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        operator: Option<String>,
        #[arg(long)]
        ffmpeg: Option<String>,
    },
    /// Generate a lower-bitrate review proxy MP4
    MakeProxy {
        case_dir: PathBuf,
        selector: String,
        #[arg(long)]
        max_width: Option<u32>,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        operator: Option<String>,
        #[arg(long)]
        ffmpeg: Option<String>,
    },
    /// Generate a JPEG thumbnail for review/reporting
    MakeThumbnail {
        case_dir: PathBuf,
        selector: String,
        #[arg(long)]
        time: Option<f64>,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        operator: Option<String>,
        #[arg(long)]
        ffmpeg: Option<String>,
    },
    /// Capture a report still frame as a derived photo artifact
    CaptureFrame {
        case_dir: PathBuf,
        selector: String,
        #[arg(long)]
        time: Option<f64>,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        operator: Option<String>,
        #[arg(long)]
        ffmpeg: Option<String>,
    },
    /// Scan a raw file or forensic image for contiguous candidates and carve them
    CarveFile {
        case_dir: PathBuf,
        source_file: PathBuf,
        #[arg(long)]
        max_bytes: Option<u64>,
        #[arg(long)]
        max_candidates: Option<usize>,
    },
    /// List active/deleted files in a raw forensic image with Sleuth Kit mmls/fls
    InspectImage {
        case_dir: PathBuf,
        image_file: PathBuf,
        #[arg(long)]
        partition_offset: Option<u64>,
        #[arg(long, default_value_t = 20000)]
        max_entries: usize,
        #[arg(long)]
        mmls: Option<String>,
        #[arg(long)]
        fls: Option<String>,
    },
    /// Recover one filesystem inode from a raw forensic image with Sleuth Kit icat
    RecoverInode {
        case_dir: PathBuf,
        image_file: PathBuf,
        inode: String,
        #[arg(long, default_value_t = 0)]
        partition_offset: u64,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        recover_deleted: bool,
        #[arg(long)]
        include_slack: bool,
        #[arg(long)]
        skip_sparse_holes: bool,
        #[arg(long)]
        icat: Option<String>,
    },
    /// Validate an indexed video, carved candidate, recovered inode, or artifact path with ffprobe
    ValidateArtifact {
        case_dir: PathBuf,
        selector: String,
        #[arg(long)]
        ffprobe: Option<String>,
        #[arg(long)]
        operator: Option<String>,
        #[arg(long)]
        external_source: bool,
    },
    /// Record examiner playback confirmation after ffprobe stream validation
    ConfirmPlayback {
        case_dir: PathBuf,
        selector: String,
        #[arg(long)]
        playback_tool: Option<String>,
        #[arg(long)]
        notes: Option<String>,
        #[arg(long)]
        operator: Option<String>,
    },
    /// Verify a chained JSONL audit log and report tamper status
    VerifyAudit { log_path: PathBuf },
    /// Mark leftover running jobs as interrupted before release or retry review
    MarkInterruptedJobs {
        case_dir: PathBuf,
        #[arg(
            long,
            default_value = "operator marked stale running jobs as interrupted"
        )]
        reason: String,
    },
    /// Create a synthetic SQLite index benchmark database for scale validation
    BenchmarkDb {
        output_dir: PathBuf,
        #[arg(long, default_value_t = 10000)]
        rows: usize,
    },
    /// Print the current case/index status
    Inspect { case_dir: PathBuf },
    /// Print bounded engine-owned status for the Windows workstation shell
    WorkstationStatus { case_dir: PathBuf },
    /// Query the SQLite-backed forensic inventory as bounded JSON
    Inventory {
        case_dir: PathBuf,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        file_id: Option<String>,
        #[arg(long)]
        facets: bool,
        #[arg(long, default_value_t = 0)]
        offset: usize,
        #[arg(long, default_value_t = 100)]
        limit: usize,
        #[arg(long)]
        extension: Option<String>,
        #[arg(long)]
        validation_state: Option<String>,
        #[arg(long)]
        sort: Option<String>,
    },
    /// Build an auditable bulk-action preview without mutating evidence state
    InventoryBulkPreview {
        case_dir: PathBuf,
        #[arg(long)]
        action: String,
        #[arg(long)]
        operator: String,
        #[arg(long)]
        filters_json: Option<String>,
        file_ids: Vec<String>,
    },
    /// Export selected inventory rows as a manifest artifact
    InventoryExportManifest {
        case_dir: PathBuf,
        #[arg(long)]
        operator: String,
        #[arg(long)]
        filters_json: Option<String>,
        #[arg(long)]
        output: Option<PathBuf>,
        file_ids: Vec<String>,
    },
    /// Run forensic QA validation checks
    Qa {
        #[command(subcommand)]
        command: QaCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum QaCommands {
    /// Compare an indexed case against a typed or legacy TSV ground-truth corpus manifest
    Accuracy {
        case_dir: PathBuf,
        corpus_manifest: PathBuf,
        #[arg(long)]
        output_dir: Option<PathBuf>,
    },
    /// Compare two case directories for normalized deterministic equivalence
    Reproducibility {
        left_case_dir: PathBuf,
        right_case_dir: PathBuf,
        #[arg(long)]
        output_dir: Option<PathBuf>,
    },
    /// Check whether required report-defensible artifacts exist
    ReportDefense {
        case_dir: PathBuf,
        #[arg(long)]
        output_dir: Option<PathBuf>,
    },
    /// Check distributable report/review/package outputs for privacy leakage and banned wording
    PrivacyReview {
        case_dir: PathBuf,
        #[arg(long)]
        output_dir: Option<PathBuf>,
    },
    /// Run a SQLite-backed scale benchmark and emit a performance report
    Performance {
        output_dir: PathBuf,
        #[arg(long, default_value_t = 10000)]
        rows: usize,
    },
    /// Run release-readiness QA and emit pass/fail blockers
    Release {
        case_dir: PathBuf,
        #[arg(long)]
        corpus_manifest: Option<PathBuf>,
        #[arg(long)]
        comparison_case: Option<PathBuf>,
        #[arg(long)]
        review_manifest: Option<PathBuf>,
        #[arg(long)]
        output_dir: Option<PathBuf>,
        #[arg(long)]
        performance_output_dir: Option<PathBuf>,
        #[arg(long, default_value_t = 100000)]
        performance_rows: usize,
    },
}
