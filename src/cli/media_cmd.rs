use crate::artifacts::{FrameCaptureOptions, ProxyOptions, ThumbnailOptions};
use crate::carve::CarveOptions;
use crate::cli::handlers::{
    capture_frame, carve_file, confirm_playback, export_video, make_proxy, make_thumbnail,
    validate_artifact,
};
use crate::playback::PlaybackConfirmationOptions;
use crate::validation::ValidationOptions;
use crate::video_export::{ExportFormat, ExportOptions};
use std::path::{Path, PathBuf};

pub struct ExportVideoCliInput {
    pub format: String,
    pub start: Option<f64>,
    pub duration: Option<f64>,
    pub output: Option<PathBuf>,
    pub operator: Option<String>,
    pub ffmpeg: Option<String>,
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
            operator: input.operator,
            ffmpeg_bin: input.ffmpeg.unwrap_or_else(|| "ffmpeg".to_string()),
        },
    )
}

pub fn run_make_proxy(
    case_dir: &Path,
    selector: &str,
    max_width: Option<u32>,
    output: Option<PathBuf>,
    operator: Option<String>,
    ffmpeg: Option<String>,
) -> Result<(), String> {
    make_proxy(
        case_dir,
        selector,
        ProxyOptions {
            max_width: max_width.unwrap_or_else(|| ProxyOptions::default().max_width),
            output_path: output,
            operator,
            ffmpeg_bin: ffmpeg.unwrap_or_else(|| "ffmpeg".to_string()),
        },
    )
}

pub fn run_make_thumbnail(
    case_dir: &Path,
    selector: &str,
    time: Option<f64>,
    output: Option<PathBuf>,
    operator: Option<String>,
    ffmpeg: Option<String>,
) -> Result<(), String> {
    make_thumbnail(
        case_dir,
        selector,
        ThumbnailOptions {
            time_seconds: time.unwrap_or(0.0),
            output_path: output,
            operator,
            ffmpeg_bin: ffmpeg.unwrap_or_else(|| "ffmpeg".to_string()),
        },
    )
}

pub fn run_capture_frame(
    case_dir: &Path,
    selector: &str,
    time: Option<f64>,
    output: Option<PathBuf>,
    operator: Option<String>,
    ffmpeg: Option<String>,
) -> Result<(), String> {
    capture_frame(
        case_dir,
        selector,
        FrameCaptureOptions {
            time_seconds: time.unwrap_or(0.0),
            output_path: output,
            operator,
            ffmpeg_bin: ffmpeg.unwrap_or_else(|| "ffmpeg".to_string()),
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
    operator: Option<String>,
    allow_external_source: bool,
) -> Result<(), String> {
    validate_artifact(
        case_dir,
        selector,
        ValidationOptions {
            ffprobe_bin: ffprobe.unwrap_or_else(|| "ffprobe".to_string()),
            operator,
            allow_external_source,
        },
    )
}

pub fn run_confirm_playback(
    case_dir: &Path,
    selector: &str,
    playback_tool: Option<String>,
    notes: Option<String>,
    operator: Option<String>,
) -> Result<(), String> {
    confirm_playback(
        case_dir,
        selector,
        PlaybackConfirmationOptions {
            operator,
            playback_tool,
            notes,
        },
    )
}
