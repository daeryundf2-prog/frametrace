use std::path::PathBuf;

pub struct IndexedVideoRow {
    pub id: String,
    pub source_path: String,
    pub file_url: String,
    pub relative_path: String,
    pub extension: String,
    pub size_bytes: u64,
    pub modified_unix: Option<u64>,
    pub sha256: Option<String>,
    pub hash_status: String,
    pub confidence: String,
    pub source_profile_json: String,
    pub duration_seconds: Option<f64>,
    pub format_name: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub width: Option<u64>,
    pub height: Option<u64>,
    pub ffprobe_ok: bool,
    pub ffprobe_error: Option<String>,
    pub ffprobe_json: Option<String>,
    pub record_json: String,
}

pub struct VideoIdRow {
    pub id: String,
    pub source_path: String,
}

pub struct CaseDbSummary {
    pub path: PathBuf,
    pub video_count: u64,
    pub scan_run_count: u64,
    pub evidence_source_count: u64,
    pub job_count: u64,
    pub active_job_count: u64,
}

pub struct EvidenceSourceInput {
    pub kind: String,
    pub path: PathBuf,
    pub source_id: Option<String>,
    pub write_protect: Option<String>,
    pub acquisition_tool: Option<String>,
    pub evidence_hash: Option<String>,
    pub notes: Option<String>,
    pub metadata_json: Option<String>,
}

pub struct EvidenceSourceRow {
    pub source_id: String,
    pub kind: String,
    pub path: String,
}

pub struct JobRecord {
    pub job_id: String,
    pub job_type: String,
    pub status: String,
}

pub struct DbBenchmarkResult {
    pub path: PathBuf,
    pub rows: usize,
    pub elapsed_ms: u128,
}
