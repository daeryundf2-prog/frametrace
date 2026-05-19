use crate::artifacts::{ProxyOptions, ThumbnailOptions};
use crate::carve::CarveOptions;
use crate::e01::E01Options;
use crate::model::ScanOptions;
use crate::video_export::{ExportFormat, ExportOptions};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub mod handlers;
use handlers::*;

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
    MakeReview { case_dir: PathBuf },
    /// Print the manufacturer/source parser plugin catalog as JSON
    ListParsers,
    /// Generate a case report at reports/case-report.html
    MakeReport { case_dir: PathBuf },
    /// Build a checksummed report/review package directory with manifest files
    PackageCase {
        case_dir: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
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
    },
    /// Generate a lower-bitrate review proxy MP4
    MakeProxy {
        case_dir: PathBuf,
        selector: String,
        #[arg(long)]
        max_width: Option<u32>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Generate a JPEG thumbnail for review/reporting
    MakeThumbnail {
        case_dir: PathBuf,
        selector: String,
        #[arg(long)]
        time: Option<f64>,
        #[arg(long)]
        output: Option<PathBuf>,
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
    /// Create a synthetic SQLite index benchmark database for scale validation
    BenchmarkDb {
        output_dir: PathBuf,
        #[arg(long, default_value_t = 10000)]
        rows: usize,
    },
    /// Print the current case/index status
    Inspect { case_dir: PathBuf },
}

pub fn run(args: Vec<String>) -> Result<(), String> {
    let cli = match Cli::try_parse_from(args) {
        Ok(c) => c,
        Err(e) => {
            e.print().unwrap();
            return Ok(());
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
        } => {
            let options = InitCaseOptions {
                title,
                operator,
                device_id,
                device_serial,
                write_protect,
                acquisition_tool,
                evidence_hash,
                notes,
            };
            init_case(&case_dir, &options)
        }
        Commands::ScanFolder {
            case_dir,
            source_dir,
            hash,
            no_ffprobe,
            max_depth,
        } => {
            let options = ScanOptions {
                hash_files: hash,
                use_ffprobe: !no_ffprobe,
                max_depth,
            };
            scan_folder(&case_dir, &source_dir, options)
        }
        Commands::RegisterSource {
            case_dir,
            path,
            kind,
            source_id,
            write_protect,
            acquisition_tool,
            evidence_hash,
            notes,
        } => {
            let options = RegisterSourceOptions {
                kind,
                source_id,
                write_protect,
                acquisition_tool,
                evidence_hash,
                notes,
            };
            register_source(&case_dir, &path, options)
        }
        Commands::InspectE01 {
            case_dir,
            e01_file,
            hash_e01,
            ewfinfo,
        } => {
            let options = E01Options {
                output_path: None,
                max_bytes: None,
                skip_verify: false,
                hash_e01,
                ewfinfo_bin: ewfinfo.unwrap_or_else(|| "ewfinfo".to_string()),
                ewfverify_bin: "ewfverify".to_string(),
                ewfexport_bin: "ewfexport".to_string(),
            };
            inspect_e01(&case_dir, &e01_file, options)
        }
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
        } => {
            let options = E01Options {
                output_path: output,
                max_bytes,
                skip_verify,
                hash_e01,
                ewfinfo_bin: ewfinfo.unwrap_or_else(|| "ewfinfo".to_string()),
                ewfverify_bin: ewfverify.unwrap_or_else(|| "ewfverify".to_string()),
                ewfexport_bin: ewfexport.unwrap_or_else(|| "ewfexport".to_string()),
            };
            import_e01(&case_dir, &e01_file, options)
        }
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
        } => {
            let fmt = ExportFormat::parse(&format)?;
            let options = ExportOptions {
                format: fmt,
                start_seconds: start,
                duration_seconds: duration,
                output_path: output,
            };
            export_video(&case_dir, &selector, options)
        }
        Commands::MakeProxy {
            case_dir,
            selector,
            max_width,
            output,
        } => {
            let options = ProxyOptions {
                max_width: max_width.unwrap_or_else(|| ProxyOptions::default().max_width),
                output_path: output,
            };
            make_proxy(&case_dir, &selector, options)
        }
        Commands::MakeThumbnail {
            case_dir,
            selector,
            time,
            output,
        } => {
            let options = ThumbnailOptions {
                time_seconds: time.unwrap_or(0.0),
                output_path: output,
            };
            make_thumbnail(&case_dir, &selector, options)
        }
        Commands::CarveFile {
            case_dir,
            source_file,
            max_bytes,
            max_candidates,
        } => {
            let mut options = CarveOptions::default();
            if let Some(mb) = max_bytes {
                options.max_bytes = mb;
            }
            if let Some(mc) = max_candidates {
                options.max_candidates = mc;
            }
            carve_file(&case_dir, &source_file, options)
        }
        Commands::BenchmarkDb { output_dir, rows } => {
            let options = BenchmarkOptions { rows };
            benchmark_db(&output_dir, options)
        }
        Commands::Inspect { case_dir } => inspect(&case_dir),
    }
}
