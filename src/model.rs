use crate::util::{compact_json_value_if_well_formed, json_escape, path_to_file_url};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CaseManifest {
    pub schema_version: u32,
    pub case_id: String,
    pub title: String,
    pub created_unix: u64,
    pub tool_name: String,
    pub tool_version: String,
    pub platform: String,
    pub operator: Option<String>,
    pub host: Option<String>,
    pub device_id: Option<String>,
    pub device_serial: Option<String>,
    pub write_protect: Option<String>,
    pub acquisition_tool: Option<String>,
    pub evidence_hash: Option<String>,
    pub notes: Option<String>,
}

impl CaseManifest {
    pub fn to_json(&self) -> String {
        format!(
            "{{\n  \"schema_version\": {},\n  \"case_id\": \"{}\",\n  \"title\": \"{}\",\n  \"created_unix\": {},\n  \"tool_name\": \"{}\",\n  \"tool_version\": \"{}\",\n  \"platform\": \"{}\",\n  \"operator\": {},\n  \"host\": {},\n  \"device_id\": {},\n  \"device_serial\": {},\n  \"write_protect\": {},\n  \"acquisition_tool\": {},\n  \"evidence_hash\": {},\n  \"notes\": {}\n}}\n",
            self.schema_version,
            json_escape(&self.case_id),
            json_escape(&self.title),
            self.created_unix,
            json_escape(&self.tool_name),
            json_escape(&self.tool_version),
            json_escape(&self.platform),
            optional_string_json(self.operator.as_deref()),
            optional_string_json(self.host.as_deref()),
            optional_string_json(self.device_id.as_deref()),
            optional_string_json(self.device_serial.as_deref()),
            optional_string_json(self.write_protect.as_deref()),
            optional_string_json(self.acquisition_tool.as_deref()),
            optional_string_json(self.evidence_hash.as_deref()),
            optional_string_json(self.notes.as_deref())
        )
    }
}

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub hash_files: bool,
    pub use_ffprobe: bool,
    pub max_depth: Option<usize>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            hash_files: false,
            use_ffprobe: true,
            max_depth: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProbeSummary {
    pub ok: bool,
    pub raw_json: Option<String>,
    pub error: Option<String>,
    pub duration_seconds: Option<f64>,
    pub format_name: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl ProbeSummary {
    pub fn skipped() -> Self {
        Self {
            ok: false,
            raw_json: None,
            error: Some("ffprobe skipped".to_string()),
            duration_seconds: None,
            format_name: None,
            video_codec: None,
            audio_codec: None,
            width: None,
            height: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceProfile {
    pub lane: String,
    pub vendor: String,
    pub parser: String,
    pub confidence: String,
    pub recommended_action: String,
    pub evidence: Vec<String>,
}

impl SourceProfile {
    pub fn generic_media(evidence: impl Into<String>) -> Self {
        Self {
            lane: "generic-video".to_string(),
            vendor: "Generic media".to_string(),
            parser: "generic_media".to_string(),
            confidence: "medium".to_string(),
            recommended_action: "Use ffprobe/FFmpeg first; preserve original and export derived clips only when requested.".to_string(),
            evidence: vec![evidence.into()],
        }
    }

    pub fn to_json(&self) -> String {
        let evidence = self
            .evidence
            .iter()
            .map(|item| format!("\"{}\"", json_escape(item)))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"lane\":\"{}\",\"vendor\":\"{}\",\"parser\":\"{}\",\"confidence\":\"{}\",\"recommended_action\":\"{}\",\"evidence\":[{}]}}",
            json_escape(&self.lane),
            json_escape(&self.vendor),
            json_escape(&self.parser),
            json_escape(&self.confidence),
            json_escape(&self.recommended_action),
            evidence
        )
    }
}

#[derive(Debug, Clone)]
pub struct VideoRecord {
    pub id: String,
    pub source_path: PathBuf,
    pub relative_path: String,
    pub extension: String,
    pub size_bytes: u64,
    pub modified_unix: Option<u64>,
    pub sha256: Option<String>,
    pub hash_status: String,
    pub probe: ProbeSummary,
    pub confidence: String,
    pub source_profile: SourceProfile,
}

impl VideoRecord {
    pub fn to_json(&self) -> String {
        let source_path = self.source_path.to_string_lossy();
        let file_url = path_to_file_url(&self.source_path);
        let sha256 = optional_string_json(self.sha256.as_deref());
        let modified_unix = optional_u64_json(self.modified_unix);
        let duration = optional_f64_json(self.probe.duration_seconds);
        let width = optional_u32_json(self.probe.width);
        let height = optional_u32_json(self.probe.height);
        let ffprobe_json = match &self.probe.raw_json {
            Some(raw) => {
                compact_json_value_if_well_formed(raw).unwrap_or_else(|| "null".to_string())
            }
            None => "null".to_string(),
        };
        format!(
            "{{\"id\":\"{}\",\"source_path\":\"{}\",\"file_url\":\"{}\",\"relative_path\":\"{}\",\"extension\":\"{}\",\"size_bytes\":{},\"modified_unix\":{},\"sha256\":{},\"hash_status\":\"{}\",\"confidence\":\"{}\",\"source_profile\":{},\"duration_seconds\":{},\"format_name\":{},\"video_codec\":{},\"audio_codec\":{},\"width\":{},\"height\":{},\"ffprobe_ok\":{},\"ffprobe_error\":{},\"ffprobe\":{}}}",
            json_escape(&self.id),
            json_escape(&source_path),
            json_escape(&file_url),
            json_escape(&self.relative_path),
            json_escape(&self.extension),
            self.size_bytes,
            modified_unix,
            sha256,
            json_escape(&self.hash_status),
            json_escape(&self.confidence),
            self.source_profile.to_json(),
            duration,
            optional_string_json(self.probe.format_name.as_deref()),
            optional_string_json(self.probe.video_codec.as_deref()),
            optional_string_json(self.probe.audio_codec.as_deref()),
            width,
            height,
            self.probe.ok,
            optional_string_json(self.probe.error.as_deref()),
            ffprobe_json
        )
    }

    pub fn to_tsv_row(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            self.id,
            self.source_path.to_string_lossy(),
            self.relative_path,
            self.extension,
            self.size_bytes,
            self.sha256.as_deref().unwrap_or(""),
            self.source_profile.vendor,
            self.source_profile.parser,
            self.source_profile.confidence
        )
    }
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub source_path: PathBuf,
    pub scanned_unix: u64,
    pub video_count: usize,
    pub total_bytes: u64,
    pub warnings: Vec<String>,
    pub options: ScanOptions,
    pub records: Vec<VideoRecord>,
}

impl ScanResult {
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str("  \"schema_version\": 1,\n");
        out.push_str(&format!(
            "  \"source_path\": \"{}\",\n",
            json_escape(&self.source_path.to_string_lossy())
        ));
        out.push_str(&format!("  \"scanned_unix\": {},\n", self.scanned_unix));
        out.push_str(&format!("  \"video_count\": {},\n", self.video_count));
        out.push_str(&format!("  \"total_bytes\": {},\n", self.total_bytes));
        out.push_str("  \"warnings\": [\n");
        for (index, warning) in self.warnings.iter().enumerate() {
            out.push_str(&format!("    \"{}\"", json_escape(warning)));
            if index + 1 != self.warnings.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ],\n");
        out.push_str("  \"options\": {\n");
        out.push_str(&format!(
            "    \"hash_files\": {},\n",
            self.options.hash_files
        ));
        out.push_str(&format!(
            "    \"use_ffprobe\": {},\n",
            self.options.use_ffprobe
        ));
        match self.options.max_depth {
            Some(max_depth) => out.push_str(&format!("    \"max_depth\": {}\n", max_depth)),
            None => out.push_str("    \"max_depth\": null\n"),
        }
        out.push_str("  },\n");
        out.push_str("  \"videos\": [\n");
        for (index, record) in self.records.iter().enumerate() {
            out.push_str("    ");
            out.push_str(&record.to_json());
            if index + 1 != self.records.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ]\n");
        out.push_str("}\n");
        out
    }
}

fn optional_string_json(value: Option<&str>) -> String {
    match value {
        Some(value) => format!("\"{}\"", json_escape(value)),
        None => "null".to_string(),
    }
}

fn optional_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn optional_u32_json(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn optional_f64_json(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "null".to_string())
}
