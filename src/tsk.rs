use crate::audit;
use crate::util::{json_escape, now_unix, unique_path, write_text};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const DEFAULT_MAX_ENTRIES: usize = 20_000;
const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "m4v", "avi", "mkv", "wmv", "asf", "mpg", "mpeg", "mts", "m2ts", "ts", "3gp",
    "webm", "flv", "dav", "dav_", "nov", "ave", "g64", "g64x", "glv", "blk", "264", "265", "h264",
    "h265", "hevc",
];

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
    fn to_json(&self) -> String {
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
    fn to_json(&self) -> String {
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

pub fn inspect_image(
    case_dir: &Path,
    image_path: &Path,
    options: &TskInspectOptions,
) -> Result<TskInspectResult, String> {
    if options.max_entries == 0 {
        return Err("--max-entries must be greater than 0".to_string());
    }
    let image_path = canonical_image_path(image_path)?;
    let inspected_unix = now_unix()?;
    let mut warnings = Vec::new();

    let mmls_log_path = unique_path(
        &case_dir
            .join("evidence/logs")
            .join(format!("tsk-mmls-{inspected_unix}.txt")),
    );
    let mmls_args = vec![tsk_path_string(&image_path)];
    let mmls = run_capture(&options.mmls_bin, &mmls_args);
    let partitions = match &mmls {
        Ok(output) if output.status_success => {
            write_text(&mmls_log_path, &output.combined_text())
                .map_err(|err| format!("failed to write mmls log: {err}"))?;
            parse_mmls_partitions(&output.stdout)
        }
        Ok(output) => {
            write_text(&mmls_log_path, &output.combined_text())
                .map_err(|err| format!("failed to write mmls log: {err}"))?;
            warnings.push(format!(
                "mmls did not complete successfully; using explicit/default offset: {}",
                output.stderr.trim()
            ));
            Vec::new()
        }
        Err(err) => {
            write_text(&mmls_log_path, &format!("mmls unavailable: {err}\n"))
                .map_err(|write_err| format!("failed to write mmls log: {write_err}"))?;
            warnings.push(format!(
                "mmls unavailable; using explicit/default offset: {err}"
            ));
            Vec::new()
        }
    };

    let partition_offset = choose_partition_offset(&partitions, options.partition_offset);
    let fls_log_path = unique_path(
        &case_dir
            .join("evidence/logs")
            .join(format!("tsk-fls-{inspected_unix}.txt")),
    );
    let fls_args = fls_args(&image_path, partition_offset);
    let fls = run_capture(&options.fls_bin, &fls_args)?;
    write_text(&fls_log_path, &fls.combined_text())
        .map_err(|err| format!("failed to write fls log: {err}"))?;
    if !fls.status_success {
        return Err(format!(
            "fls failed at offset {}: {}",
            partition_offset,
            fls.stderr.trim()
        ));
    }

    let mut entries = parse_fls_entries(&fls.stdout);
    if entries.len() > options.max_entries {
        warnings.push(format!(
            "filesystem listing truncated from {} to {} entries",
            entries.len(),
            options.max_entries
        ));
        entries.truncate(options.max_entries);
    }

    let entries_jsonl_path = unique_path(
        &case_dir
            .join("db/filesystem")
            .join(format!("tsk-files-{inspected_unix}.jsonl")),
    );
    let jsonl = entries
        .iter()
        .map(FlsEntry::to_json)
        .collect::<Vec<_>>()
        .join("\n");
    write_text(&entries_jsonl_path, &(jsonl + "\n"))
        .map_err(|err| format!("failed to write filesystem entries: {err}"))?;

    let summary_path = unique_path(
        &case_dir
            .join("db/filesystem")
            .join(format!("tsk-inspection-{inspected_unix}.json")),
    );
    let summary = inspect_summary_json(&InspectSummaryInput {
        image_path: &image_path,
        inspected_unix,
        partition_offset,
        partitions: &partitions,
        entries: &entries,
        warnings: &warnings,
        mmls_log_path: &mmls_log_path,
        fls_log_path: &fls_log_path,
        entries_jsonl_path: &entries_jsonl_path,
    });
    write_text(&summary_path, &summary)
        .map_err(|err| format!("failed to write filesystem inspection summary: {err}"))?;

    append_tsk_audit(
        case_dir,
        &format!(
            "{{\"schema_version\":1,\"event\":\"inspect-image-filesystem\",\"inspected_unix\":{},\"image_path\":\"{}\",\"partition_offset\":{},\"partition_count\":{},\"entry_count\":{},\"deleted_count\":{},\"video_candidate_count\":{},\"mmls_version\":\"{}\",\"fls_version\":\"{}\",\"mmls_log_path\":\"{}\",\"fls_log_path\":\"{}\",\"entries_jsonl_path\":\"{}\",\"summary_path\":\"{}\",\"warnings\":{}}}",
            inspected_unix,
            json_escape(&image_path.to_string_lossy()),
            partition_offset,
            partitions.len(),
            entries.len(),
            entries.iter().filter(|entry| entry.deleted).count(),
            entries.iter().filter(|entry| entry.video_candidate).count(),
            json_escape(&tsk_command_version(&options.mmls_bin)),
            json_escape(&tsk_command_version(&options.fls_bin)),
            json_escape(&mmls_log_path.to_string_lossy()),
            json_escape(&fls_log_path.to_string_lossy()),
            json_escape(&entries_jsonl_path.to_string_lossy()),
            json_escape(&summary_path.to_string_lossy()),
            audit::json_string_array(&warnings)
        ),
    )?;

    Ok(TskInspectResult {
        image_path,
        inspected_unix,
        partition_offset,
        partitions,
        entries,
        warnings,
        mmls_log_path,
        fls_log_path,
        entries_jsonl_path,
        summary_path,
    })
}

pub fn recover_inode(
    case_dir: &Path,
    image_path: &Path,
    options: &TskRecoverOptions,
) -> Result<TskRecoverResult, String> {
    let image_path = canonical_image_path(image_path)?;
    if options.inode.trim().is_empty() {
        return Err("inode must not be empty".to_string());
    }
    let recovered_unix = now_unix()?;
    let output_path = match &options.output_path {
        Some(path) => {
            if path.exists() {
                return Err(format!(
                    "output already exists: {} (choose a new --output path)",
                    path.display()
                ));
            }
            path.to_path_buf()
        }
        None => unique_path(
            &case_dir
                .join("artifacts/recovered/filesystem")
                .join(format!("inode_{}.bin", sanitize_filename(&options.inode))),
        ),
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create recovery output directory: {err}"))?;
    }

    let args = icat_args(&image_path, options);
    let output = File::create(&output_path)
        .map_err(|err| format!("failed to create {}: {err}", output_path.display()))?;
    let result = Command::new(&options.icat_bin)
        .args(&args)
        .stdout(Stdio::from(output))
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| format!("failed to run {}: {err}", options.icat_bin))?;
    if !result.status.success() {
        let _ = fs::remove_file(&output_path);
        return Err(format!(
            "icat failed for inode {} at offset {}: {}",
            options.inode,
            options.partition_offset,
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }

    let output_path = output_path
        .canonicalize()
        .map_err(|err| format!("failed to canonicalize recovered output: {err}"))?;
    let size_bytes = fs::metadata(&output_path)
        .map_err(|err| format!("failed to read recovered output metadata: {err}"))?
        .len();
    let sha256 = audit::digest_file(&output_path)?;
    let validation_status = "candidate-unvalidated".to_string();

    append_tsk_audit(
        case_dir,
        &format!(
            "{{\"schema_version\":1,\"event\":\"recover-inode\",\"recovered_unix\":{},\"image_path\":\"{}\",\"partition_offset\":{},\"inode\":\"{}\",\"output_path\":\"{}\",\"size_bytes\":{},\"sha256\":\"{}\",\"validation_status\":\"{}\",\"recover_deleted\":{},\"include_slack\":{},\"skip_sparse_holes\":{},\"icat_version\":\"{}\",\"command\":\"{}\",\"command_args\":{}}}",
            recovered_unix,
            json_escape(&image_path.to_string_lossy()),
            options.partition_offset,
            json_escape(&options.inode),
            json_escape(&output_path.to_string_lossy()),
            size_bytes,
            json_escape(&sha256),
            json_escape(&validation_status),
            options.recover_deleted,
            options.include_slack,
            options.skip_sparse_holes,
            json_escape(&tsk_command_version(&options.icat_bin)),
            json_escape(&options.icat_bin),
            audit::json_string_array(&args)
        ),
    )?;

    Ok(TskRecoverResult {
        image_path,
        output_path,
        recovered_unix,
        partition_offset: options.partition_offset,
        inode: options.inode.clone(),
        size_bytes,
        sha256,
        validation_status,
    })
}

struct InspectSummaryInput<'a> {
    image_path: &'a Path,
    inspected_unix: u64,
    partition_offset: u64,
    partitions: &'a [MmlsPartition],
    entries: &'a [FlsEntry],
    warnings: &'a [String],
    mmls_log_path: &'a Path,
    fls_log_path: &'a Path,
    entries_jsonl_path: &'a Path,
}

fn inspect_summary_json(input: &InspectSummaryInput<'_>) -> String {
    let partitions_json = input
        .partitions
        .iter()
        .map(MmlsPartition::to_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\n  \"schema_version\": 1,\n  \"image_path\": \"{}\",\n  \"inspected_unix\": {},\n  \"partition_offset\": {},\n  \"partition_count\": {},\n  \"entry_count\": {},\n  \"deleted_count\": {},\n  \"video_candidate_count\": {},\n  \"warnings\": {},\n  \"mmls_log_path\": \"{}\",\n  \"fls_log_path\": \"{}\",\n  \"entries_jsonl_path\": \"{}\",\n  \"partitions\": [{}]\n}}\n",
        json_escape(&input.image_path.to_string_lossy()),
        input.inspected_unix,
        input.partition_offset,
        input.partitions.len(),
        input.entries.len(),
        input.entries.iter().filter(|entry| entry.deleted).count(),
        input
            .entries
            .iter()
            .filter(|entry| entry.video_candidate)
            .count(),
        audit::json_string_array(input.warnings),
        json_escape(&input.mmls_log_path.to_string_lossy()),
        json_escape(&input.fls_log_path.to_string_lossy()),
        json_escape(&input.entries_jsonl_path.to_string_lossy()),
        partitions_json
    )
}

fn append_tsk_audit(case_dir: &Path, body_json: &str) -> Result<(), String> {
    audit::append_chained_jsonl(&case_dir.join("evidence/logs/tsk-audit.jsonl"), body_json)
}

fn canonical_image_path(path: &Path) -> Result<PathBuf, String> {
    if !path.is_file() {
        return Err(format!("forensic image is not a file: {}", path.display()));
    }
    Ok(path.to_path_buf())
}

fn run_capture(binary: &str, args: &[String]) -> Result<CommandOutput, String> {
    let output = Command::new(binary)
        .args(args)
        .output()
        .map_err(|err| format!("failed to run {binary}: {err}"))?;
    Ok(CommandOutput {
        status_success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

#[derive(Debug, Clone)]
struct CommandOutput {
    status_success: bool,
    stdout: String,
    stderr: String,
}

impl CommandOutput {
    fn combined_text(&self) -> String {
        format!(
            "status_success: {}\n\nstdout:\n{}\n\nstderr:\n{}\n",
            self.status_success, self.stdout, self.stderr
        )
    }
}

fn tsk_command_version(binary: &str) -> String {
    match Command::new(binary).arg("-V").output() {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("unknown")
            .trim()
            .to_string(),
        Ok(output) => format!(
            "unavailable: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(err) => format!("unavailable: {err}"),
    }
}

fn parse_mmls_partitions(text: &str) -> Vec<MmlsPartition> {
    text.lines()
        .filter_map(|line| {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            if fields.len() < 5 || !fields[0].ends_with(':') {
                return None;
            }
            let start = fields.get(2)?.parse::<u64>().ok()?;
            let end = fields.get(3)?.parse::<u64>().ok()?;
            let length = fields.get(4)?.parse::<u64>().ok()?;
            let slot_type = fields[1];
            let description = if fields.len() > 5 {
                fields[5..].join(" ")
            } else {
                String::new()
            };
            let description_lc = description.to_ascii_lowercase();
            let allocated = slot_type != "Meta"
                && slot_type != "-------"
                && !description_lc.contains("unallocated")
                && !description_lc.contains("primary table");
            Some(MmlsPartition {
                slot: fields[0].trim_end_matches(':').to_string(),
                start,
                end,
                length,
                description,
                allocated,
            })
        })
        .collect()
}

fn choose_partition_offset(partitions: &[MmlsPartition], explicit: Option<u64>) -> u64 {
    explicit.unwrap_or_else(|| {
        partitions
            .iter()
            .find(|partition| partition.allocated)
            .map(|partition| partition.start)
            .unwrap_or(0)
    })
}

fn parse_fls_entries(text: &str) -> Vec<FlsEntry> {
    text.lines().filter_map(parse_fls_entry).collect()
}

fn parse_fls_entry(line: &str) -> Option<FlsEntry> {
    let raw = line.trim();
    if raw.is_empty() {
        return None;
    }
    let (left, right) = raw.split_once(':')?;
    let tokens = left.split_whitespace().collect::<Vec<_>>();
    let file_type = tokens
        .iter()
        .find(|token| token.contains('/'))
        .map(|token| token.trim_matches('+').to_string());
    let inode = tokens
        .iter()
        .rev()
        .find(|token| token.chars().any(|ch| ch.is_ascii_digit()))
        .map(|token| token.trim_matches('+').to_string());
    let path = right.trim().to_string();
    let deleted = tokens.contains(&"*") || path.contains("(deleted)");
    let video_candidate = path_has_video_extension(&path);

    Some(FlsEntry {
        raw_line: raw.to_string(),
        file_type,
        inode,
        path: Some(path),
        deleted,
        video_candidate,
    })
}

fn path_has_video_extension(path: &str) -> bool {
    let Some((_, extension)) = path.rsplit_once('.') else {
        return false;
    };
    let extension = extension
        .trim()
        .trim_end_matches("(deleted)")
        .trim()
        .to_ascii_lowercase();
    VIDEO_EXTENSIONS
        .iter()
        .any(|known| extension.eq_ignore_ascii_case(known))
}

fn fls_args(image_path: &Path, offset: u64) -> Vec<String> {
    vec![
        "-r".to_string(),
        "-p".to_string(),
        "-o".to_string(),
        offset.to_string(),
        tsk_path_string(image_path),
    ]
}

fn icat_args(image_path: &Path, options: &TskRecoverOptions) -> Vec<String> {
    let mut args = Vec::new();
    if options.skip_sparse_holes {
        args.push("-h".to_string());
    }
    if options.recover_deleted {
        args.push("-r".to_string());
    }
    if options.include_slack {
        args.push("-s".to_string());
    }
    args.extend([
        "-o".to_string(),
        options.partition_offset.to_string(),
        tsk_path_string(image_path),
        options.inode.clone(),
    ]);
    args
}

fn tsk_path_string(path: &Path) -> String {
    let raw = audit::path_string(path);
    raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_string()
}

fn sanitize_filename(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "unknown".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{
        TskRecoverOptions, choose_partition_offset, fls_args, icat_args, parse_fls_entry,
        parse_mmls_partitions,
    };
    use std::path::Path;

    #[test]
    fn parses_mmls_allocated_partition_offsets() {
        let text = "\
DOS Partition Table
Offset Sector: 0
Units are in 512-byte sectors

      Slot      Start        End          Length       Description
000:  Meta      0000000000   0000000000   0000000001   Primary Table (#0)
001:  -------   0000000000   0000002047   0000002048   Unallocated
002:  000:000   0000002048   0004095999   0004093952   NTFS / exFAT (0x07)
";
        let partitions = parse_mmls_partitions(text);
        assert_eq!(partitions.len(), 3);
        assert_eq!(choose_partition_offset(&partitions, None), 2048);
        assert_eq!(choose_partition_offset(&partitions, Some(63)), 63);
    }

    #[test]
    fn parses_deleted_fls_video_entries() {
        let entry =
            parse_fls_entry("r/r * 1304-128-1: /BLACKBOX/event001.mp4 (deleted)").expect("entry");
        assert_eq!(entry.file_type.as_deref(), Some("r/r"));
        assert_eq!(entry.inode.as_deref(), Some("1304-128-1"));
        assert_eq!(
            entry.path.as_deref(),
            Some("/BLACKBOX/event001.mp4 (deleted)")
        );
        assert!(entry.deleted);
        assert!(entry.video_candidate);
    }

    #[test]
    fn builds_tsk_command_args() {
        let image = Path::new("/cases/image.raw");
        assert_eq!(
            fls_args(image, 2048),
            vec!["-r", "-p", "-o", "2048", "/cases/image.raw"]
        );

        let options = TskRecoverOptions {
            partition_offset: 2048,
            inode: "1304-128-1".to_string(),
            output_path: None,
            recover_deleted: true,
            include_slack: false,
            skip_sparse_holes: true,
            icat_bin: "icat".to_string(),
        };
        assert_eq!(
            icat_args(image, &options),
            vec!["-h", "-r", "-o", "2048", "/cases/image.raw", "1304-128-1"]
        );
    }
}
