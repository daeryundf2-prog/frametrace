use crate::artifacts::{ProxyOptions, ThumbnailOptions};
use crate::carve::CarveOptions;
use crate::cli::handlers::{
    carve_file, export_video, make_proxy, make_thumbnail, validate_artifact,
};
use crate::validation::ValidationOptions;
use crate::video_export::{ExportFormat, ExportOptions};
use std::path::{Path, PathBuf};

pub struct ExportVideoCliInput {
    pub format: String,
    pub start: Option<f64>,
    pub duration: Option<f64>,
    pub output: Option<PathBuf>,
}

pub struct CarveCliInput {
    pub max_bytes: Option<u64>,
    pub max_candidates: Option<usize>,
}

pub fn run_export_video(
    case_dir: &Path,
    selector: &str,
    input: ExportVideoCliInput,
) -> Result<(), String> {
    export_video(
        case_dir,
        selector,
        ExportOptions {
            format: ExportFormat::parse(&input.format)?,
            start_seconds: input.start,
            duration_seconds: input.duration,
            output_path: input.output,
        },
    )
}

pub fn run_make_proxy(
    case_dir: &Path,
    selector: &str,
    max_width: Option<u32>,
    output: Option<PathBuf>,
) -> Result<(), String> {
    make_proxy(
        case_dir,
        selector,
        ProxyOptions {
            max_width: max_width.unwrap_or_else(|| ProxyOptions::default().max_width),
            output_path: output,
        },
    )
}

pub fn run_make_thumbnail(
    case_dir: &Path,
    selector: &str,
    time: Option<f64>,
    output: Option<PathBuf>,
) -> Result<(), String> {
    make_thumbnail(
        case_dir,
        selector,
        ThumbnailOptions {
            time_seconds: time.unwrap_or(0.0),
            output_path: output,
        },
    )
}

pub fn run_carve_file(
    case_dir: &Path,
    source_file: &Path,
    input: CarveCliInput,
) -> Result<(), String> {
    let mut options = CarveOptions::default();
    if let Some(max_bytes) = input.max_bytes {
        options.max_bytes = max_bytes;
    }
    if let Some(max_candidates) = input.max_candidates {
        options.max_candidates = max_candidates;
    }
    carve_file(case_dir, source_file, options)
}

pub fn run_validate_artifact(
    case_dir: &Path,
    selector: &str,
    ffprobe: Option<String>,
) -> Result<(), String> {
    validate_artifact(
        case_dir,
        selector,
        ValidationOptions {
            ffprobe_bin: ffprobe.unwrap_or_else(|| "ffprobe".to_string()),
        },
    )
}
