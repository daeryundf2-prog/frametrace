//! Deleted-video triage for an inspected image: lists what can be
//! recovered without reading it out, so the review shows deleted videos
//! next to the live ones and only the clips examined or selected are read.
//!
//! Two sources, both from the newest filesystem inspection:
//!
//! * **Deleted directory entries** (fls `*`): `istat` names the units
//!   `icat -r` would read. Compared with the units unallocated now
//!   (`blkls -l`), each entry is graded — `intact` (no unit reused by a live
//!   file), `partial`, `overwritten` — and its first unit is checked against
//!   the extension's container signature.
//! * **Unallocated space**: `blkls` streams only the free units of the image
//!   (no raw export) through the carve signature scanner; each hit is mapped
//!   back to its image position. A hit inside a deleted entry's own units is
//!   that entry, never a second candidate.
//!
//! Nothing but the candidate list is written. [`ensure_candidate`] reads a
//! candidate out — `icat -r`, or the unit runs via `blkcat` trimmed by the
//! carve extent rules — the first time it is played or exported, logs it in
//! `evidence/logs/tsk-audit.jsonl` and validates it with ffprobe.

use crate::lazy_extract::path_key;
use crate::tool_policy::resolve_tool_binary;
use crate::util::{json_escape, now_unix, write_text};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// Marker the viewer splits these rows out of `flsEntries` by (the rows
/// ride in that array so the bundle format stays unchanged).
pub const ROW_KIND: &str = "recovery-candidate";
/// Most signature hits one unallocated scan keeps.
const MAX_HITS: usize = 50_000;
/// Upper bound read out for one unallocated candidate (the carve default).
const MAX_CARVE_SPAN: u64 = 512 * 1024 * 1024;
/// Smaller unallocated hits are noise for video review.
const MIN_CARVE_SPAN: u64 = 16 * 1024;
/// Parallel istat/blkcat probes for deleted entries.
const PROBE_WORKERS: usize = 4;

const PREVIEW_FILESYSTEM: &str = "artifacts/Recovery/Filesystem";
const PREVIEW_UNALLOCATED: &str = "artifacts/Recovery/Unallocated";
const PREVIEW_TAILS: &str = "artifacts/Recovery/AllocatedTails";
/// Allocated files smaller than this are not checked for an older tail.
const MIN_TAIL_FILE: u64 = 1024 * 1024;
/// Tails smaller than this are not scanned.
const MIN_TAIL_BYTES: u64 = 1024 * 1024;

/// One deleted video the review can show before it is read out.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecoveryCandidate {
    /// Always [`ROW_KIND`].
    pub kind: String,
    /// `fls:<inode>` (the id the viewer always gave deleted entries, so
    /// marks stay attached) or `ucarve:<image byte offset>`.
    pub id: String,
    /// `deleted-entry` or `unallocated`.
    pub source: String,
    #[serde(default)]
    pub inode: Option<String>,
    #[serde(default)]
    pub card_path: Option<String>,
    pub name: String,
    pub extension: String,
    #[serde(default)]
    pub signature: Option<String>,
    /// Entry: its recorded size. Unallocated: bytes up to the next
    /// signature (an upper bound until the candidate is read out).
    pub size_bytes: u64,
    pub size_exact: bool,
    /// Byte offset of the first data byte in the image.
    pub image_offset: u64,
    /// Unallocated: offset in the `blkls` stream (maps through the map file).
    #[serde(default)]
    pub stream_offset: Option<u64>,
    pub total_units: u64,
    pub overwritten_units: u64,
    /// `intact` | `partial` | `overwritten` | `unknown` | `unallocated`.
    pub grade: String,
    #[serde(default)]
    pub header_ok: Option<bool>,
    /// Where the candidate is read out to (case-relative, `/`-separated).
    pub preview_path: String,
    pub note: String,
    pub image_path: String,
    /// Partition offset in 512-byte sectors (TSK `-o`).
    pub partition_offset: u64,
    pub unit_bytes: u64,
    /// The scanned region `stream_offset` refers to: [`STREAM_UNALLOCATED`]
    /// or [`STREAM_TAILS`].
    #[serde(default = "default_stream")]
    pub stream: String,
    /// Headerless frame runs: video frames in the run.
    #[serde(default)]
    pub video_frames: Option<u64>,
}

/// The free units of the partition (`blkls`).
pub const STREAM_UNALLOCATED: &str = "unallocated";
/// Allocated files past their current content: preallocated (format-free)
/// recordings' older footage, reserved recording files.
pub const STREAM_TAILS: &str = "allocated-tails";

fn default_stream() -> String {
    STREAM_UNALLOCATED.to_string()
}

/// Free units of the partition as sorted runs, in `blkls` stream order.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
struct UnitRuns {
    /// `(first unit address, unit count)`, ascending and non-adjacent.
    runs: Vec<(u64, u64)>,
}

impl UnitRuns {
    fn from_sorted(addresses: impl IntoIterator<Item = u64>) -> Self {
        let mut runs: Vec<(u64, u64)> = Vec::new();
        for address in addresses {
            match runs.last_mut() {
                Some((start, count)) if *start + *count == address => *count += 1,
                Some((start, count)) if address < *start + *count => {}
                _ => runs.push((address, 1)),
            }
        }
        Self { runs }
    }

    fn total(&self) -> u64 {
        self.runs.iter().map(|(_, count)| count).sum()
    }

    fn contains(&self, unit: u64) -> bool {
        let at = self.runs.partition_point(|(start, _)| *start <= unit);
        at > 0 && {
            let (start, count) = self.runs[at - 1];
            unit < start + count
        }
    }

    /// Image units holding stream units `[first, first + count)`.
    fn physical(&self, first: u64, count: u64) -> Vec<(u64, u64)> {
        let mut out = Vec::new();
        let mut skipped = 0u64;
        let mut wanted = count;
        let mut first = first;
        for &(start, run) in &self.runs {
            if wanted == 0 {
                break;
            }
            if first >= skipped + run {
                skipped += run;
                continue;
            }
            let inside = first - skipped;
            let take = (run - inside).min(wanted);
            out.push((start + inside, take));
            wanted -= take;
            first += take;
            skipped += run;
        }
        out
    }
}

/// The unit map of one scanned stream (free units, or allocated tails, in
/// stream order); its candidates are read through it.
#[derive(Debug, Serialize, Deserialize)]
struct UnallocMap {
    image_path: String,
    partition_offset: u64,
    unit_bytes: u64,
    stream_bytes: u64,
    runs: UnitRuns,
}

/// Map file name prefix of each stream.
fn map_prefix(stream: &str) -> &'static str {
    if stream == STREAM_TAILS {
        "tail-map-"
    } else {
        "unalloc-map-"
    }
}

/// What [`scan_deleted`] found.
#[derive(Debug, Default)]
pub struct ScanSummary {
    pub deleted_entries: usize,
    pub intact: usize,
    pub partial: usize,
    pub overwritten: usize,
    pub unknown: usize,
    pub unallocated_bytes: u64,
    pub carved: usize,
    /// Headerless frame runs (all regions).
    pub frame_runs: usize,
    /// Allocated files whose tail (past their current content) was scanned.
    pub tail_files: usize,
    pub tail_bytes: u64,
    /// Candidates found in those tails (signature + frame runs).
    pub tail_candidates: usize,
    pub warnings: Vec<String>,
    pub candidates_path: PathBuf,
}

/// Progress: `(done, total)` in abstract units (entries + scanned MiB).
pub type ScanProgress<'a> = dyn Fn(u64, u64) + Send + Sync + 'a;

/// Inspection facts a scan needs.
struct ImageTarget {
    image: String,
    offset: u64,
}

fn newest_inspection(case_dir: &Path) -> Result<ImageTarget, String> {
    let missing = || {
        "no filesystem inspection found; run inspect-image or inspect-e01 --filesystem first"
            .to_string()
    };
    let summary_path = newest_named(case_dir, "tsk-inspection-", ".json").ok_or_else(missing)?;
    let summary: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&summary_path)
            .map_err(|err| format!("failed to read {}: {err}", summary_path.display()))?,
    )
    .map_err(|err| format!("invalid {}: {err}", summary_path.display()))?;
    let image_path = summary
        .get("image_path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .ok_or_else(missing)?;
    let offset = summary
        .get("partition_offset")
        .and_then(|v| v.as_u64())
        .ok_or_else(missing)?;
    let image_path = crate::tsk::canonical_image_path(&image_path)?;
    Ok(ImageTarget {
        image: crate::tsk::tsk_path_string(&image_path),
        offset,
    })
}

fn tool(name: &str) -> Result<Command, String> {
    let binary = resolve_tool_binary(name, &[name])
        .map_err(|err| format!("{err} (install Sleuth Kit and ensure {name} is in PATH)"))?;
    let mut command = Command::new(binary);
    crate::util::hide_console(&mut command);
    Ok(command)
}

fn run_tool(name: &str, args: &[String]) -> Result<Vec<u8>, String> {
    let output = tool(name)?
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|err| format!("failed to run {name}: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "{name} failed: {}",
            crate::util::tool_output_text(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}

fn spawn_tool(name: &str, args: &[String]) -> Result<Child, String> {
    tool(name)?
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| format!("failed to run {name}: {err}"))
}

/// Bytes of one `blkls` unit: a sector on FAT/exFAT, a cluster/block
/// elsewhere (from `fsstat`).
fn unit_bytes(fsstat: &str, unit_name: &str) -> u64 {
    let field = |label: &str| {
        fsstat.lines().find_map(|line| {
            line.trim()
                .strip_prefix(label)
                .and_then(|rest| rest.trim().parse::<u64>().ok())
        })
    };
    if unit_name.eq_ignore_ascii_case("sector") {
        field("Sector Size:").unwrap_or(512)
    } else {
        field("Cluster Size:")
            .or_else(|| field("Block Size:"))
            .unwrap_or(4096)
    }
}

/// Unit name from the `blkls -l` header (`class|host|image|first_time|unit`).
fn parse_blkls_list(text: impl BufRead) -> (String, UnitRuns) {
    let mut unit = String::new();
    let mut addresses = Vec::new();
    let mut header_next = false;
    for line in text.lines().map_while(Result::ok) {
        let line = line.trim();
        if line.starts_with("class|") {
            header_next = true;
            continue;
        }
        if header_next {
            unit = line.rsplit('|').next().unwrap_or_default().to_string();
            header_next = false;
            continue;
        }
        if let Some((address, flag)) = line.split_once('|')
            && let Ok(address) = address.parse::<u64>()
            && flag == "f"
        {
            addresses.push(address);
        }
    }
    addresses.sort_unstable();
    (unit, UnitRuns::from_sorted(addresses))
}

/// Unit addresses `istat` lists for an entry (FAT/exFAT `Sectors:`;
/// `Direct Blocks:` elsewhere).
fn parse_istat_units(text: &str) -> Vec<u64> {
    let mut units = Vec::new();
    let mut collecting = false;
    for line in text.lines() {
        let line = line.trim();
        if line == "Sectors:" || line.starts_with("Direct Blocks:") {
            collecting = true;
            continue;
        }
        if !collecting || line.is_empty() {
            continue;
        }
        if line.contains(':') {
            break;
        }
        // istat pads the last cluster's unused slots with 0 (the boot
        // sector, never file data).
        units.extend(
            line.split_whitespace()
                .filter_map(|token| token.parse::<u64>().ok())
                .filter(|unit| *unit != 0),
        );
    }
    units
}

/// Whether `head` starts like the container `extension` names (`None`
/// when the extension has no signature rule).
fn header_matches(extension: &str, head: &[u8]) -> Option<bool> {
    match extension {
        "mp4" | "mov" | "m4v" | "3gp" => Some(
            head.len() >= 8
                && matches!(
                    &head[4..8],
                    b"ftyp" | b"moov" | b"mdat" | b"free" | b"wide" | b"skip"
                ),
        ),
        "avi" => Some(head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"AVI "),
        "ts" | "mts" | "m2ts" => Some(head.first() == Some(&0x47)),
        "dav" => Some(head.starts_with(b"DHAV")),
        _ => None,
    }
}

/// Units missing between consecutive listed units (the clusters TSK
/// skipped because a live file holds them).
fn skipped_units(units: &[u64]) -> u64 {
    units
        .windows(2)
        .map(|pair| pair[1].saturating_sub(pair[0]).saturating_sub(1))
        .sum()
}

fn grade(total: u64, overwritten: u64) -> &'static str {
    if total == 0 {
        "unknown"
    } else if overwritten == 0 {
        "intact"
    } else if overwritten >= total {
        "overwritten"
    } else {
        "partial"
    }
}

fn extension_of(path: &str) -> String {
    Path::new(path.trim_end_matches("(deleted)").trim())
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// Where a deleted entry is read out to: `<inode>_<card file name>`,
/// keeping the name and its extension.
fn entry_preview_path(inode: &str, card_path: &str) -> String {
    let leaf = Path::new(card_path.trim_end_matches("(deleted)").trim())
        .file_name()
        .map(|name| crate::util::sanitize_path_component(&name.to_string_lossy()))
        .filter(|name| !name.is_empty());
    let inode = crate::util::sanitize_filename(inode);
    match leaf {
        Some(leaf) => format!("{PREVIEW_FILESYSTEM}/{inode}_{leaf}"),
        None => format!("{PREVIEW_FILESYSTEM}/inode_{inode}.bin"),
    }
}

/// A deleted entry of the newest listing worth probing.
struct DeletedEntry {
    inode: String,
    path: String,
    size: Option<u64>,
    /// No video extension: kept only when its first bytes are a video
    /// container (a dashcam's extensionless in-progress recording).
    sniff: bool,
}

/// Deleted files without a video extension smaller than this are not
/// probed (config/log files).
const MIN_SNIFF_BYTES: u64 = 256 * 1024;

fn deleted_video_entries(case_dir: &Path) -> Result<Vec<DeletedEntry>, String> {
    let Some(path) = crate::lazy_extract::latest_entries_jsonl(case_dir) else {
        return Ok(Vec::new());
    };
    let text = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let mut seen = std::collections::HashSet::new();
    Ok(text
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line.trim()).ok())
        .filter(|row| row.get("deleted").and_then(|v| v.as_bool()) == Some(true))
        .filter_map(|row| {
            let video = row.get("video_candidate").and_then(|v| v.as_bool()) == Some(true);
            let file = row
                .get("file_type")
                .and_then(|v| v.as_str())
                .is_some_and(|kind| kind.ends_with('r'));
            let size = row.get("size_bytes").and_then(|v| v.as_u64());
            if !(video || file && size.is_none_or(|size| size >= MIN_SNIFF_BYTES)) {
                return None;
            }
            let inode = row.get("inode")?.as_str()?.to_string();
            // `1234(realloc)`: the inode now belongs to another file.
            if inode.is_empty() || !inode.chars().all(|c| c.is_ascii_digit() || c == '-') {
                return None;
            }
            seen.insert(inode.clone()).then(|| DeletedEntry {
                inode,
                path: row
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                size,
                sniff: !video,
            })
        })
        .collect())
}

/// `len` bytes at `stream_offset` of the unallocated stream, read with
/// blkcat from the units holding them (`None` when they cannot be read).
fn read_stream_bytes(
    image: &str,
    offset_arg: &str,
    free: &UnitRuns,
    unit: u64,
    stream_offset: u64,
    len: u64,
) -> Option<Vec<u8>> {
    let intra = stream_offset % unit;
    let units = (intra + len).div_ceil(unit);
    let mut data = Vec::new();
    for (address, count) in free.physical(stream_offset / unit, units) {
        data.extend(
            run_tool(
                "blkcat",
                &[
                    "-o".into(),
                    offset_arg.to_string(),
                    image.to_string(),
                    address.to_string(),
                    count.to_string(),
                ],
            )
            .ok()?,
        );
    }
    let start = intra as usize;
    let end = (start + len as usize).min(data.len());
    (start <= end).then(|| data[start..end].to_vec())
}

/// Structural size of an unallocated hit within `span`: RIFF declares it,
/// MP4 is walked box by box (only box headers are read). `None` when the
/// signature has no such rule or the first header does not parse.
fn probe_extent(
    read: &dyn Fn(u64, u64) -> Option<Vec<u8>>,
    offset: u64,
    signature: &str,
    span: u64,
) -> Option<u64> {
    match signature {
        "riff-avi" => {
            let head = read(offset, 8)?;
            let declared = u32::from_le_bytes(head.get(4..8)?.try_into().ok()?) as u64 + 8;
            (declared >= 12).then(|| declared.min(span))
        }
        "mp4-ftyp" => {
            let mut cursor = 0u64;
            for _ in 0..64 {
                if cursor + 8 > span {
                    break;
                }
                let Some(head) = read(offset + cursor, 16) else {
                    break;
                };
                if head.len() < 8
                    || !head[4..8]
                        .iter()
                        .all(|byte| byte.is_ascii_alphanumeric() || *byte == b' ')
                {
                    break;
                }
                let size = match u32::from_be_bytes(head[0..4].try_into().ok()?) as u64 {
                    1 if head.len() >= 16 => u64::from_be_bytes(head[8..16].try_into().ok()?),
                    0 => span - cursor,
                    size => size,
                };
                if size < 8 {
                    break;
                }
                cursor = cursor.saturating_add(size);
            }
            (cursor > 0).then(|| cursor.min(span))
        }
        _ => None,
    }
}

/// One probed deleted entry: (entry index, istat units, header check,
/// sniffed container of an extensionless entry).
type Probed = (usize, Vec<u64>, Option<bool>, Option<&'static str>);

/// Read counter around a scanned stream: progress, and every byte handed
/// to the frame-run scanner as the signature scan reads it (one pass).
struct Counting<'a, R> {
    inner: R,
    read: u64,
    reported: u64,
    on_bytes: &'a dyn Fn(u64),
    frames: crate::frame_carve::FrameScanner,
}

impl<R: Read> Read for Counting<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buf)?;
        self.frames.feed(&buf[..read]);
        self.read += read as u64;
        if self.read - self.reported >= 64 * 1024 * 1024 || read == 0 {
            self.reported = self.read;
            (self.on_bytes)(self.read);
        }
        Ok(read)
    }
}

/// What one stream pass found.
struct RegionScan {
    hits: Vec<crate::carve::CarveHit>,
    frames: Vec<crate::frame_carve::FrameRun>,
    stream_bytes: u64,
    hit_cap: bool,
}

/// Signature scan + frame-run scan of one stream in a single read.
fn scan_region(
    reader: impl Read,
    avcc: Option<crate::video_repair::AvccProbe>,
    on_bytes: &dyn Fn(u64),
) -> Result<RegionScan, String> {
    let mut counting = Counting {
        inner: BufReader::with_capacity(4 * 1024 * 1024, reader),
        read: 0,
        reported: 0,
        on_bytes,
        frames: crate::frame_carve::FrameScanner::new(avcc),
    };
    let hits = crate::carve::find_video_signatures_in(&mut counting, MAX_HITS)?;
    // The signature scan stops at its hit cap; the frame scan reads on.
    let mut rest = vec![0u8; 4 * 1024 * 1024];
    while counting
        .read(&mut rest)
        .map_err(|err| format!("failed to read the scan stream: {err}"))?
        > 0
    {}
    let hit_cap = hits.len() >= MAX_HITS;
    Ok(RegionScan {
        hits,
        stream_bytes: counting.read,
        frames: counting.frames.finish(),
        hit_cap,
    })
}

/// Reads the units of `runs` in order, one blkcat per run.
struct RunsReader<'a> {
    image: &'a str,
    offset_arg: &'a str,
    runs: std::vec::IntoIter<(u64, u64)>,
    child: Option<(Child, std::process::ChildStdout)>,
}

impl Read for RunsReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            if let Some((_, stdout)) = &mut self.child {
                let read = stdout.read(buf)?;
                if read > 0 {
                    return Ok(read);
                }
                if let Some((mut child, _)) = self.child.take() {
                    let _ = child.wait();
                }
            }
            let Some((address, count)) = self.runs.next() else {
                return Ok(0);
            };
            let mut child = spawn_tool(
                "blkcat",
                &[
                    "-o".into(),
                    self.offset_arg.to_string(),
                    self.image.to_string(),
                    address.to_string(),
                    count.to_string(),
                ],
            )
            .map_err(std::io::Error::other)?;
            let stdout = child.stdout.take().expect("piped blkcat stdout");
            self.child = Some((child, stdout));
        }
    }
}

/// Sniffs a container from a file's first bytes (deleted entries without a
/// video extension: an in-progress recording's temp file).
fn sniff_video(head: &[u8]) -> Option<&'static str> {
    if head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"AVI " {
        Some("avi")
    } else if head.len() >= 8 && matches!(&head[4..8], b"ftyp" | b"mdat" | b"moov" | b"wide") {
        Some("mp4")
    } else if head.starts_with(b"DHAV") {
        Some("dav")
    } else if head.len() > 376 && head[0] == 0x47 && head[188] == 0x47 && head[376] == 0x47 {
        Some("ts")
    } else {
        None
    }
}

/// One scanned region and how its candidates are named.
struct Region<'a> {
    stream: &'static str,
    /// Image units of the region in stream order.
    runs: &'a UnitRuns,
    /// `(stream offset, file path)` of the allocated file each tail
    /// starts — empty for unallocated space.
    owners: &'a [(u64, String)],
    preview_dir: &'static str,
    /// Id/name prefix letter: `u` (unallocated) or `t` (allocated tails).
    prefix: &'static str,
    label: &'static str,
}

impl Region<'_> {
    fn owner(&self, stream_offset: u64) -> Option<&str> {
        let at = self
            .owners
            .partition_point(|(start, _)| *start <= stream_offset);
        at.checked_sub(1).map(|at| self.owners[at].1.as_str())
    }
}

/// Candidate rows for one scanned region: container signatures (sized by
/// their structure) and headerless frame runs not inside one of them.
#[allow(clippy::too_many_arguments)]
fn add_region_candidates(
    target: &ImageTarget,
    offset_arg: &str,
    unit: u64,
    region: &Region<'_>,
    scan: &RegionScan,
    excluded: &dyn Fn(u64) -> bool,
    entry_by_first: Option<&HashMap<u64, usize>>,
    candidates: &mut Vec<RecoveryCandidate>,
    summary: &mut ScanSummary,
) {
    let grade = if region.stream == STREAM_TAILS {
        "tail"
    } else {
        "unallocated"
    };
    let owner_note = |offset: u64| {
        region
            .owner(offset)
            .map(|owner| format!(" · 위치: {owner} 안쪽"))
            .unwrap_or_default()
    };
    let address_of = |offset: u64| region.runs.physical(offset / unit, 1).first().map(|r| r.0);
    let boundaries: Vec<u64> = scan
        .hits
        .iter()
        .filter(|hit| hit.signature != "jpeg" && hit.nested_in.is_none())
        .map(|hit| hit.offset)
        .collect();
    // Stream spans the signature candidates claim: a frame run inside one
    // is that container's own frames.
    let mut covered: Vec<(u64, u64)> = Vec::new();
    // A raw H.264/H.265 start-code anchor inside a frame run is one of that
    // run's frames (AVI stores them Annex-B), not a second video.
    let inside_frame_run = |offset: u64| {
        scan.frames
            .iter()
            .any(|run| offset >= run.start && offset < run.end)
    };
    for hit in scan.hits.iter().filter(|hit| {
        hit.signature != "jpeg"
            && hit.nested_in.is_none()
            && !(hit.signature.ends_with("annexb") && inside_frame_run(hit.offset))
    }) {
        let intra = hit.offset % unit;
        let Some(address) = address_of(hit.offset) else {
            continue;
        };
        if intra == 0
            && let Some(at) = entry_by_first.and_then(|map| map.get(&address))
        {
            candidates[*at]
                .note
                .push_str(" · 빈 영역 스캔에서도 같은 위치에 영상 시그니처가 확인됐습니다");
            covered.push((hit.offset, hit.offset + candidates[*at].size_bytes));
            continue;
        }
        if excluded(address) {
            continue;
        }
        let next = boundaries
            .iter()
            .find(|offset| **offset > hit.offset)
            .copied()
            .unwrap_or(scan.stream_bytes);
        let span = (next - hit.offset).min(MAX_CARVE_SPAN);
        // The container's own structure (RIFF size, MP4 box chain) bounds
        // it far tighter than the next signature: shown as the size, and
        // what is read out on playback.
        let probed = probe_extent(
            &|offset, len| {
                read_stream_bytes(&target.image, offset_arg, region.runs, unit, offset, len)
            },
            hit.offset,
            &hit.signature,
            span,
        );
        let size = probed.unwrap_or(span);
        if size < MIN_CARVE_SPAN {
            continue;
        }
        covered.push((hit.offset, hit.offset + size));
        let image_offset = target.offset * 512 + address * unit + intra;
        let extension = if hit.extension.is_empty() {
            "bin".to_string()
        } else {
            hit.extension.clone()
        };
        let name = format!("{}carve_{image_offset}.{extension}", region.prefix);
        candidates.push(RecoveryCandidate {
            kind: ROW_KIND.to_string(),
            id: format!("{}carve:{image_offset}", region.prefix),
            source: region.stream.to_string(),
            inode: None,
            card_path: region.owner(hit.offset).map(str::to_string),
            name: name.clone(),
            extension: extension.clone(),
            signature: Some(hit.signature.clone()),
            size_bytes: size,
            size_exact: probed.is_some(),
            image_offset,
            stream_offset: Some(hit.offset),
            total_units: size.div_ceil(unit),
            overwritten_units: 0,
            grade: grade.to_string(),
            header_ok: Some(true),
            preview_path: format!("{}/{name}", region.preview_dir),
            note: if probed.is_some() {
                format!(
                    "{} {} 시그니처 — 크기는 컨테이너 구조(헤더)로 정했습니다; 조각났다면 실제 내용과 다를 수 있습니다{}",
                    region.label,
                    hit.signature,
                    owner_note(hit.offset)
                )
            } else {
                format!(
                    "{} {} 시그니처 — 크기는 다음 시그니처까지의 상한이며, 재생할 때 컨테이너 구조로 끝을 확정합니다{}",
                    region.label,
                    hit.signature,
                    owner_note(hit.offset)
                )
            },
            image_path: target.image.clone(),
            partition_offset: target.offset,
            unit_bytes: unit,
            stream: region.stream.to_string(),
            video_frames: None,
        });
        summary.carved += 1;
    }

    for run in &scan.frames {
        // An AVI frame run's H.264 payload can read as length-prefixed
        // NAL units too: the AVI run is the video.
        let inside_avi_run = run.kind == crate::frame_carve::RunKind::AvccNals
            && scan.frames.iter().any(|avi| {
                avi.kind == crate::frame_carve::RunKind::AviChunks
                    && run.start < avi.end
                    && avi.start < run.end
            });
        if inside_avi_run
            || covered
                .iter()
                .any(|(start, end)| run.start >= *start && run.start < *end)
        {
            continue;
        }
        let Some(address) = address_of(run.start) else {
            continue;
        };
        if excluded(address) {
            continue;
        }
        let intra = run.start % unit;
        let image_offset = target.offset * 512 + address * unit + intra;
        let (extension, what) = match run.kind {
            crate::frame_carve::RunKind::AviChunks => ("avi", "AVI 프레임 청크"),
            crate::frame_carve::RunKind::AvccNals => ("mp4", "H.264/H.265 프레임(NAL)"),
        };
        let size = run.end - run.start;
        let name = format!("{}frames_{image_offset}.{extension}", region.prefix);
        candidates.push(RecoveryCandidate {
            kind: ROW_KIND.to_string(),
            id: format!("{}frames:{image_offset}", region.prefix),
            source: region.stream.to_string(),
            inode: None,
            card_path: region.owner(run.start).map(str::to_string),
            name: name.clone(),
            extension: extension.to_string(),
            signature: Some(run.kind.signature().to_string()),
            size_bytes: size,
            size_exact: true,
            image_offset,
            stream_offset: Some(run.start),
            total_units: size.div_ceil(unit),
            overwritten_units: 0,
            grade: "frames".to_string(),
            header_ok: None,
            preview_path: format!("{}/{name}", region.preview_dir),
            note: format!(
                "{} 헤더 없이 남은 {what} 구간 — 영상 {}프레임; 재생하면 같은 카드의 정상 영상 헤더로 다시 묶습니다{}",
                region.label,
                run.video_frames,
                owner_note(run.start)
            ),
            image_path: target.image.clone(),
            partition_offset: target.offset,
            unit_bytes: unit,
            stream: region.stream.to_string(),
            video_frames: Some(run.video_frames),
        });
        summary.frame_runs += 1;
    }
}

/// The allocated-tail region: for each allocated file of at least
/// [`MIN_TAIL_FILE`], its units past its current content — the structural
/// end of an AVI/MP4 (RIFF size, box chain), or the whole file when it is
/// no video (a reserved recording file).
struct Tails {
    runs: UnitRuns,
    owners: Vec<(u64, String)>,
    files: usize,
}

fn allocated_tails(
    case_dir: &Path,
    target: &ImageTarget,
    offset_arg: &str,
    unit: u64,
    tick: &(dyn Fn(u64) + Sync),
) -> Tails {
    let files: Vec<(String, String, u64)> = crate::lazy_extract::latest_entries_jsonl(case_dir)
        .and_then(|path| fs::read_to_string(path).ok())
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|row| row.get("deleted").and_then(|v| v.as_bool()) != Some(true))
        .filter(|row| {
            row.get("file_type")
                .and_then(|v| v.as_str())
                .is_some_and(|kind| kind.ends_with('r'))
        })
        .filter_map(|row| {
            let size = row.get("size_bytes")?.as_u64()?;
            let path = row.get("path")?.as_str()?.to_string();
            let inode = row.get("inode")?.as_str()?.to_string();
            (size >= MIN_TAIL_FILE && !path.starts_with('$')).then_some((inode, path, size))
        })
        .collect();
    let found: std::sync::Mutex<Vec<FileTail>> = std::sync::Mutex::new(Vec::new());
    let next = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..(PROBE_WORKERS * 2).min(files.len()).max(1) {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let Some((inode, path, size)) = files.get(index) else {
                        break;
                    };
                    if let Some(tail) = file_tail(target, offset_arg, unit, inode, path, *size) {
                        found
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .push((index, tail.0, tail.1));
                    }
                }
            });
        }
    });
    tick(0);
    let mut found = found.into_inner().unwrap_or_else(|e| e.into_inner());
    found.sort_by_key(|(index, ..)| *index);
    let mut runs = Vec::new();
    let mut owners = Vec::new();
    let mut stream = 0u64;
    for (index, file_runs, bytes) in &found {
        owners.push((stream, files[*index].1.clone()));
        runs.extend(file_runs.iter().copied());
        stream += bytes;
    }
    Tails {
        runs: UnitRuns { runs },
        owners,
        files: found.len(),
    }
}

/// One file's tail: (file index, unit runs, bytes).
type FileTail = (usize, Vec<(u64, u64)>, u64);

/// `(unit runs, bytes)` of one allocated file past its current content.
fn file_tail(
    target: &ImageTarget,
    offset_arg: &str,
    unit: u64,
    inode: &str,
    path: &str,
    size: u64,
) -> Option<(Vec<(u64, u64)>, u64)> {
    let mut units = run_tool(
        "istat",
        &[
            "-o".into(),
            offset_arg.to_string(),
            target.image.clone(),
            inode.to_string(),
        ],
    )
    .map(|out| parse_istat_units(&String::from_utf8_lossy(&out)))
    .ok()?;
    units.truncate(size.div_ceil(unit) as usize);
    let file_runs = UnitRuns {
        runs: compress_units(&units),
    };
    let read = |offset: u64, len: u64| {
        read_stream_bytes(&target.image, offset_arg, &file_runs, unit, offset, len)
    };
    let head = read(0, 16)?;
    let content_end = match extension_of(path).as_str() {
        "avi" => {
            if head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"AVI " {
                let declared = u32::from_le_bytes(head[4..8].try_into().ok()?) as u64 + 8;
                // A size field still 0 (recording in progress) gives no end:
                // the tail cannot be told apart from the current clip.
                if (12..=size).contains(&declared) {
                    declared
                } else {
                    size
                }
            } else {
                0
            }
        }
        "mp4" | "mov" | "m4v" => {
            if head.len() >= 8 && matches!(&head[4..8], b"ftyp" | b"mdat" | b"moov" | b"wide") {
                probe_extent(&read, 0, "mp4-ftyp", size).unwrap_or(size)
            } else {
                0
            }
        }
        // Other video containers: their end is not modelled here.
        ext if VIDEO_EXTENSIONS_NO_TAIL.contains(&ext) => size,
        // No video: a reserved/preallocated recording file — all of it.
        _ => 0,
    };
    let first = (content_end / unit) as usize;
    let tail: Vec<u64> = units.get(first..)?.to_vec();
    let bytes = tail.len() as u64 * unit;
    (bytes >= MIN_TAIL_BYTES).then(|| (compress_units(&tail), bytes))
}

/// Video containers whose content end is not modelled (no tail scan).
const VIDEO_EXTENSIONS_NO_TAIL: &[&str] = &[
    "ts", "mts", "m2ts", "dav", "h264", "h265", "264", "265", "mkv", "wmv", "asf", "flv", "3gp",
    "webm", "mpg", "mpeg",
];

/// Consecutive unit addresses as `(first, count)` runs, order kept.
fn compress_units(units: &[u64]) -> Vec<(u64, u64)> {
    let mut runs: Vec<(u64, u64)> = Vec::new();
    for &unit in units {
        match runs.last_mut() {
            Some((start, count)) if *start + *count == unit => *count += 1,
            _ => runs.push((unit, 1)),
        }
    }
    runs
}

/// Grades every deleted video entry and carves the unallocated space of
/// the newest inspected image; writes `db/filesystem/recovery-candidates-
/// <unix>.jsonl` (plus the free-unit map and a summary) and one
/// `recovery-scan` event in `evidence/logs/tsk-audit.jsonl`.
pub fn scan_deleted(
    case_dir: &Path,
    progress: Option<&ScanProgress<'_>>,
) -> Result<ScanSummary, String> {
    let target = newest_inspection(case_dir)?;
    let offset_arg = target.offset.to_string();
    let scanned_unix = now_unix()?;
    let mut summary = ScanSummary::default();

    let fsstat = crate::util::tool_output_text(&run_tool(
        "fsstat",
        &["-o".into(), offset_arg.clone(), target.image.clone()],
    )?);
    let mut list = spawn_tool(
        "blkls",
        &[
            "-l".into(),
            "-o".into(),
            offset_arg.clone(),
            target.image.clone(),
        ],
    )?;
    let (unit_name, free) = parse_blkls_list(BufReader::new(
        list.stdout.take().expect("piped blkls stdout"),
    ));
    let listed = list.wait().map_err(|err| format!("blkls -l: {err}"))?;
    if !listed.success() {
        return Err("blkls -l failed to list the unallocated units".to_string());
    }
    let unit = unit_bytes(&fsstat, &unit_name);
    let unit_hint = unit.max(1);
    let free_bytes = free.total() * unit;
    summary.unallocated_bytes = free_bytes;

    let entries = deleted_video_entries(case_dir)?;
    summary.deleted_entries = entries.len();
    // Grows once the allocated-tail region is known.
    let total_work =
        std::sync::atomic::AtomicU64::new(entries.len() as u64 + free_bytes / (1024 * 1024) + 1);
    let done = std::sync::atomic::AtomicU64::new(0);
    let tick = |add: u64| {
        let now = done.fetch_add(add, std::sync::atomic::Ordering::SeqCst) + add;
        let total = total_work.load(std::sync::atomic::Ordering::SeqCst);
        if let Some(report) = progress {
            report(now.min(total), total);
        }
    };

    // Deleted entries: istat + first-unit header, on a small worker pool.
    let probed: std::sync::Mutex<Vec<Probed>> = std::sync::Mutex::new(Vec::new());
    let next = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..PROBE_WORKERS.min(entries.len()).max(1) {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let Some(entry) = entries.get(index) else {
                        break;
                    };
                    let units = run_tool(
                        "istat",
                        &[
                            "-o".into(),
                            offset_arg.clone(),
                            target.image.clone(),
                            entry.inode.clone(),
                        ],
                    )
                    .map(|out| parse_istat_units(&String::from_utf8_lossy(&out)))
                    .unwrap_or_default();
                    // Only the units holding the file's bytes matter.
                    let mut units = units;
                    if let Some(size) = entry.size {
                        units.truncate(size.div_ceil(unit_hint).max(1) as usize);
                    }
                    let head = units.first().and_then(|first| {
                        run_tool(
                            "blkcat",
                            &[
                                "-o".into(),
                                offset_arg.clone(),
                                target.image.clone(),
                                first.to_string(),
                                "1".into(),
                            ],
                        )
                        .ok()
                    });
                    let sniffed = if entry.sniff {
                        head.as_deref().and_then(sniff_video)
                    } else {
                        None
                    };
                    let header = if entry.sniff {
                        sniffed.map(|_| true)
                    } else {
                        head.as_deref()
                            .and_then(|head| header_matches(&extension_of(&entry.path), head))
                    };
                    probed
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .push((index, units, header, sniffed));
                    tick(1);
                }
            });
        }
    });
    let mut probed = probed.into_inner().unwrap_or_else(|e| e.into_inner());
    probed.sort_by_key(|(index, ..)| *index);

    let mut candidates = Vec::new();
    let mut entry_units: Vec<u64> = Vec::new();
    let mut entry_by_first: HashMap<u64, usize> = HashMap::new();
    summary.deleted_entries = 0;
    for (index, units, header, sniffed) in probed {
        let entry = &entries[index];
        // An extensionless deleted file whose head is no video container is
        // not a video candidate.
        if entry.sniff && sniffed.is_none() {
            continue;
        }
        summary.deleted_entries += 1;
        // TSK recovers a deleted FAT file as one run from its first cluster
        // but SKIPS clusters a live file holds now: a jump in the unit list
        // is data of the original file that is gone, and the run continues
        // past it into unrelated free space.
        let skipped = skipped_units(&units);
        let overwritten =
            units.iter().filter(|unit| !free.contains(**unit)).count() as u64 + skipped;
        let total = units.len() as u64 + skipped;
        let grade = grade(total, overwritten);
        match grade {
            "intact" => summary.intact += 1,
            "partial" => summary.partial += 1,
            "overwritten" => summary.overwritten += 1,
            _ => summary.unknown += 1,
        }
        if let Some(first) = units.first() {
            entry_by_first.insert(*first, candidates.len());
        }
        entry_units.extend(&units);
        let mut notes = Vec::new();
        match grade {
            "intact" => notes.push("삭제 엔트리의 데이터 영역을 현재 다른 파일이 쓰고 있지 않습니다 (덮어쓰기 흔적 없음)".to_string()),
            "partial" if skipped > 0 => notes.push(format!(
                "데이터 영역 {overwritten}/{total} 단위를 현재 다른 파일이 쓰고 있어 건너뛰었습니다 — 그 뒤로 읽히는 부분은 원래 영상이 아닐 수 있습니다"
            )),
            "partial" => notes.push(format!(
                "데이터 영역 {overwritten}/{total} 단위를 현재 다른 파일이 쓰고 있습니다 — 그 부분은 다른 내용입니다"
            )),
            "overwritten" => notes.push("데이터 영역 전체를 현재 다른 파일이 쓰고 있습니다 — 원래 내용은 남아 있지 않을 가능성이 큽니다".to_string()),
            _ => notes.push("데이터 위치를 확인하지 못했습니다".to_string()),
        }
        if header == Some(false) {
            notes.push("첫 단위가 확장자의 컨테이너 시그니처와 맞지 않습니다".to_string());
        }
        // An extensionless entry is read out under its sniffed container's
        // extension so it plays as what it is.
        let (extension, preview_path) = match sniffed {
            Some(ext) => {
                notes.push(format!(
                    "확장자가 없지만 첫 부분이 {} 영상 구조입니다 (녹화 중 임시 파일일 수 있음)",
                    ext.to_uppercase()
                ));
                (
                    ext.to_string(),
                    format!("{}.{ext}", entry_preview_path(&entry.inode, &entry.path)),
                )
            }
            None => (
                extension_of(&entry.path),
                entry_preview_path(&entry.inode, &entry.path),
            ),
        };
        candidates.push(RecoveryCandidate {
            kind: ROW_KIND.to_string(),
            id: format!("fls:{}", entry.inode),
            source: "deleted-entry".to_string(),
            inode: Some(entry.inode.clone()),
            card_path: Some(entry.path.clone()),
            name: entry.path.trim_end_matches("(deleted)").trim().to_string(),
            extension,
            signature: None,
            size_bytes: entry.size.unwrap_or(total * unit),
            size_exact: entry.size.is_some(),
            image_offset: units
                .first()
                .map_or(0, |first| target.offset * 512 + first * unit),
            stream_offset: None,
            total_units: total,
            overwritten_units: overwritten,
            grade: grade.to_string(),
            header_ok: header,
            preview_path,
            note: notes.join(" · "),
            image_path: target.image.clone(),
            partition_offset: target.offset,
            unit_bytes: unit,
            stream: default_stream(),
            video_frames: None,
        });
    }
    entry_units.sort_unstable();
    let entry_set = UnitRuns::from_sorted(entry_units);

    // Healthy videos of this card are the reference: headerless H.264/H.265
    // frames are recognised by their codec configuration (and AVI frames
    // later rebuilt with their header).
    let avcc_config = crate::video_repair::reference_config(case_dir).map(|(config, _)| config);
    let probe = || {
        avcc_config
            .as_ref()
            .map(crate::video_repair::AvccProbe::new)
    };
    let scanned_mib = std::cell::Cell::new(0u64);
    let on_bytes = |bytes: u64| {
        let mib = bytes / (1024 * 1024);
        let before = scanned_mib.get();
        if mib > before {
            scanned_mib.set(mib);
            tick(mib - before);
        }
    };

    // Region 1 — unallocated space: every free unit through the signature
    // scan and the frame-run scan in one read.
    let mut stream = spawn_tool(
        "blkls",
        &["-o".into(), offset_arg.clone(), target.image.clone()],
    )?;
    let region = scan_region(
        stream.stdout.take().expect("piped blkls stdout"),
        probe(),
        &on_bytes,
    );
    let _ = stream.kill();
    let _ = stream.wait();
    let region = region?;
    let stream_bytes = region.stream_bytes;
    if region.hit_cap {
        summary.warnings.push(format!(
            "빈 영역에서 시그니처가 {MAX_HITS}개를 넘어 시그니처 스캔을 멈췄습니다 — 이후 영역은 프레임 구간 스캔만 했습니다"
        ));
    }
    let mapping_ok = stream_bytes == free_bytes;
    if !mapping_ok {
        summary.warnings.push(format!(
            "빈 영역 스트림 크기({stream_bytes} B)가 빈 단위 목록({free_bytes} B)과 달라 위치를 확정할 수 없습니다 — 빈 영역 카빙 후보를 만들지 않았습니다"
        ));
    } else {
        let unallocated = Region {
            stream: STREAM_UNALLOCATED,
            runs: &free,
            owners: &[],
            preview_dir: PREVIEW_UNALLOCATED,
            prefix: "u",
            label: "빈 영역",
        };
        add_region_candidates(
            &target,
            &offset_arg,
            unit,
            &unallocated,
            &region,
            &|address| entry_set.contains(address),
            Some(&entry_by_first),
            &mut candidates,
            &mut summary,
        );
    }

    // Region 2 — allocated files past their current content: a format-free
    // recorder overwrites a preallocated file from its start and leaves the
    // older recording behind it; reserved recording files hold whole ones.
    let tails = allocated_tails(case_dir, &target, &offset_arg, unit, &tick);
    summary.tail_files = tails.files;
    summary.tail_bytes = tails.runs.total() * unit;
    total_work.fetch_add(
        summary.tail_bytes / (1024 * 1024),
        std::sync::atomic::Ordering::SeqCst,
    );
    let mut tail_stream_bytes = 0;
    if !tails.runs.runs.is_empty() {
        scanned_mib.set(0);
        let reader = RunsReader {
            image: &target.image,
            offset_arg: &offset_arg,
            runs: tails.runs.runs.clone().into_iter(),
            child: None,
        };
        let tail_scan = scan_region(reader, probe(), &on_bytes)?;
        tail_stream_bytes = tail_scan.stream_bytes;
        let before = candidates.len();
        let region = Region {
            stream: STREAM_TAILS,
            runs: &tails.runs,
            owners: &tails.owners,
            preview_dir: PREVIEW_TAILS,
            prefix: "t",
            label: "할당 파일 내부(현재 녹화 뒤)",
        };
        add_region_candidates(
            &target,
            &offset_arg,
            unit,
            &region,
            &tail_scan,
            &|_| false,
            None,
            &mut candidates,
            &mut summary,
        );
        summary.tail_candidates = candidates.len() - before;
    }

    let dir = case_dir.join("db/filesystem");
    let map_path = dir.join(format!("unalloc-map-{scanned_unix}.json"));
    let map = UnallocMap {
        image_path: target.image.clone(),
        partition_offset: target.offset,
        unit_bytes: unit,
        stream_bytes,
        runs: free,
    };
    write_text(
        &map_path,
        &serde_json::to_string(&map).map_err(|err| err.to_string())?,
    )
    .map_err(|err| format!("failed to write {}: {err}", map_path.display()))?;
    let tail_map_path = dir.join(format!("tail-map-{scanned_unix}.json"));
    let tail_map = UnallocMap {
        image_path: target.image.clone(),
        partition_offset: target.offset,
        unit_bytes: unit,
        stream_bytes: tail_stream_bytes,
        runs: tails.runs,
    };
    write_text(
        &tail_map_path,
        &serde_json::to_string(&tail_map).map_err(|err| err.to_string())?,
    )
    .map_err(|err| format!("failed to write {}: {err}", tail_map_path.display()))?;
    let candidates_path = dir.join(format!("recovery-candidates-{scanned_unix}.jsonl"));
    let mut jsonl = String::new();
    for candidate in &candidates {
        jsonl.push_str(&serde_json::to_string(candidate).map_err(|err| err.to_string())?);
        jsonl.push('\n');
    }
    write_text(&candidates_path, &jsonl)
        .map_err(|err| format!("failed to write {}: {err}", candidates_path.display()))?;
    summary.candidates_path = candidates_path.clone();

    crate::tsk::append_tsk_audit(
        case_dir,
        &format!(
            "{{\"schema_version\":1,\"event\":\"recovery-scan\",\"scanned_unix\":{scanned_unix},\"image_path\":\"{}\",\"partition_offset\":{},\"unit_bytes\":{unit},\"unallocated_bytes\":{free_bytes},\"stream_bytes\":{stream_bytes},\"deleted_entries\":{},\"intact\":{},\"partial\":{},\"overwritten\":{},\"unknown\":{},\"carved_candidates\":{},\"frame_runs\":{},\"tail_files\":{},\"tail_bytes\":{},\"tail_candidates\":{},\"candidates_path\":\"{}\",\"unalloc_map_path\":\"{}\",\"tail_map_path\":\"{}\",\"blkls_version\":\"{}\",\"warnings\":{}}}",
            json_escape(&target.image),
            target.offset,
            summary.deleted_entries,
            summary.intact,
            summary.partial,
            summary.overwritten,
            summary.unknown,
            summary.carved,
            summary.frame_runs,
            summary.tail_files,
            summary.tail_bytes,
            summary.tail_candidates,
            json_escape(&candidates_path.to_string_lossy()),
            json_escape(&map_path.to_string_lossy()),
            json_escape(&tail_map_path.to_string_lossy()),
            json_escape(&crate::tsk::tsk_command_version("blkls", &["blkls"])),
            crate::audit::json_string_array(&summary.warnings),
        ),
    )?;
    Ok(summary)
}

fn newest_named(case_dir: &Path, prefix: &str, suffix: &str) -> Option<PathBuf> {
    fs::read_dir(case_dir.join("db/filesystem"))
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(prefix) && name.ends_with(suffix))
        })
        .max()
}

/// The newest scan's candidate rows as JSONL text ("" when none) — appended
/// to the `flsEntries` the viewer receives.
pub fn latest_candidates_jsonl(case_dir: &Path) -> String {
    newest_named(case_dir, "recovery-candidates-", ".jsonl")
        .and_then(|path| fs::read_to_string(path).ok())
        .unwrap_or_default()
}

fn latest_candidates(case_dir: &Path) -> Vec<RecoveryCandidate> {
    latest_candidates_jsonl(case_dir)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// Makes `path` exist when it is the preview path of a candidate of the
/// newest scan, reading that one candidate out of the image. `Ok(false)`
/// when `path` is no such candidate.
pub fn ensure_candidate(case_dir: &Path, path: &Path) -> Result<bool, String> {
    if path.is_file() {
        return Ok(true);
    }
    let wanted = path_key(&crate::util::strip_windows_extended_prefix(path));
    let roots: Vec<PathBuf> = [
        Some(case_dir.to_path_buf()),
        crate::util::canonicalize_display(case_dir).ok(),
    ]
    .into_iter()
    .flatten()
    .collect();
    let recovery_root = |root: &Path| path_key(&root.join("artifacts").join("Recovery"));
    if !roots
        .iter()
        .any(|root| wanted.starts_with(&recovery_root(root)))
    {
        return Ok(false);
    }
    let candidates = latest_candidates(case_dir);
    let Some((target, candidate)) = candidates.iter().find_map(|candidate| {
        roots
            .iter()
            .map(|root| root.join(&candidate.preview_path))
            .find(|full| path_key(full) == wanted)
            .map(|full| (full, candidate))
    }) else {
        return Ok(false);
    };
    let lock = crate::lazy_extract::lock_target(case_dir, &wanted)?;
    if target.is_file() {
        return Ok(true);
    }
    let read = match candidate.source.as_str() {
        "deleted-entry" => read_deleted_entry(case_dir, candidate, &target),
        _ => read_unallocated(case_dir, candidate, &target),
    };
    drop(lock);
    match read {
        Ok(()) => {
            validate_in_background(case_dir, &target);
            Ok(true)
        }
        Err(_) if target.is_file() => Ok(true),
        Err(err) => Err(format!("{}: {err}", candidate.name)),
    }
}

fn read_deleted_entry(
    case_dir: &Path,
    candidate: &RecoveryCandidate,
    target: &Path,
) -> Result<(), String> {
    let context = crate::tsk::RecoverContext::new(Path::new(&candidate.image_path), "icat")?;
    let options = crate::tsk::TskRecoverOptions {
        partition_offset: candidate.partition_offset,
        inode: candidate.inode.clone().unwrap_or_default(),
        output_path: Some(target.to_path_buf()),
        recover_deleted: true,
        include_slack: false,
        skip_sparse_holes: false,
        icat_bin: "icat".to_string(),
        timeout_secs: None,
    };
    crate::tsk::recover_inode_in(case_dir, &context, &options, true).map(|_| ())
}

/// Writes at most `limit` bytes after skipping `skip` (an unallocated
/// candidate starts inside its first unit and ends at its span).
struct Window<W> {
    inner: W,
    skip: u64,
    limit: u64,
}

impl<W: Write> Window<W> {
    fn feed(&mut self, mut bytes: &[u8]) -> std::io::Result<()> {
        if self.skip > 0 {
            let drop = (self.skip as usize).min(bytes.len());
            bytes = &bytes[drop..];
            self.skip -= drop as u64;
        }
        let take = (self.limit as usize).min(bytes.len());
        self.inner.write_all(&bytes[..take])?;
        self.limit -= take as u64;
        Ok(())
    }
}

/// Streams `count` units from `address` (blkcat) into `sink`.
fn blkcat_into(
    map: &UnallocMap,
    address: u64,
    count: u64,
    sink: &mut dyn FnMut(&[u8]) -> std::io::Result<()>,
) -> Result<(), String> {
    let mut child = spawn_tool(
        "blkcat",
        &[
            "-o".into(),
            map.partition_offset.to_string(),
            map.image_path.clone(),
            address.to_string(),
            count.to_string(),
        ],
    )?;
    let mut stdout = child.stdout.take().expect("piped blkcat stdout");
    let mut buffer = vec![0u8; 1024 * 1024];
    let copied = (|| -> Result<(), String> {
        loop {
            let read = stdout
                .read(&mut buffer)
                .map_err(|err| format!("blkcat read failed: {err}"))?;
            if read == 0 {
                return Ok(());
            }
            sink(&buffer[..read]).map_err(|err| format!("write failed: {err}"))?;
        }
    })();
    drop(stdout);
    let status = child.wait().map_err(|err| format!("blkcat: {err}"))?;
    copied?;
    if !status.success() {
        return Err(format!("blkcat failed at unit {address}"));
    }
    Ok(())
}

/// An AVI candidate read in 16 MiB steps until its chunk chain breaks:
/// `(bytes read, where the intact chain ends)`.
fn read_avi_chain(
    map: &UnallocMap,
    runs: &[(u64, u64)],
    intra: u64,
    limit: u64,
) -> Result<(Vec<u8>, usize), String> {
    let step = (16 * 1024 * 1024 / map.unit_bytes).max(1);
    let mut window = Window {
        inner: Vec::new(),
        skip: intra,
        limit,
    };
    for &(address, count) in runs {
        let mut offset = 0;
        while offset < count {
            let take = (count - offset).min(step);
            blkcat_into(map, address + offset, take, &mut |bytes| window.feed(bytes))?;
            offset += take;
            if let Some(end) = crate::video_repair::avi_data_end(&window.inner, false) {
                return Ok((window.inner, end));
            }
            if window.limit == 0 {
                break;
            }
        }
    }
    let end = crate::video_repair::avi_data_end(&window.inner, true).unwrap_or(window.inner.len());
    Ok((window.inner, end))
}

fn read_unallocated(
    case_dir: &Path,
    candidate: &RecoveryCandidate,
    target: &Path,
) -> Result<(), String> {
    // A headerless frame run: its bytes are read out as they are (the
    // evidence, `.raw`), then rebuilt into a playable file with a healthy
    // video's header at `target`.
    let frame_kind = match candidate.signature.as_deref() {
        Some("avi-frames") => Some(crate::frame_carve::RunKind::AviChunks),
        Some("h264-frames") => Some(crate::frame_carve::RunKind::AvccNals),
        _ => None,
    };
    if let Some(kind) = frame_kind {
        let raw = target.with_extension("raw");
        if !raw.is_file() {
            let as_read = RecoveryCandidate {
                signature: Some("frame-run".to_string()),
                ..candidate.clone()
            };
            read_unallocated(case_dir, &as_read, &raw)?;
        }
        return crate::video_repair::rebuild_frames(case_dir, &raw, kind, target).map(|_| ());
    }
    let map_path = newest_named(case_dir, map_prefix(&candidate.stream), ".json")
        .ok_or_else(|| format!("the {} map of the scan is missing", candidate.stream))?;
    let map: UnallocMap = serde_json::from_str(
        &fs::read_to_string(&map_path)
            .map_err(|err| format!("failed to read {}: {err}", map_path.display()))?,
    )
    .map_err(|err| format!("invalid {}: {err}", map_path.display()))?;
    let stream_offset = candidate
        .stream_offset
        .ok_or_else(|| "candidate has no stream offset".to_string())?;
    let unit = map.unit_bytes;
    let intra = stream_offset % unit;
    let units = (intra + candidate.size_bytes).div_ceil(unit);
    let runs = map.runs.physical(stream_offset / unit, units);

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let part = crate::join::pid_part_path(target, std::process::id());
    crate::join::remove_file_retry(&part)?;
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&part)
        .map_err(|err| format!("failed to create {}: {err}", part.display()))?;
    let signature = candidate.signature.clone().unwrap_or_default();
    let copied = if signature == "riff-avi" {
        // A cut-off AVI declares no usable size: read 16 MiB at a time
        // and stop where its chunk chain breaks, not at the span's end.
        read_avi_chain(&map, &runs, intra, candidate.size_bytes).and_then(|(bytes, end)| {
            let mut file = file;
            file.write_all(&bytes[..end])
                .map_err(|err| format!("write failed: {err}"))?;
            Ok(Some((
                bytes.len() as u64,
                end as u64,
                format!(
                    "AVI chunk chain followed: the first chunk that does not parse ends the data at {end} bytes (RIFF/movi size fields ignored)"
                ),
            )))
        })
    } else {
        let mut window = Window {
            inner: std::io::BufWriter::with_capacity(1024 * 1024, file),
            skip: intra,
            limit: candidate.size_bytes,
        };
        let streamed = runs
            .iter()
            .try_for_each(|&(address, count)| {
                blkcat_into(&map, address, count, &mut |bytes| window.feed(bytes))
            })
            .and_then(|()| {
                window
                    .inner
                    .flush()
                    .map_err(|err| format!("write failed: {err}"))
            });
        drop(window);
        streamed.map(|()| None)
    };
    let (available, extent, boundary_note) = match copied {
        Err(err) => {
            let _ = fs::remove_file(&part);
            return Err(err);
        }
        Ok(Some(avi)) => avi,
        Ok(None) => {
            let available = fs::metadata(&part).map(|meta| meta.len()).unwrap_or(0);
            let (extent, note) = crate::carve::refine_extent_in_file(
                &part,
                &signature,
                &candidate.extension,
                available,
            );
            (available, extent.min(available), note)
        }
    };
    fs::OpenOptions::new()
        .write(true)
        .open(&part)
        .and_then(|file| file.set_len(extent))
        .map_err(|err| format!("failed to trim {}: {err}", part.display()))?;
    let sha256 = crate::audit::digest_file(&part)?;
    match crate::join::publish_part_as(&part, target) {
        Ok(true) => {}
        Ok(false) => {
            let _ = fs::remove_file(&part);
            return Ok(());
        }
        Err(err) => {
            let _ = fs::remove_file(&part);
            return Err(err);
        }
    }
    let used: Vec<String> = runs
        .iter()
        .map(|(address, count)| format!("[{address},{count}]"))
        .collect();
    crate::tsk::append_tsk_audit(
        case_dir,
        &format!(
            "{{\"schema_version\":1,\"event\":\"recover-unallocated\",\"recovered_unix\":{},\"id\":\"{}\",\"image_path\":\"{}\",\"partition_offset\":{},\"unit_bytes\":{unit},\"stream_offset\":{stream_offset},\"image_offset\":{},\"unit_runs\":[{}],\"read_bytes\":{available},\"size_bytes\":{extent},\"signature\":\"{}\",\"boundary_note\":\"{}\",\"output_path\":\"{}\",\"sha256\":\"{sha256}\",\"validation_status\":\"candidate-unvalidated\",\"blkcat_version\":\"{}\"}}",
            now_unix()?,
            json_escape(&candidate.id),
            json_escape(&map.image_path),
            map.partition_offset,
            candidate.image_offset,
            used.join(","),
            json_escape(&signature),
            json_escape(&boundary_note),
            json_escape(&target.to_string_lossy()),
            json_escape(&crate::tsk::tsk_command_version("blkcat", &["blkcat"])),
        ),
    )
}

/// ffprobe check of a candidate just read out, off the caller's thread: the
/// result lands in `evidence/logs/validation-log.jsonl` (the viewer's
/// status badge) without delaying playback.
fn validate_in_background(case_dir: &Path, target: &Path) {
    let case_dir = case_dir.to_path_buf();
    let target = target.to_string_lossy().to_string();
    let _ = std::thread::Builder::new()
        .name("ft-validate-candidate".into())
        .spawn(move || {
            if let Err(err) = crate::validation::validate_artifact(
                &case_dir,
                &target,
                &crate::validation::ValidationOptions::default(),
            ) {
                eprintln!("warning: validation of {target} failed: {err}");
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_runs_merge_and_map_stream_ranges() {
        let runs = UnitRuns::from_sorted([10, 11, 12, 20, 21, 30]);
        assert_eq!(runs.runs, [(10, 3), (20, 2), (30, 1)]);
        assert_eq!(runs.total(), 6);
        assert!(runs.contains(11) && runs.contains(21) && runs.contains(30));
        assert!(!runs.contains(13) && !runs.contains(19) && !runs.contains(31));
        // Stream units 2..5 are image units 12, 20, 21, 30.
        assert_eq!(runs.physical(2, 4), [(12, 1), (20, 2), (30, 1)]);
        assert_eq!(runs.physical(0, 1), [(10, 1)]);
        assert!(runs.physical(6, 1).is_empty());
    }

    #[test]
    fn parses_blkls_list_and_istat_sectors() {
        let list = "class|host|image|first_time|unit\nblkls|unknown||1793000274|Sector\naddr|alloc\n368|f\n369|f\n370|f\n409|f\n";
        let (unit, runs) = parse_blkls_list(std::io::Cursor::new(list));
        assert_eq!(unit, "Sector");
        assert_eq!(runs.runs, [(368, 3), (409, 1)]);
        let istat = "Directory Entry: 525\nNot Allocated\nSize: 3016\nName: _T0003~1.AVI\n\nSectors:\n368 369 370 371\n372 373 0 0\n";
        assert_eq!(parse_istat_units(istat), [368, 369, 370, 371, 372, 373]);
        let fsstat = "Sector Size: 512\nCluster Size: 32768\n";
        assert_eq!(unit_bytes(fsstat, "Sector"), 512);
        assert_eq!(unit_bytes(fsstat, "Cluster"), 32768);
    }

    #[test]
    fn grades_and_headers() {
        assert_eq!(grade(0, 0), "unknown");
        assert_eq!(grade(6, 0), "intact");
        assert_eq!(grade(6, 2), "partial");
        assert_eq!(grade(6, 6), "overwritten");
        assert_eq!(skipped_units(&[10, 11, 12, 13]), 0);
        // Clusters 14..=19 belong to a live file now: TSK jumped over them.
        assert_eq!(skipped_units(&[10, 11, 12, 13, 20, 21]), 6);
        assert_eq!(header_matches("avi", b"RIFF\0\0\0\0AVI LIST"), Some(true));
        assert_eq!(header_matches("mp4", b"\0\0\0\x18ftypisom"), Some(true));
        assert_eq!(header_matches("mp4", b"\0\0\0\x08mdat...."), Some(true));
        assert_eq!(header_matches("mp4", b"garbage-garbage"), Some(false));
        assert_eq!(header_matches("txt", b"x"), None);
        assert_eq!(
            entry_preview_path("525", "EVENT/20260920_100000_EVT_1.avi"),
            "artifacts/Recovery/Filesystem/525_20260920_100000_EVT_1.avi"
        );
        assert_eq!(
            entry_preview_path("77", "상시/정상.avi (deleted)"),
            "artifacts/Recovery/Filesystem/77_정상.avi"
        );
    }

    #[test]
    fn probes_container_extents_from_headers() {
        // ftyp(24) + free(8) + mdat(1000) + moov(100), then junk.
        let mut mp4 = Vec::new();
        mp4.extend_from_slice(&24u32.to_be_bytes());
        mp4.extend_from_slice(b"ftypisom");
        mp4.extend_from_slice(&[0; 12]);
        mp4.extend_from_slice(&8u32.to_be_bytes());
        mp4.extend_from_slice(b"free");
        mp4.extend_from_slice(&1000u32.to_be_bytes());
        mp4.extend_from_slice(b"mdat");
        mp4.extend(std::iter::repeat_n(7u8, 992));
        mp4.extend_from_slice(&100u32.to_be_bytes());
        mp4.extend_from_slice(b"moov");
        mp4.extend(std::iter::repeat_n(1u8, 92));
        mp4.extend_from_slice(&[0xFF; 64]);
        let read = |offset: u64, len: u64| {
            let start = offset as usize;
            (start <= mp4.len()).then(|| mp4[start..(start + len as usize).min(mp4.len())].to_vec())
        };
        assert_eq!(probe_extent(&read, 0, "mp4-ftyp", 10_000), Some(1132));
        // A box claiming more than the span is cut at the span.
        assert_eq!(probe_extent(&read, 0, "mp4-ftyp", 500), Some(500));
        let riff = [b"RIFF".as_slice(), &4000u32.to_le_bytes(), b"AVI "].concat();
        let read_riff = |_offset: u64, _len: u64| Some(riff.clone());
        assert_eq!(probe_extent(&read_riff, 0, "riff-avi", 1 << 20), Some(4008));
        assert_eq!(probe_extent(&read_riff, 0, "mpegts-sync", 1 << 20), None);
    }

    #[test]
    fn window_skips_and_limits() {
        let mut window = Window {
            inner: Vec::new(),
            skip: 3,
            limit: 5,
        };
        window.feed(b"ab").unwrap();
        window.feed(b"cdefghij").unwrap();
        assert_eq!(window.inner, b"defgh");
    }
}
