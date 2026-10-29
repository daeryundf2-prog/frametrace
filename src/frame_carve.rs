//! Frame-run carving: video frames whose container header is gone.
//!
//! Signature carving only finds clips whose first bytes survived (an MP4
//! `ftyp`, an AVI `RIFF`). On a dashcam card most deleted footage is not
//! like that: the head cluster was reused, the clip was fragmented across
//! channels, or a format-free recorder overwrote a preallocated file from
//! its start and left an older recording behind the new one. The frames
//! themselves are still there, and both containers dashcams use store
//! them as a chain of self-sized units:
//!
//! * AVI `movi` chunks — `00dc`/`01wb`… + little-endian size, word aligned;
//! * MP4 `mdat` samples — length-prefixed H.264/H.265 NAL units, checked
//!   against a healthy MP4 of the same card (slice header, PPS id).
//!
//! [`FrameScanner`] follows such chains through a byte stream in one pass
//! (fed alongside the signature scan) and reports each run long enough to
//! be footage. A run is rebuilt into a playable file with a reference
//! header ([`crate::video_repair`]).

use crate::video_repair::AvccProbe;

/// Video frames a run needs before it is reported (~1 s at 30 fps).
pub const MIN_VIDEO_FRAMES: u64 = 30;
/// Largest AVI chunk accepted (a 4K I-frame stays well below).
const MAX_AVI_CHUNK: u64 = 8 * 1024 * 1024;

/// What a run is made of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunKind {
    AviChunks,
    AvccNals,
}

impl RunKind {
    pub fn signature(self) -> &'static str {
        match self {
            RunKind::AviChunks => "avi-frames",
            RunKind::AvccNals => "h264-frames",
        }
    }
}

/// A chain of frames found in the stream: `[start, end)` stream offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameRun {
    pub kind: RunKind,
    pub start: u64,
    pub end: u64,
    pub units: u64,
    pub video_frames: u64,
}

/// One unit check: `(unit length, counts as a video frame)`.
trait UnitCheck {
    /// Bytes needed from a unit's first byte to judge it.
    fn peek(&self) -> usize;
    /// Positions tried while searching advance by this much.
    fn step(&self) -> u64;
    fn kind(&self) -> RunKind;
    fn unit(&self, head: &[u8], starting: bool) -> Option<(u64, bool)>;
    /// Foreign bytes a chain may skip and continue after (0: none).
    fn max_gap(&self) -> u64 {
        0
    }
    /// The picture number a video unit carries, when the format has one.
    fn order(&self, _head: &[u8]) -> Option<u32> {
        None
    }
    /// Whether a picture start found across a gap continues the numbering
    /// after `last`.
    fn continues(&self, _last: Option<u32>, _head: &[u8]) -> bool {
        true
    }
}

struct AviCheck;

impl UnitCheck for AviCheck {
    fn peek(&self) -> usize {
        8
    }
    fn step(&self) -> u64 {
        // Chunks are word aligned within their file, but a scan region can
        // start anywhere (a file's tail), so every offset is tried.
        1
    }
    fn kind(&self) -> RunKind {
        RunKind::AviChunks
    }
    fn unit(&self, head: &[u8], starting: bool) -> Option<(u64, bool)> {
        let id = head.get(..4)?;
        let size = u32::from_le_bytes(head.get(4..8)?.try_into().ok()?) as u64;
        let stream = id[0].is_ascii_digit() && id[1].is_ascii_digit();
        let video = stream && matches!(&id[2..4], b"dc" | b"db");
        let audio = stream && &id[2..4] == b"wb";
        // Index and padding chunks continue a chain but never start one.
        let filler = !starting && ((&id[..2] == b"ix" && id[2].is_ascii_digit()) || id == b"JUNK");
        if !(video || audio || filler) || size == 0 || size > MAX_AVI_CHUNK {
            return None;
        }
        Some((8 + size + (size & 1), video))
    }
}

struct AvccCheck(AvccProbe);

impl UnitCheck for AvccCheck {
    fn peek(&self) -> usize {
        AvccProbe::PEEK
    }
    fn step(&self) -> u64 {
        1
    }
    fn kind(&self) -> RunKind {
        RunKind::AvccNals
    }
    fn unit(&self, head: &[u8], starting: bool) -> Option<(u64, bool)> {
        self.0.unit(head, starting)
    }
    /// MP4 interleaves each video sample (or second) with an audio chunk:
    /// the chain picks up at the next picture start after it.
    fn max_gap(&self) -> u64 {
        512 * 1024
    }
    fn order(&self, head: &[u8]) -> Option<u32> {
        self.0.order(head)
    }
    fn continues(&self, last: Option<u32>, head: &[u8]) -> bool {
        self.0.continues(last, head)
    }
}

enum Mode {
    /// Testing positions from here for a chain start.
    Search(u64),
    /// Inside a chain; the next unit header is here.
    Follow { next: u64, run: FrameRun },
    /// The chain broke at `from`; looking for the next picture start at
    /// `pos` (up to [`UnitCheck::max_gap`] past `from`).
    Gap { from: u64, pos: u64, run: FrameRun },
}

struct Tracker {
    check: Box<dyn UnitCheck + Send>,
    mode: Mode,
    /// Largest video unit of the current run: a picture start found
    /// across a gap may not be much larger (a false match in audio bytes
    /// reads a random length and would swallow the frames after it).
    largest: u64,
    /// Picture number of the run's last video unit.
    last_order: Option<u32>,
}

/// Floor of the size a picture start across a gap may have.
const GAP_UNIT_FLOOR: u64 = 512 * 1024;

/// Where the search goes on after `run` closed at `at`: a run too short
/// to be footage may have started on a false match, so its own bytes are
/// searched again.
fn search_from(run: &FrameRun, at: u64) -> u64 {
    if run.video_frames >= MIN_VIDEO_FRAMES {
        at
    } else {
        run.start + 1
    }
}

/// Follows AVI-chunk and (with a reference) AVCC-NAL chains through a byte
/// stream fed in order.
pub struct FrameScanner {
    trackers: Vec<Tracker>,
    /// Buffered stream bytes; `buffer[0]` is stream offset `base`.
    buffer: Vec<u8>,
    base: u64,
    runs: Vec<FrameRun>,
}

impl FrameScanner {
    /// AVI chunks always; AVCC NAL units when `avcc` (from a healthy MP4 of
    /// the same card) is given.
    pub(crate) fn new(avcc: Option<AvccProbe>) -> Self {
        let mut trackers = vec![Tracker {
            check: Box::new(AviCheck),
            mode: Mode::Search(0),
            largest: 0,
            last_order: None,
        }];
        if let Some(probe) = avcc {
            trackers.push(Tracker {
                check: Box::new(AvccCheck(probe)),
                mode: Mode::Search(0),
                largest: 0,
                last_order: None,
            });
        }
        Self {
            trackers,
            buffer: Vec::new(),
            base: 0,
            runs: Vec::new(),
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
        self.advance(false);
        self.trim();
    }

    /// Ends the stream; returns every run long enough to be footage.
    pub fn finish(mut self) -> Vec<FrameRun> {
        self.advance(true);
        let mut runs = self.runs;
        runs.sort_by_key(|run| run.start);
        runs
    }

    fn end(&self) -> u64 {
        self.base + self.buffer.len() as u64
    }

    fn peek(&self, at: u64, len: usize) -> &[u8] {
        if at < self.base || at + len as u64 > self.end() {
            return &[];
        }
        let from = (at - self.base) as usize;
        &self.buffer[from..from + len]
    }

    fn advance(&mut self, eof: bool) {
        let end = self.end();
        for index in 0..self.trackers.len() {
            loop {
                let check = &self.trackers[index].check;
                let peek = check.peek();
                match &self.trackers[index].mode {
                    Mode::Follow { next, run } => {
                        let next = *next;
                        if next + peek as u64 > end {
                            if eof {
                                let mut run = run.clone();
                                run.end = run.end.min(end);
                                self.close(run);
                                self.trackers[index].mode = Mode::Search(end);
                            }
                            break;
                        }
                        let head = self.peek(next, peek);
                        match check.unit(head, false) {
                            Some((len, video)) => {
                                let order = if video { check.order(head) } else { None };
                                let tracker = &mut self.trackers[index];
                                if video {
                                    tracker.largest = tracker.largest.max(len);
                                    tracker.last_order = order.or(tracker.last_order);
                                }
                                if let Mode::Follow { next, run } = &mut tracker.mode {
                                    run.units += 1;
                                    run.video_frames += u64::from(video);
                                    run.end = *next + len;
                                    *next += len;
                                }
                            }
                            None if check.max_gap() > 0 => {
                                let run = run.clone();
                                self.trackers[index].mode = Mode::Gap {
                                    from: next,
                                    pos: next + 1,
                                    run,
                                };
                            }
                            None => {
                                let run = run.clone();
                                self.trackers[index].mode = Mode::Search(search_from(&run, next));
                                self.close(run);
                            }
                        }
                    }
                    Mode::Gap { from, pos, run } => {
                        let (from, run) = (*from, run.clone());
                        let limit = from + check.max_gap();
                        let cap = (self.trackers[index].largest * 4).max(GAP_UNIT_FLOOR);
                        let last = self.trackers[index].last_order;
                        let mut pos = *pos;
                        let mut resumed = None;
                        while pos <= limit && pos + peek as u64 <= end {
                            // Only a strictly checked picture start that
                            // continues the numbering resumes the chain.
                            let head = self.peek(pos, peek);
                            if let Some((len, true)) = check.unit(head, true)
                                && len <= cap
                                && check.continues(last, head)
                            {
                                resumed = Some(pos);
                                break;
                            }
                            pos += 1;
                        }
                        match resumed {
                            Some(at) => {
                                self.trackers[index].mode = Mode::Follow { next: at, run };
                            }
                            None if pos > limit || eof => {
                                self.trackers[index].mode = Mode::Search(search_from(&run, from));
                                self.close(run);
                            }
                            None => {
                                self.trackers[index].mode = Mode::Gap { from, pos, run };
                                break;
                            }
                        }
                    }
                    Mode::Search(from) => {
                        let mut pos = *from;
                        let step = check.step();
                        let mut started = None;
                        while pos + peek as u64 <= end {
                            if let Some((len, video)) = check.unit(self.peek(pos, peek), true) {
                                let next = pos + len;
                                if check.max_gap() > 0 {
                                    // A chain that skips gaps (MP4 audio after
                                    // every sample) starts on one strictly
                                    // checked picture start; a stray match
                                    // never reaches MIN_VIDEO_FRAMES.
                                    if video {
                                        started = Some((pos, len, video));
                                        break;
                                    }
                                } else if next + peek as u64 > end {
                                    // A start counts once the following unit
                                    // also parses (one stray match is noise);
                                    // until it is buffered the search waits.
                                    if !eof {
                                        break;
                                    }
                                } else if check.unit(self.peek(next, peek), false).is_some() {
                                    started = Some((pos, len, video));
                                    break;
                                }
                            }
                            pos += step;
                        }
                        let kind = check.kind();
                        match started {
                            Some((start, len, video)) => {
                                let order = check.order(self.peek(start, peek));
                                self.trackers[index].largest = if video { len } else { 0 };
                                self.trackers[index].last_order = order;
                                self.trackers[index].mode = Mode::Follow {
                                    next: start + len,
                                    run: FrameRun {
                                        kind,
                                        start,
                                        end: start + len,
                                        units: 1,
                                        video_frames: u64::from(video),
                                    },
                                };
                            }
                            None => {
                                self.trackers[index].mode = Mode::Search(pos);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
    fn close(&mut self, run: FrameRun) {
        if run.video_frames >= MIN_VIDEO_FRAMES {
            self.runs.push(run);
        }
    }

    /// Drops bytes no tracker can look at again.
    fn trim(&mut self) {
        let keep_from = self
            .trackers
            .iter()
            .map(|tracker| match &tracker.mode {
                Mode::Search(pos) => *pos,
                // A run too short to be footage is searched again from its
                // start when it breaks (see `search_from`).
                Mode::Follow { run, .. } | Mode::Gap { run, .. }
                    if run.video_frames < MIN_VIDEO_FRAMES =>
                {
                    run.start
                }
                Mode::Follow { next, .. } => *next,
                // A failed gap restarts the search at `from`.
                Mode::Gap { from, .. } => *from,
            })
            .min()
            .unwrap_or(self.end())
            .min(self.end());
        // Compacted in large steps: a drain moves the whole tail.
        if keep_from > self.base + 8 * 1024 * 1024 {
            let drop = (keep_from - self.base) as usize;
            self.buffer.drain(..drop.min(self.buffer.len()));
            self.base += drop as u64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(id: &[u8; 4], len: usize, fill: u8) -> Vec<u8> {
        let mut out = id.to_vec();
        out.extend_from_slice(&(len as u32).to_le_bytes());
        out.extend(std::iter::repeat_n(fill, len));
        if len % 2 == 1 {
            out.push(0);
        }
        out
    }

    fn avi_frames(frames: usize) -> Vec<u8> {
        let mut out = Vec::new();
        for i in 0..frames {
            out.extend(chunk(b"00dc", 3000 + i * 7, 0x11));
            out.extend(chunk(b"01wb", 1001, 0x22));
        }
        out
    }

    #[test]
    fn finds_headerless_avi_runs_between_foreign_bytes() {
        let mut stream = vec![0xA5u8; 5_001];
        let first = stream.len() as u64;
        stream.extend(avi_frames(40));
        let first_end = stream.len() as u64;
        stream.extend(vec![0x5Au8; 70_000]);
        let second = stream.len() as u64;
        stream.extend(avi_frames(35));
        // Too short to be footage.
        stream.extend(vec![0x33u8; 999]);
        stream.extend(avi_frames(5));
        stream.extend(vec![0x44u8; 1000]);
        let mut scanner = FrameScanner::new(None);
        // Odd-sized feeds: chains cross every feed boundary.
        for piece in stream.chunks(4093) {
            scanner.feed(piece);
        }
        let runs = scanner.finish();
        assert_eq!(runs.len(), 2, "{runs:?}");
        assert_eq!((runs[0].start, runs[0].end), (first, first_end));
        assert_eq!(runs[0].video_frames, 40);
        assert_eq!(runs[0].units, 80);
        assert_eq!(runs[1].start, second);
        assert_eq!(runs[1].video_frames, 35);
    }

    /// AVCC with audio interleaved after every sample (ffmpeg/dashcam MP4).
    #[test]
    fn avcc_runs_continue_across_interleaved_audio() {
        let config_bytes = {
            // Minimal moov-free config: 4-byte lengths, PPS id 0 (SPS/PPS
            // content is not inspected by the probe beyond the PPS id).
            crate::video_repair::test_config()
        };
        let probe = crate::video_repair::AvccProbe::new(&config_bytes);
        let slice = |header: u8, bits: &[u8]| {
            let mut nal = vec![header];
            nal.extend_from_slice(bits);
            nal.extend(std::iter::repeat_n(0x5Au8, 900));
            let mut out = (nal.len() as u32).to_be_bytes().to_vec();
            out.extend(nal);
            out
        };
        let mut stream = vec![0x11u8; 3001];
        let start = stream.len() as u64;
        stream.extend(slice(0x65, &[0x88, 0x80]));
        for _ in 0..40 {
            stream.extend_from_slice(&[0xFF, 0xF1, 0x50, 0x80, 0x10, 0x1F, 0xFC, 0x21, 0x10]);
            stream.extend(slice(0x41, &[0x9A]));
        }
        let end = stream.len() as u64;
        stream.extend(vec![0xEEu8; 600_000]);
        let mut scanner = FrameScanner::new(Some(probe));
        for piece in stream.chunks(65_537) {
            scanner.feed(piece);
        }
        let runs: Vec<FrameRun> = scanner
            .finish()
            .into_iter()
            .filter(|run| run.kind == RunKind::AvccNals)
            .collect();
        assert_eq!(runs.len(), 1, "{runs:?}");
        assert_eq!((runs[0].start, runs[0].end), (start, end));
        assert_eq!(runs[0].video_frames, 41);
    }

    /// Audio bytes that happen to read as a picture start with a huge
    /// length must not swallow the frames after them.
    #[test]
    fn a_false_picture_start_in_audio_does_not_end_the_run() {
        let probe = crate::video_repair::AvccProbe::new(&crate::video_repair::test_config());
        let slice = |header: u8, bits: &[u8]| {
            let mut nal = vec![header];
            nal.extend_from_slice(bits);
            nal.extend(std::iter::repeat_n(0x5Au8, 900));
            let mut out = (nal.len() as u32).to_be_bytes().to_vec();
            out.extend(nal);
            out
        };
        let mut stream = vec![0x11u8; 2001];
        let start = stream.len() as u64;
        stream.extend(slice(0x65, &[0x88, 0x80]));
        for i in 0..40 {
            stream.extend_from_slice(&[0xFF, 0xF1, 0x50, 0x80]);
            if i == 10 {
                // A 3 MB "IDR slice" inside the audio chunk.
                stream.extend_from_slice(&[0x00, 0x30, 0x00, 0x00, 0x65, 0x88, 0x80]);
            }
            stream.extend(slice(0x41, &[0x9A]));
        }
        let end = stream.len() as u64;
        stream.extend(vec![0xEEu8; 600_000]);
        let mut scanner = FrameScanner::new(Some(probe));
        for piece in stream.chunks(65_537) {
            scanner.feed(piece);
        }
        let runs: Vec<FrameRun> = scanner
            .finish()
            .into_iter()
            .filter(|run| run.kind == RunKind::AvccNals)
            .collect();
        assert_eq!(runs.len(), 1, "{runs:?}");
        assert_eq!((runs[0].start, runs[0].end), (start, end));
        assert_eq!(runs[0].video_frames, 41);
    }

    #[test]
    fn a_run_cut_by_the_stream_end_is_kept() {
        let mut stream = avi_frames(50);
        stream.truncate(stream.len() - 500);
        let mut scanner = FrameScanner::new(None);
        scanner.feed(&stream);
        let runs = scanner.finish();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].video_frames, 50);
        assert_eq!(runs[0].end, stream.len() as u64);
    }
}
