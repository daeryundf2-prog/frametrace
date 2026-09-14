//! Local examiner workstation launcher.
//!
//! Running the bare `frametrace.exe` binary starts a 127.0.0.1-only HTTP
//! server that hosts the INPUT wizard, drives the existing CLI pipeline as
//! subprocesses (so audit logging and job tracking stay identical), serves the
//! generated review/report pages, and streams evidence media with Range
//! support so the browser can play it without file:// restrictions.

use crate::audit;
use crate::util::json_escape;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

const EXAMINER_PAGE: &str = include_str!("../assets/examiner_app.html");
const MAX_BODY: usize = 1024 * 1024;

pub struct ServeOptions {
    pub case_dir: Option<PathBuf>,
    pub port: Option<u16>,
}

#[derive(Clone, Copy, PartialEq)]
enum InputKind {
    Folder,
    E01,
}

impl InputKind {
    fn step_names(self) -> &'static [&'static str] {
        match self {
            InputKind::Folder => &[
                "케이스 준비",
                "소스 등록",
                "스캔 · 색인",
                "재생성 검증",
                "리뷰 생성",
            ],
            InputKind::E01 => &[
                "케이스 준비",
                "E01 검증 · 추출",
                "이미지 조사 (mmls/fls)",
                "논리 파일 색인",
                "리뷰 생성",
            ],
        }
    }
}

struct PipelineJob {
    kind: InputKind,
    case_dir: PathBuf,
    source_path: PathBuf,
    with_hash: bool,
    with_ffprobe: bool,
    skip_e01_verify: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum StepStatus {
    Pending,
    Running,
    Done,
    Failed,
}

impl StepStatus {
    fn as_str(self) -> &'static str {
        match self {
            StepStatus::Pending => "pending",
            StepStatus::Running => "running",
            StepStatus::Done => "done",
            StepStatus::Failed => "failed",
        }
    }
}

struct JobState {
    phase: &'static str, // idle | running | review-ready | finalizing | done | error
    steps: [StepStatus; 5],
    step_names: Vec<&'static str>,
    logs: Vec<String>,
    case_dir: Option<PathBuf>,
    media_roots: Vec<PathBuf>,
    package_dir: Option<PathBuf>,
    error: Option<String>,
    busy: bool,
    cancel_requested: bool,
    /// Byte-level progress hint for steps that run inside an external tool
    /// (ewfexport) where no in-process callback exists: the pipeline spawns
    /// a thread that stats the growing output file and stores its size
    /// here. `None` when the DB `jobs` row is the progress source.
    byte_progress: Option<(PathBuf, Option<u64>)>,
}

impl JobState {
    fn new() -> Self {
        Self {
            phase: "idle",
            steps: [StepStatus::Pending; 5],
            step_names: InputKind::Folder.step_names().to_vec(),
            logs: Vec::new(),
            case_dir: None,
            media_roots: Vec::new(),
            package_dir: None,
            error: None,
            busy: false,
            cancel_requested: false,
            byte_progress: None,
        }
    }
}

type SharedState = Arc<Mutex<JobState>>;

fn state_lock(state: &SharedState) -> MutexGuard<'_, JobState> {
    match state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub fn run(options: ServeOptions) -> Result<(), String> {
    let state: SharedState = Arc::new(Mutex::new(JobState::new()));
    if let Some(case_dir) = &options.case_dir {
        state_lock(&state).case_dir = Some(case_dir.clone());
    }
    let listener = match options.port {
        Some(port) => TcpListener::bind(("127.0.0.1", port))
            .map_err(|err| format!("failed to bind 127.0.0.1:{port}: {err}"))?,
        // The probe listener is passed straight through instead of being
        // dropped and re-bound: between "probe says free" and "bind for
        // real" another process could claim the port (classic TOCTOU).
        None => first_free_port().ok_or("사용 가능한 로컬 포트를 찾지 못했습니다")?,
    };
    let port = listener.local_addr().map(|addr| addr.port()).unwrap_or(0);
    let url = format!("http://127.0.0.1:{port}/");
    println!("FrameTrace examiner workstation is running.");
    println!("  {url}");
    println!("Close this window to stop the workstation.");
    let _ = std::io::stdout().flush();
    // Test harnesses set FRAMETRACE_NO_BROWSER=1 so spawning the server
    // never steals focus with a real browser window.
    if std::env::var("FRAMETRACE_NO_BROWSER").as_deref() != Ok("1") {
        open_in_browser(&url);
    }
    serve_on(listener, state);
    Ok(())
}

/// Accept loop shared by `run` and the integration tests (no browser side
/// effects here).
fn serve_on(listener: TcpListener, state: SharedState) {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let state = Arc::clone(&state);
        let _ = thread::Builder::new()
            .name("ft-http".into())
            .spawn(move || {
                let _ = handle_connection(stream, state);
            });
    }
}

fn first_free_port() -> Option<TcpListener> {
    for port in 8477..=8486 {
        if let Ok(listener) = TcpListener::bind(("127.0.0.1", port)) {
            return Some(listener);
        }
    }
    TcpListener::bind(("127.0.0.1", 0)).ok()
}

fn open_in_browser(url: &str) {
    #[cfg(target_os = "windows")]
    let _ = Command::new("explorer.exe").arg(url).spawn();
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("xdg-open").arg(url).spawn();
    }
}

fn open_in_explorer(path: &Path) {
    #[cfg(target_os = "windows")]
    let _ = Command::new("explorer.exe").arg(path).spawn();
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("xdg-open").arg(path).spawn();
    }
}

struct Request {
    method: String,
    path: String,
    query: String,
    body: String,
    range: Option<String>,
    /// Raw Origin header, when present (CSRF gate for state-changing POSTs).
    origin: Option<String>,
    /// Raw Host header, when present (DNS-rebinding gate).
    host: Option<String>,
}

fn handle_connection(mut stream: TcpStream, state: SharedState) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|err| format!("read timeout: {err}"))?;
    let request = read_request(&mut stream)?;
    if !request_is_localhost_trusted(&request) {
        let response = plain(403, b"cross-origin request rejected".to_vec());
        let _ = stream.write_all(&response);
        let _ = stream.flush();
        // Half-close so simple clients see EOF instead of waiting on the
        // read timeout; we never reuse connections.
        let _ = stream.shutdown(std::net::Shutdown::Write);
        return Ok(());
    }
    if request.method == "GET" && request.path == "/media" {
        let result = serve_media(&mut stream, &request, &state);
        let _ = stream.shutdown(std::net::Shutdown::Write);
        return result;
    }
    let response = route(&request, &state);
    stream
        .write_all(&response)
        .map_err(|err| format!("write failed: {err}"))?;
    let _ = stream.flush();
    let _ = stream.shutdown(std::net::Shutdown::Write);
    Ok(())
}

fn read_request(stream: &mut TcpStream) -> Result<Request, String> {
    let mut buffer: Vec<u8> = Vec::with_capacity(2048);
    let mut chunk = [0u8; 2048];
    let header_end = loop {
        let read = stream
            .read(&mut chunk)
            .map_err(|err| format!("read failed: {err}"))?;
        if read == 0 {
            return Err("connection closed before request header ended".into());
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(position) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
            break position;
        }
        if buffer.len() > 64 * 1024 {
            return Err("request header too large".into());
        }
    };
    let head = String::from_utf8_lossy(&buffer[..header_end]).to_string();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_uppercase();
    let target = parts.next().unwrap_or("/").to_string();
    let mut content_length = 0usize;
    let mut range = None;
    let mut origin = None;
    let mut host = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        let value = value.trim();
        if name.eq_ignore_ascii_case("content-length") {
            content_length = value
                .parse::<usize>()
                .map_err(|_| "invalid content-length")?;
        } else if name.eq_ignore_ascii_case("range") {
            range = Some(value.to_string());
        } else if name.eq_ignore_ascii_case("origin") {
            origin = Some(value.to_string());
        } else if name.eq_ignore_ascii_case("host") {
            host = Some(value.to_string());
        }
    }
    if content_length > MAX_BODY {
        return Err("request body too large".into());
    }
    let mut body = buffer[header_end + 4..].to_vec();
    while body.len() < content_length {
        let read = stream
            .read(&mut chunk)
            .map_err(|err| format!("read failed: {err}"))?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(content_length);
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (percent_decode_component(path, false), query.to_string()),
        None => (percent_decode_component(&target, false), String::new()),
    };
    Ok(Request {
        method,
        path,
        query,
        body: String::from_utf8_lossy(&body).to_string(),
        range,
        origin,
        host,
    })
}

/// Percent-decodes a URL component. `plus_as_space` selects query-string
/// semantics; path components must NOT translate `+` (Unix filenames can
/// legitimately contain it), so callers pass false there.
fn percent_decode_component(input: &str, plus_as_space: bool) -> String {
    let bytes = input.as_bytes();
    let mut output: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => match bytes.get(index + 1..index + 3) {
                Some(hex) => match u8::from_str_radix(&String::from_utf8_lossy(hex), 16) {
                    Ok(byte) => {
                        output.push(byte);
                        index += 3;
                    }
                    Err(_) => {
                        output.push(b'%');
                        index += 1;
                    }
                },
                None => {
                    output.push(b'%');
                    index += 1;
                }
            },
            b'+' if plus_as_space => {
                output.push(b' ');
                index += 1;
            }
            byte => {
                output.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&output).to_string()
}

fn query_value(query: &str, key: &str) -> Option<String> {
    for pair in query.split('&') {
        // A pair without '=' is a bare flag; skip it instead of aborting the
        // whole scan, so `?flag&path=...` still serves `path`.
        let Some((name, value)) = pair.split_once('=') else {
            continue;
        };
        if name == key {
            return Some(percent_decode_component(value, true));
        }
    }
    None
}

/// Top-level field extractor for the tiny fixed-shape JSON bodies this
/// server accepts ({"key": value, ...}). Parsed with serde_json instead of
/// a `"key":` substring scan, so a `"key":` literal embedded in a string
/// *value* (e.g. a marks note quoting JSON) can no longer masquerade as a
/// real field. A malformed body simply yields no values, which every
/// caller already maps to the same "missing/invalid input" error shape.
fn body_value(body: &str, key: &str) -> Option<String> {
    let parsed = serde_json::from_str::<serde_json::Value>(body).ok()?;
    match parsed.get(key)? {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Bool(flag) => Some(flag.to_string()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

fn json_string(value: &str) -> String {
    format!("\"{}\"", json_escape(value))
}

/// The workstation binds loopback only, but any website open in the
/// examiner's browser can still issue `text/plain` form POSTs without a
/// CORS preflight, and a rebinding domain can point at the same port. Gate
/// every state-changing request on same-origin evidence: an Origin header
/// must be absent (curl / the opened page itself) or match the loopback
/// host, and a Host header must resolve to loopback.
fn request_is_localhost_trusted(request: &Request) -> bool {
    if let Some(origin) = request.origin.as_deref() {
        let origin = origin.trim();
        // `null` origins come from sandboxed frames, not our own pages.
        // The authority must parse to an exact loopback host: a prefix
        // match would admit `http://127.0.0.1.evil.com`.
        let loopback_origin = origin
            .strip_prefix("http://")
            .and_then(origin_authority_host)
            .is_some_and(|host| {
                matches!(host.as_str(), "127.0.0.1" | "localhost" | "[::1]" | "::1")
            });
        if !loopback_origin {
            return false;
        }
    }
    if let Some(host) = request.host.as_deref() {
        let host = host.trim().to_ascii_lowercase();
        let host_only = host.split(':').next().unwrap_or_default();
        if !matches!(host_only, "127.0.0.1" | "localhost" | "[::1]" | "::1") {
            return false;
        }
    }
    true
}

/// Extracts the lowercase host from the authority portion of an
/// `http://` Origin (the scheme is stripped by the caller). Prefix
/// matching is unsafe here: `http://127.0.0.1.evil.com` and
/// `http://127.0.0.1@evil.example` both contain the loopback literal
/// while pointing at an attacker-controlled host.
fn origin_authority_host(authority: &str) -> Option<String> {
    // Origins are `scheme://host[:port]`; a path tail is cut off and any
    // userinfo (`host@real-host`) disqualifies the value outright.
    let authority = authority.split('/').next()?;
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    let host = if let Some(rest) = authority.strip_prefix('[') {
        // IPv6 literal: `[host]` followed by nothing or a valid `:port`.
        let end = rest.find(']')?;
        let tail = &rest[end + 1..];
        if let Some(port) = tail.strip_prefix(':') {
            port.parse::<u16>().ok()?;
        } else if !tail.is_empty() {
            return None;
        }
        &rest[..end]
    } else {
        match authority.split_once(':') {
            Some((host, port)) => {
                port.parse::<u16>().ok()?;
                host
            }
            None => authority,
        }
    };
    if host.is_empty() {
        return None;
    }
    Some(host.to_ascii_lowercase())
}

fn route(request: &Request, state: &SharedState) -> Vec<u8> {
    // GET responses are safe to read from any origin only when the Host
    // header is loopback; a rebinding domain pointing at this port is the
    // documented DNS-rebinding attack against local tools.
    if !request_is_localhost_trusted(request) {
        return plain(403, b"cross-origin request rejected".to_vec());
    }
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => page(EXAMINER_PAGE.as_bytes().to_vec()),
        ("GET", "/api/env") => json(api_env()),
        ("GET", "/api/status") => json(api_status(state)),
        ("POST", "/api/start") => json(api_start(request, state)),
        ("POST", "/api/finalize") => json(api_finalize(state)),
        ("POST", "/api/verify-audit") => json(api_verify_audit(state)),
        ("POST", "/api/cancel") => json(api_cancel(state)),
        ("POST", "/api/open-case") => json(api_open_case(request, state)),
        ("POST", "/api/import-marks") => json(api_import_marks(request, state)),
        ("POST", "/api/open-folder") => {
            let path = body_value(&request.body, "path").unwrap_or_default();
            if !path.is_empty() {
                open_in_explorer(Path::new(&path));
            }
            json("{\"ok\":true}".into())
        }
        (method, path)
            if method == "GET" && (path.starts_with("/review/") || path.starts_with("/case/")) =>
        {
            serve_case_file(request, state)
        }
        _ => plain(404, b"not found".to_vec()),
    }
}

/// Tools probed at startup. `(binary, version-arg)`: TSK tools print usage
/// to stderr and exit non-zero for a bare `-V`, so presence is checked by
/// binary resolution alone (arg `""` → resolve only, don't execute).
const PROBED_TOOLS: &[(&str, &str)] = &[
    ("ffmpeg", "-version"),
    ("ffprobe", "-version"),
    ("ewfinfo", "-V"),
    ("ewfverify", "-V"),
    ("ewfexport", "-V"),
    ("mmls", ""),
    ("fls", ""),
    ("icat", ""),
];

fn api_env() -> String {
    let tools: Vec<(&str, bool)> = PROBED_TOOLS
        .iter()
        .map(|(name, arg)| (*name, tool_available(name, arg)))
        .collect();
    let has = |name: &str| tools.iter().any(|(n, ok)| *n == name && *ok);
    // Workflows require every binary in their set.
    let media = has("ffmpeg") && has("ffprobe");
    let ewf = has("ewfinfo") && has("ewfverify") && has("ewfexport");
    let tsk = has("mmls") && has("fls") && has("icat");
    format!(
        "{{\"ok\":true,\"ffmpeg\":{},\"ffprobe\":{},\"ewf\":{},\"tools\":{{{}}},\"workflows\":{{\"media\":{},\"e01\":{},\"filesystem\":{}}},\"hints\":{{{}}}}}",
        json_bool(has("ffmpeg")),
        json_bool(has("ffprobe")),
        json_bool(ewf),
        tools
            .iter()
            .map(|(name, ok)| format!("{}:{}", json_string(name), json_bool(*ok)))
            .collect::<Vec<_>>()
            .join(","),
        json_bool(media),
        json_bool(ewf),
        json_bool(tsk),
        [
            (
                "media",
                "FFmpeg 설치 후 ffmpeg/ffprobe가 PATH에 있어야 합니다. portable 배포본은 tools/bin에 복사하면 자동 인식됩니다.",
            ),
            (
                "e01",
                "libewf(ewfinfo/ewfverify/ewfexport) 설치 후 PATH 등록 또는 tools/bin에 복사하십시오.",
            ),
            (
                "filesystem",
                "Sleuth Kit(mmls/fls/icat) 설치 후 PATH 등록 또는 tools/bin에 복사하십시오.",
            ),
        ]
        .iter()
        .map(|(key, hint)| format!("{}:{}", json_string(key), json_string(hint)))
        .collect::<Vec<_>>()
        .join(","),
    )
}

fn tool_available(name: &str, version_arg: &str) -> bool {
    // resolve_tool_binary covers PATH plus the portable tools/bin layout, so
    // a bare binary dropped next to frametrace-app.exe is detected.
    let Ok(resolved) = crate::tool_policy::resolve_tool_binary(name, &[name]) else {
        return false;
    };
    // An empty version_arg means "resolution is enough" — TSK tools have no
    // -V flag and print usage to stderr, so executing them tells us nothing.
    if version_arg.is_empty() {
        return true;
    }
    let mut command = Command::new(&resolved);
    command.arg(version_arg);
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn json_bool(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn api_status(state: &SharedState) -> String {
    let guard = state_lock(state);
    let case_dir = guard.case_dir.clone();
    let byte_progress = guard.byte_progress.clone();
    let steps: Vec<String> = guard
        .steps
        .iter()
        .map(|step| json_string(step.as_str()))
        .collect();
    let current = guard
        .steps
        .iter()
        .position(|step| *step == StepStatus::Running)
        .map(|index| index.to_string())
        .unwrap_or_default();
    let logs: Vec<String> = guard
        .logs
        .iter()
        .rev()
        .take(60)
        .rev()
        .map(|line| json_string(line))
        .collect();
    let names: Vec<String> = guard.step_names.iter().copied().map(json_string).collect();
    let opt = |value: &Option<PathBuf>| match value {
        Some(path) => json_string(&path.to_string_lossy()),
        None => "null".to_string(),
    };
    let error = match &guard.error {
        Some(error) => json_string(error),
        None => "null".to_string(),
    };
    format!(
        "{{\"ok\":true,\"has_job\":{},\"phase\":{},\"steps\":[{}],\"step_names\":[{}],\"current\":\"{current}\",\"logs\":[{}],\"case_dir\":{},\"package_dir\":{},\"error\":{error},\"progress\":{}}}",
        json_bool(guard.phase != "idle"),
        json_string(guard.phase),
        steps.join(","),
        names.join(","),
        logs.join(","),
        opt(&guard.case_dir),
        opt(&guard.package_dir),
        progress_json(case_dir.as_deref(), byte_progress.as_ref()),
    )
}

/// Reads the newest running job's progress from the case DB so the UI can
/// render a real progress bar + ETA instead of an indeterminate spinner.
/// Failures degrade to `"progress":null` — status must never 500 because
/// a progress read raced the job writer.
fn progress_json(
    case_dir: Option<&Path>,
    byte_progress: Option<&(PathBuf, Option<u64>)>,
) -> String {
    let Some(case_dir) = case_dir else {
        return "null".to_string();
    };
    let job = crate::case_db::latest_running_job(case_dir).ok().flatten();
    let Some(job) = job else {
        // Fallback: external-tool steps (e.g. ewfexport) don't touch the
        // jobs table — report output bytes seen so far as an indeterminate
        // progress signal.
        return byte_progress_json(byte_progress);
    };
    let now = crate::util::now_unix().unwrap_or(job.updated_unix);
    let elapsed = now.saturating_sub(job.started_unix).max(1);
    let total = job.total_units.unwrap_or(0);
    let done = job.completed_units;
    // ETA from the average rate since job start; caller treats it as an
    // estimate ("계산 중" below some completeness) and rounds for display.
    let eta = if total > 0 && done > 0 && done < total {
        let rate = done as f64 / elapsed as f64;
        if rate > 0.0 {
            ((total - done) as f64 / rate) as u64
        } else {
            0
        }
    } else {
        0
    };
    format!(
        "{{\"job_type\":{},\"done\":{},\"total\":{},\"elapsed_secs\":{},\"eta_secs\":{}}}",
        json_string(&job.job_type),
        done,
        total,
        elapsed,
        eta
    )
}

/// Byte-level fallback for steps whose work happens inside an external
/// tool (ewfexport): the pipeline records the output path being written
/// and an optional total (E01 source size ≈ lower bound of raw bytes).
fn byte_progress_json(byte_progress: Option<&(PathBuf, Option<u64>)>) -> String {
    let Some((path, total)) = byte_progress else {
        return "null".to_string();
    };
    let done = std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    if done == 0 {
        return "null".to_string();
    }
    let total = match total {
        Some(t) if *t > done => *t,
        _ => 0, // unknown total → UI shows indeterminate + bytes written
    };
    format!(
        "{{\"job_type\":\"import-e01\",\"done\":{},\"total\":{},\"elapsed_secs\":0,\"eta_secs\":0}}",
        done, total
    )
}

fn api_start(request: &Request, state: &SharedState) -> String {
    let input_kind = match body_value(&request.body, "input_kind").as_deref() {
        Some("e01") => InputKind::E01,
        _ => InputKind::Folder,
    };
    let source_path = match body_value(&request.body, "source_path") {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path.trim()),
        _ => {
            return "{\"ok\":false,\"error\":\"증거 소스 경로를 입력하십시오.\"}".to_string();
        }
    };
    if !source_path.exists() {
        return format!(
            "{{\"ok\":false,\"error\":{}}}",
            json_string(&format!(
                "소스 경로가 존재하지 않습니다: {}",
                source_path.display()
            ))
        );
    }
    if input_kind == InputKind::E01 && !tool_available("ewfinfo", "-V") {
        return "{\"ok\":false,\"error\":\"E01 처리에는 libewf 도구(ewfinfo/ewfverify/ewfexport)가 필요합니다. 도구를 설치한 뒤 다시 시도하십시오.\"}".to_string();
    }
    let with_hash = body_value(&request.body, "with_hash").as_deref() == Some("true");
    let with_ffprobe = body_value(&request.body, "with_ffprobe").as_deref() == Some("true");
    let skip_e01_verify = body_value(&request.body, "skip_e01_verify").as_deref() == Some("true");
    let case_dir = match body_value(&request.body, "case_dir") {
        Some(dir) if !dir.trim().is_empty() => PathBuf::from(dir.trim()),
        _ => default_case_dir(),
    };
    let job = PipelineJob {
        kind: input_kind,
        case_dir: case_dir.clone(),
        source_path: source_path.clone(),
        with_hash,
        with_ffprobe,
        skip_e01_verify,
    };
    {
        let mut guard = state_lock(state);
        if guard.busy {
            return "{\"ok\":false,\"error\":\"이미 분석이 진행 중입니다.\"}".to_string();
        }
        guard.busy = true;
        guard.phase = "running";
        guard.cancel_requested = false;
        guard.steps = [StepStatus::Pending; 5];
        guard.step_names = job.kind.step_names().to_vec();
        guard.logs = Vec::new();
        guard.error = None;
        guard.package_dir = None;
        guard.case_dir = Some(case_dir.clone());
        guard.media_roots = match input_kind {
            InputKind::Folder => vec![case_dir.clone(), source_path.clone()],
            InputKind::E01 => vec![case_dir.clone()],
        };
    }
    let worker_state = Arc::clone(state);
    let spawned = thread::Builder::new()
        .name("ft-pipeline".into())
        .spawn(move || run_pipeline(worker_state, job));
    if spawned.is_err() {
        let mut guard = state_lock(state);
        guard.busy = false;
        guard.phase = "idle";
        return "{\"ok\":false,\"error\":\"작업 스레드 시작 실패\"}".to_string();
    }
    "{\"ok\":true}".to_string()
}

fn default_case_dir() -> PathBuf {
    let root = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    root.join("FrameTrace").join(format!("case-{stamp}"))
}

fn run_pipeline(state: SharedState, job: PipelineJob) {
    match job.kind {
        InputKind::Folder => run_folder_pipeline(state, job),
        InputKind::E01 => run_e01_pipeline(state, job),
    }
}

fn run_e01_pipeline(state: SharedState, job: PipelineJob) {
    let log = |state: &SharedState, line: String| state_lock(state).logs.push(line);
    let set_step = |state: &SharedState, index: usize, status: StepStatus| {
        state_lock(state).steps[index] = status;
    };
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(err) => {
            fail(
                &state,
                &format!("실행 파일 경로를 확인할 수 없습니다: {err}"),
            );
            return;
        }
    };
    let case_text = job.case_dir.to_string_lossy().to_string();
    let source_text = job.source_path.to_string_lossy().to_string();
    let raw_path = job.case_dir.join("evidence/images/evidence.raw");
    let raw_text = raw_path.to_string_lossy().to_string();

    // Step 1: case init (reuses an existing case folder).
    set_step(&state, 0, StepStatus::Running);
    if job.case_dir.join("case.json").exists() {
        log(&state, format!("기존 케이스 재사용: {case_text}"));
        set_step(&state, 0, StepStatus::Done);
    } else {
        match run_step(
            &exe,
            &[
                "init-case".into(),
                case_text.clone(),
                "--title".into(),
                "FrameTrace E01 검수 케이스".into(),
            ],
            &state,
        ) {
            Ok(output) => {
                log(&state, output);
                set_step(&state, 0, StepStatus::Done);
            }
            Err(err) => {
                set_step(&state, 0, StepStatus::Failed);
                fail(&state, &err);
                return;
            }
        }
    }

    // Step 2: import the E01 (ewfverify runs unless explicitly skipped).
    // A pre-existing raw image is reused ONLY when it was imported from
    // the same E01 path; otherwise the wizard would silently analyze the
    // previous evidence end-to-end (wrong-evidence results).
    set_step(&state, 1, StepStatus::Running);
    let raw_source_binding = raw_path.with_extension("raw.source");
    let reuse_existing_raw = raw_path.exists() && {
        let previous_source = std::fs::read_to_string(&raw_source_binding)
            .map(|text| text.trim().to_string())
            .unwrap_or_default();
        let same_source = previous_source == source_text;
        if !same_source && !previous_source.is_empty() {
            log(
                &state,
                format!(
                    "기존 raw 이미지는 다른 E01에서 임포트된 것입니다. 재임포트합니다: {raw_text}"
                ),
            );
        }
        same_source
    };
    if reuse_existing_raw {
        log(&state, format!("기존 raw 이미지 재사용: {raw_text}"));
        set_step(&state, 1, StepStatus::Done);
    } else {
        let mut args: Vec<String> = vec![
            "import-e01".into(),
            case_text.clone(),
            source_text.clone(),
            "--output".into(),
            raw_text.clone(),
        ];
        if job.skip_e01_verify {
            args.push("--skip-verify".into());
        }
        // ewfexport runs inside the CLI child, so no in-process progress
        // callback exists — the UI instead reports the growing raw file
        // size as a byte-level progress signal. E01 is compressed, so the
        // source size is only a lower bound; pass None (indeterminate).
        state_lock(&state).byte_progress = Some((raw_path.clone(), None));
        let import_result = run_step(&exe, &args, &state);
        state_lock(&state).byte_progress = None;
        match import_result {
            Ok(output) => {
                log(&state, output);
                // Bind the imported raw to this E01 so a later run with a
                // different E01 re-imports instead of reusing stale bytes.
                let _ = std::fs::write(&raw_source_binding, &source_text);
                set_step(&state, 1, StepStatus::Done);
            }
            Err(err) => {
                set_step(&state, 1, StepStatus::Failed);
                fail(&state, &err);
                return;
            }
        }
    }

    // Step 3: partition table + file listing (auto-selects the partition).
    set_step(&state, 2, StepStatus::Running);
    match run_step(
        &exe,
        &["inspect-image".into(), case_text.clone(), raw_text.clone()],
        &state,
    ) {
        Ok(output) => {
            log(&state, output);
            set_step(&state, 2, StepStatus::Done);
        }
        Err(err) => {
            set_step(&state, 2, StepStatus::Failed);
            fail(&state, &err);
            return;
        }
    }

    // Step 4: refresh the logical index over the case evidence tree so the
    // review bundle always has a current db/video_index.json (normally 0
    // logical files for a pure E01 case; recovered exports land here later).
    set_step(&state, 3, StepStatus::Running);
    let mut scan_args: Vec<String> = vec![
        "scan-folder".into(),
        case_text.clone(),
        job.case_dir.join("evidence").to_string_lossy().to_string(),
        "--no-ffprobe".into(),
    ];
    if job.with_hash {
        scan_args.push("--hash".into());
    }
    match run_step(&exe, &scan_args, &state) {
        Ok(output) => {
            log(&state, output);
            set_step(&state, 3, StepStatus::Done);
        }
        Err(err) => {
            log(&state, format!("논리 색인 건너뜀: {err}"));
            set_step(&state, 3, StepStatus::Done);
        }
    }

    // Step 5: candidate anomaly scan (non-fatal) then review bundle.
    match run_step(
        &exe,
        &["qa".into(), "anomalies".into(), case_text.clone()],
        &state,
    ) {
        Ok(output) => log(&state, output),
        Err(err) => log(&state, format!("이상 징후 스캔 건너뜀: {err}")),
    }
    set_step(&state, 4, StepStatus::Running);
    match run_step(&exe, &["make-review".into(), case_text.clone()], &state) {
        Ok(output) => {
            log(&state, output);
            set_step(&state, 4, StepStatus::Done);
            let mut guard = state_lock(&state);
            guard.phase = "review-ready";
            guard.busy = false;
            guard.byte_progress = None;
            guard.logs.push(
                "검토 화면이 준비되었습니다. 삭제파일 복구·카빙은 CLI(recover-inode/carve-file)로 수행한 뒤 재검토하십시오.".into(),
            );
        }
        Err(err) => {
            set_step(&state, 4, StepStatus::Failed);
            fail(&state, &err);
        }
    }
}

fn run_folder_pipeline(state: SharedState, job: PipelineJob) {
    let case_dir = job.case_dir;
    let source_path = job.source_path;
    let with_hash = job.with_hash;
    let with_ffprobe = job.with_ffprobe;
    let log = |state: &SharedState, line: String| state_lock(state).logs.push(line);
    let set_step = |state: &SharedState, index: usize, status: StepStatus| {
        state_lock(state).steps[index] = status;
    };
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(err) => {
            fail(
                &state,
                &format!("실행 파일 경로를 확인할 수 없습니다: {err}"),
            );
            return;
        }
    };
    let case_text = case_dir.to_string_lossy().to_string();
    let source_text = source_path.to_string_lossy().to_string();

    set_step(&state, 0, StepStatus::Running);
    if case_dir.join("case.json").exists() {
        log(&state, format!("기존 케이스 재사용: {case_text}"));
        set_step(&state, 0, StepStatus::Done);
    } else {
        match run_step(
            &exe,
            &[
                "init-case".into(),
                case_text.clone(),
                "--title".into(),
                "FrameTrace 검수 케이스".into(),
            ],
            &state,
        ) {
            Ok(output) => {
                log(&state, output);
                set_step(&state, 0, StepStatus::Done);
            }
            Err(err) => {
                set_step(&state, 0, StepStatus::Failed);
                fail(&state, &err);
                return;
            }
        }
    }

    set_step(&state, 1, StepStatus::Running);
    match run_step(
        &exe,
        &[
            "register-source".into(),
            case_text.clone(),
            source_text.clone(),
            "--kind".into(),
            "folder".into(),
            "--write-protect".into(),
            "launcher-managed read-only review".into(),
        ],
        &state,
    ) {
        Ok(output) => {
            log(&state, output);
            set_step(&state, 1, StepStatus::Done);
        }
        Err(err) => {
            log(
                &state,
                format!("소스 등록 건너뜀(이미 등록된 경우 무시): {err}"),
            );
            set_step(&state, 1, StepStatus::Done);
        }
    }

    set_step(&state, 2, StepStatus::Running);
    let mut scan_args: Vec<String> = vec!["scan-folder".into(), case_text.clone(), source_text];
    if with_hash {
        scan_args.push("--hash".into());
    }
    if !with_ffprobe {
        scan_args.push("--no-ffprobe".into());
    }
    match run_step(&exe, &scan_args, &state) {
        Ok(output) => {
            log(&state, output);
            set_step(&state, 2, StepStatus::Done);
        }
        Err(err) => {
            set_step(&state, 2, StepStatus::Failed);
            fail(&state, &err);
            return;
        }
    }

    set_step(&state, 3, StepStatus::Running);
    if with_ffprobe {
        let selection_path = case_dir.join("selection-all.json");
        match build_selection_file(&case_dir, &selection_path) {
            Ok(count) => {
                log(&state, format!("검증 대상 {count}건"));
                match run_step(
                    &exe,
                    &[
                        "validate-batch".into(),
                        case_text.clone(),
                        selection_path.to_string_lossy().to_string(),
                    ],
                    &state,
                ) {
                    Ok(output) => {
                        log(&state, output);
                        set_step(&state, 3, StepStatus::Done);
                    }
                    Err(err) => {
                        log(&state, format!("일괄 검증 실패, 건너뜀: {err}"));
                        set_step(&state, 3, StepStatus::Done);
                    }
                }
            }
            Err(err) => {
                log(&state, format!("선택 목록 생성 실패, 검증 건너뜀: {err}"));
                set_step(&state, 3, StepStatus::Done);
            }
        }
    } else {
        log(&state, "ffprobe 검증이 꺼져 있어 건너뜁니다.".to_string());
        set_step(&state, 3, StepStatus::Done);
    }

    match run_step(
        &exe,
        &["qa".into(), "anomalies".into(), case_text.clone()],
        &state,
    ) {
        Ok(output) => log(&state, output),
        Err(err) => log(&state, format!("이상 징후 스캔 건너뜀: {err}")),
    }

    set_step(&state, 4, StepStatus::Running);
    match run_step(&exe, &["make-review".into(), case_text.clone()], &state) {
        Ok(output) => {
            log(&state, output);
            set_step(&state, 4, StepStatus::Done);
            let mut guard = state_lock(&state);
            guard.phase = "review-ready";
            guard.busy = false;
            guard.byte_progress = None;
            guard
                .logs
                .push("검토 화면이 준비되었습니다. 뷰어에서 증거를 확인하십시오.".into());
        }
        Err(err) => {
            set_step(&state, 4, StepStatus::Failed);
            fail(&state, &err);
        }
    }
}

fn fail(state: &SharedState, message: &str) {
    let mut guard = state_lock(state);
    guard.phase = "error";
    guard.busy = false;
    guard.byte_progress = None;
    guard.error = Some(message.to_string());
    guard.logs.push(format!("오류: {message}"));
}

/// How much of a pipeline step's stdout/stderr the workstation keeps.
/// The full stream is drained (required to prevent the deadlock below) but
/// only this tail is retained for the log panel and error reporting.
const STEP_OUTPUT_TAIL_BYTES: usize = 256 * 1024;

fn run_step(exe: &Path, args: &[String], state: &SharedState) -> Result<String, String> {
    let mut child = Command::new(exe)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("{} 실행 실패: {err}", exe.display()))?;
    // Both pipes must be drained while the child runs: a child that fills
    // the OS pipe buffer blocks on write, and a parent that only polls
    // try_wait() then waits forever — the classic pipe deadlock. tsk.rs
    // uses the same reader-thread pattern for icat.
    let mut stdout_pipe = child.stdout.take().expect("stdout was piped");
    let mut stderr_pipe = child.stderr.take().expect("stderr was piped");
    let stdout_reader = thread::spawn(move || drain_capped(&mut stdout_pipe));
    let stderr_reader = thread::spawn(move || drain_capped(&mut stderr_pipe));
    let status = loop {
        if state_lock(state).cancel_requested {
            let _ = child.kill();
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(150)),
            Err(err) => return Err(format!("{} 실행 대기 실패: {err}", exe.display())),
        }
    };
    // The child has exited, so both pipes are at EOF and these joins
    // return immediately with whatever tail was retained.
    let stdout_tail = stdout_reader.join().unwrap_or_default();
    let stderr_tail = stderr_reader.join().unwrap_or_default();
    if state_lock(state).cancel_requested {
        return Err("사용자가 분석을 중단했습니다.".to_string());
    }
    let mut text = String::from_utf8_lossy(&stdout_tail).to_string();
    let stderr = String::from_utf8_lossy(&stderr_tail);
    if !stderr.trim().is_empty() {
        text.push_str(&stderr);
    }
    if !status.success() {
        let mut tail_lines: Vec<&str> = text.lines().rev().take(4).collect();
        tail_lines.reverse();
        let tail = tail_lines.join(" | ");
        return Err(format!(
            "{} 실패 (exit {:?}): {}",
            args.first().map(String::as_str).unwrap_or("command"),
            status.code(),
            tail
        ));
    }
    Ok(text.trim_end().to_string())
}

/// Drains `pipe` to EOF, retaining only the last `STEP_OUTPUT_TAIL_BYTES`.
/// Retention stays bounded even when a step emits unbounded progress
/// output, while the drain itself keeps the child's write end from ever
/// blocking on a full pipe.
fn drain_capped(pipe: &mut impl std::io::Read) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match pipe.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                buffer.extend_from_slice(&chunk[..read]);
                if buffer.len() > STEP_OUTPUT_TAIL_BYTES {
                    let excess = buffer.len() - STEP_OUTPUT_TAIL_BYTES;
                    buffer.drain(..excess);
                }
            }
        }
    }
    buffer
}

/// Extract every top-level `videos[].id` from db/video_index.json and
/// write a validate-batch selection file covering all of them. The index
/// is one JSON document (see `scan_index_json`); parsing it properly keeps
/// `"id"` literals inside string values or nested ffprobe records from
/// being mistaken for video ids.
fn build_selection_file(case_dir: &Path, output: &Path) -> Result<usize, String> {
    let index_path = case_dir.join("db/video_index.json");
    let index = std::fs::read_to_string(&index_path)
        .map_err(|err| format!("failed to read {}: {err}", index_path.display()))?;
    let parsed =
        serde_json::from_str::<serde_json::Value>(&index).unwrap_or(serde_json::Value::Null);
    let ids: Vec<String> = parsed
        .get("videos")
        .and_then(|videos| videos.as_array())
        .map(|videos| {
            videos
                .iter()
                .filter_map(|video| video.get("id").and_then(|id| id.as_str()))
                .filter(|id| id.starts_with("vid_"))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if ids.is_empty() {
        return Err("색인된 영상이 없습니다 (스캔 결과 확인 필요)".into());
    }
    let items = ids
        .iter()
        .map(|id| {
            format!(
                "{{\"selector\":{},\"kind\":\"video\",\"action\":\"validate\"}}",
                json_string(id)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let file = format!("{{\"schema_version\":1,\"items\":[{items}]}}");
    std::fs::write(output, file)
        .map_err(|err| format!("failed to write {}: {err}", output.display()))?;
    Ok(ids.len())
}

/// Accepts the viewer's downloaded marks JSON, stores it in the case, and
/// refreshes the report so examiner marks land in the deliverable.
fn api_import_marks(request: &Request, state: &SharedState) -> String {
    let marks_body = match body_value(&request.body, "marks_json") {
        Some(text) if !text.trim().is_empty() => text,
        _ => {
            return "{\"ok\":false,\"error\":\"마크 JSON이 비어 있습니다.\"}".to_string();
        }
    };
    let (case_dir, busy) = {
        let guard = state_lock(state);
        (guard.case_dir.clone(), guard.busy)
    };
    let Some(case_dir) = case_dir else {
        return "{\"ok\":false,\"error\":\"먼저 INPUT 분석을 실행하십시오.\"}".to_string();
    };
    if busy {
        return "{\"ok\":false,\"error\":\"작업이 진행 중입니다.\"}".to_string();
    }
    let marks_path = case_dir.join("marks-imported.json");
    if let Err(err) = std::fs::write(&marks_path, &marks_body) {
        return format!(
            "{{\"ok\":false,\"error\":{}}}",
            json_string(&err.to_string())
        );
    }
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(err) => {
            return format!(
                "{{\"ok\":false,\"error\":{}}}",
                json_string(&err.to_string())
            );
        }
    };
    let case_text = case_dir.to_string_lossy().to_string();
    if let Err(err) = run_step(
        &exe,
        &[
            "import-marks".into(),
            case_text.clone(),
            marks_path.to_string_lossy().to_string(),
        ],
        state,
    ) {
        return format!("{{\"ok\":false,\"error\":{}}}", json_string(&err));
    }
    if let Err(err) = run_step(&exe, &["make-report".into(), case_text.clone()], state) {
        return format!("{{\"ok\":false,\"error\":{}}}", json_string(&err));
    }
    {
        let mut guard = state_lock(state);
        guard
            .logs
            .push("판독 마크를 반영해 보고서를 갱신했습니다.".into());
    }
    "{\"ok\":true,\"report_url\":\"case/reports/case-report.html\"}".to_string()
}

/// `POST /api/verify-audit`: verify every chained audit log under
/// `case_dir/evidence/logs/*.jsonl` in-process and report per-log and
/// overall integrity (keyed vs structural-only vs failed).
fn api_cancel(state: &SharedState) -> String {
    let mut guard = state_lock(state);
    if !guard.busy {
        return "{\"ok\":false,\"error\":\"진행 중인 작업이 없습니다.\"}".to_string();
    }
    guard.cancel_requested = true;
    guard
        .logs
        .push("중단 요청을 받았습니다. 현재 단계를 멈추는 중입니다.".to_string());
    "{\"ok\":true}".to_string()
}

fn api_open_case(request: &Request, state: &SharedState) -> String {
    let dir_text = body_value(&request.body, "case_dir").unwrap_or_default();
    if dir_text.trim().is_empty() {
        return "{\"ok\":false,\"error\":\"케이스 폴더 경로를 입력하십시오.\"}".to_string();
    }
    let case_dir = PathBuf::from(dir_text.trim());
    if !case_dir.join("case.json").is_file() {
        return "{\"ok\":false,\"error\":\"해당 폴더에 case.json이 없습니다 — 기존 케이스 폴더가 아닙니다.\"}"
            .to_string();
    }
    let has_review = case_dir.join("review/index.html").is_file();
    let mut guard = state_lock(state);
    if guard.busy {
        return "{\"ok\":false,\"error\":\"분석이 진행 중입니다.\"}".to_string();
    }
    guard.case_dir = Some(case_dir.clone());
    guard.media_roots = vec![case_dir.clone()];
    if has_review {
        guard.phase = "review-ready";
        guard.steps = [StepStatus::Done; 5];
        guard
            .logs
            .push(format!("기존 케이스를 열었습니다: {}", case_dir.display()));
    }
    format!(
        "{{\"ok\":true,\"has_review\":{}}}",
        if has_review { "true" } else { "false" }
    )
}

fn api_verify_audit(state: &SharedState) -> String {
    let case_dir = {
        let guard = state_lock(state);
        guard.case_dir.clone()
    };
    let Some(case_dir) = case_dir else {
        return "{\"ok\":false,\"error\":\"먼저 INPUT 분석을 실행하십시오.\"}".to_string();
    };
    let logs_dir = case_dir.join("evidence/logs");
    let mut log_paths: Vec<PathBuf> = match std::fs::read_dir(&logs_dir) {
        Ok(read_dir) => read_dir
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("jsonl"))
            .collect(),
        Err(err) => {
            return format!(
                "{{\"ok\":false,\"error\":{}}}",
                json_string(&format!("감사 로그 디렉터리를 읽지 못했습니다: {err}"))
            );
        }
    };
    log_paths.sort();
    if log_paths.is_empty() {
        return "{\"ok\":false,\"error\":\"검증할 감사 로그가 없습니다.\"}".to_string();
    }
    let mut logs_json = Vec::new();
    let mut failed = 0usize;
    let mut any_structural_only = false;
    let mut total_entries = 0usize;
    for path in &log_paths {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        match audit::verify_chained_jsonl(path) {
            Ok(result) => {
                total_entries += result.entries;
                if result.integrity == audit::AuditIntegrity::StructuralOnly {
                    any_structural_only = true;
                }
                let warnings: Vec<String> =
                    result.warnings.iter().map(|w| json_string(w)).collect();
                logs_json.push(format!(
                    "{{\"name\":{},\"entries\":{},\"integrity\":{},\"keyed_entries\":{},\"unauthenticated_keyed_entries\":{},\"warnings\":[{}]}}",
                    json_string(&name),
                    result.entries,
                    json_string(result.integrity.label()),
                    result.keyed_entries,
                    result.unauthenticated_keyed_entries,
                    warnings.join(",")
                ));
            }
            Err(err) => {
                failed += 1;
                logs_json.push(format!(
                    "{{\"name\":{},\"entries\":0,\"integrity\":\"failed\",\"error\":{},\"warnings\":[]}}",
                    json_string(&name),
                    json_string(&err)
                ));
            }
        }
    }
    let overall = if failed > 0 {
        "integrity-failed"
    } else if any_structural_only {
        "integrity-structural-only"
    } else {
        "integrity-keyed"
    };
    format!(
        "{{\"ok\":{},\"verified\":{},\"failed\":{},\"entries\":{},\"integrity\":{},\"logs\":[{}]}}",
        if failed == 0 { "true" } else { "false" },
        log_paths.len() - failed,
        failed,
        total_entries,
        json_string(overall),
        logs_json.join(",")
    )
}

fn api_finalize(state: &SharedState) -> String {
    let (case_dir, busy) = {
        let guard = state_lock(state);
        (guard.case_dir.clone(), guard.busy)
    };
    let Some(case_dir) = case_dir else {
        return "{\"ok\":false,\"error\":\"먼저 INPUT 분석을 실행하십시오.\"}".to_string();
    };
    if busy {
        return "{\"ok\":false,\"error\":\"분석이 아직 진행 중입니다.\"}".to_string();
    }
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(err) => {
            return format!(
                "{{\"ok\":false,\"error\":{}}}",
                json_string(&err.to_string())
            );
        }
    };
    {
        let mut guard = state_lock(state);
        guard.phase = "finalizing";
        guard.busy = true;
        guard.error = None;
        guard.logs.push("결과 보고서 생성 중…".into());
    }
    let case_text = case_dir.to_string_lossy().to_string();
    // --rehash makes the packaged report re-digest every indexed source file
    // and flag hash-revalidation mismatches — the integrity claim behind a
    // finalized package should rest on live digests, not stored scan hashes.
    let report = run_step(
        &exe,
        &["make-report".into(), case_text.clone(), "--rehash".into()],
        state,
    );
    let packaging = if report.is_ok() {
        run_step(&exe, &["package-case".into(), case_text.clone()], state)
    } else {
        report.clone()
    };
    let mut guard = state_lock(state);
    match (report, packaging) {
        (Ok(_), Ok(_)) => {
            let package_dir = newest_package_dir(&case_dir);
            let package_url = package_dir
                .as_ref()
                .and_then(|path| path.strip_prefix(&case_dir).ok())
                .map(|relative| relative.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            guard.phase = "done";
            guard.busy = false;
            guard.byte_progress = None;
            guard.package_dir = package_dir.clone();
            guard.logs.push("보고서·패키지 생성 완료.".into());
            format!(
                "{{\"ok\":true,\"package_url\":{},\"package_dir\":{}}}",
                json_string(&package_url),
                opt_path(&package_dir)
            )
        }
        (report, packaging) => {
            let error = report.err().or_else(|| packaging.err()).unwrap_or_default();
            guard.phase = "review-ready";
            guard.busy = false;
            guard.error = Some(error.clone());
            format!("{{\"ok\":false,\"error\":{}}}", json_string(&error))
        }
    }
}

fn opt_path(value: &Option<PathBuf>) -> String {
    match value {
        Some(path) => json_string(&path.to_string_lossy()),
        None => "null".to_string(),
    }
}

fn newest_package_dir(case_dir: &Path) -> Option<PathBuf> {
    let entries = case_dir.join("reports").read_dir().ok()?;
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in entries.flatten() {
        let name = entry.file_name();
        if !name.to_string_lossy().starts_with("package_") || !entry.path().is_dir() {
            continue;
        }
        if let Ok(modified) = entry.metadata().and_then(|meta| meta.modified())
            && best.as_ref().is_none_or(|(time, _)| modified > *time)
        {
            best = Some((modified, entry.path()));
        }
    }
    best.map(|(_, path)| path)
}

fn page(body: Vec<u8>) -> Vec<u8> {
    respond(200, "text/html; charset=utf-8", body, "no-store", None)
}

fn json(body: String) -> Vec<u8> {
    respond(
        200,
        "application/json; charset=utf-8",
        body.into_bytes(),
        "no-store",
        None,
    )
}

fn plain(code: u16, body: Vec<u8>) -> Vec<u8> {
    respond(code, "text/plain; charset=utf-8", body, "no-store", None)
}

fn serve_case_file(request: &Request, state: &SharedState) -> Vec<u8> {
    let case_dir = state_lock(state).case_dir.clone();
    let Some(case_dir) = case_dir else {
        return plain(404, b"no case loaded".to_vec());
    };
    let prefix = if request.path.starts_with("/review/") {
        "/review/"
    } else {
        "/case/"
    };
    let relative = request.path.trim_start_matches(prefix);
    if relative.is_empty() || relative.contains("..") {
        return plain(403, b"invalid path".to_vec());
    }
    let full = if prefix == "/review/" {
        case_dir.join("review").join(relative)
    } else {
        case_dir.join(relative)
    };
    match full.canonicalize() {
        Ok(path) if path_is_under(&case_dir, &path) => match std::fs::read(&path) {
            Ok(bytes) => respond(200, mime_for(&path), bytes, "private, max-age=60", None),
            Err(_) => plain(404, b"file not found".to_vec()),
        },
        Ok(_) => plain(403, b"path outside case".to_vec()),
        Err(_) => plain(404, b"file not found".to_vec()),
    }
}

fn path_is_under(root: &Path, candidate: &Path) -> bool {
    let Ok(root_canonical) = root.canonicalize() else {
        return false;
    };
    // Component-wise comparison on canonicalized paths: both sides resolve
    // to the filesystem's real casing, so the prefix check is exact. The
    // old case-folded string compare treated `CASE/` as `case/` — on a
    // case-sensitive filesystem that is a *different* directory, i.e. a
    // containment bypass.
    candidate.starts_with(&root_canonical)
}

/// Streams a media file with HTTP Range support so <video> can seek.
fn serve_media(
    stream: &mut TcpStream,
    request: &Request,
    state: &SharedState,
) -> Result<(), String> {
    let Some(raw_path) = query_value(&request.query, "path") else {
        return write_simple(stream, 400, b"missing path");
    };
    let candidate = PathBuf::from(&raw_path);
    let roots = state_lock(state).media_roots.clone();
    let canonical = match candidate.canonicalize() {
        Ok(path) => path,
        Err(_) => return write_simple(stream, 404, b"media not found"),
    };
    let allowed = roots
        .iter()
        .filter_map(|root| root.canonicalize().ok())
        .any(|root| path_is_under(&root, &canonical));
    if !allowed {
        return write_simple(stream, 403, b"path outside approved roots");
    }
    let total = match std::fs::metadata(&canonical) {
        Ok(meta) => meta.len(),
        Err(_) => return write_simple(stream, 404, b"media not found"),
    };
    let (start, end, code) = match request
        .range
        .as_deref()
        .and_then(|value| parse_range(value, total))
    {
        Some((start, end)) if start < total => {
            let end = end.unwrap_or(total - 1).min(total - 1);
            if end < start {
                return write_range_unsatisfiable(stream, total);
            }
            (start, end, 206u16)
        }
        Some(_) => return write_range_unsatisfiable(stream, total),
        // A zero-byte file must report Content-Length: 0. The previous
        // `total.saturating_sub(1)` produced `end = 0, length = 1` and
        // stalled browsers waiting for a byte that never comes.
        None => (0u64, total.saturating_sub(1), 200u16),
    };
    let length = end.saturating_sub(start) + if total == 0 { 0 } else { 1 };
    let mut file = match std::fs::File::open(&canonical) {
        Ok(file) => file,
        Err(_) => return write_simple(stream, 404, b"media not found"),
    };
    if file.seek(SeekFrom::Start(start)).is_err() {
        return write_simple(stream, 500, b"seek failed");
    }
    let mut head = format!(
        "HTTP/1.1 {code} {}\r\nContent-Type: {}\r\nContent-Length: {length}\r\nAccept-Ranges: bytes\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n",
        if code == 206 { "Partial Content" } else { "OK" },
        mime_for(&canonical)
    );
    if code == 206 {
        head.push_str(&format!("Content-Range: bytes {start}-{end}/{total}\r\n"));
    }
    head.push_str("\r\n");
    stream
        .write_all(head.as_bytes())
        .map_err(|err| format!("write failed: {err}"))?;
    let mut remaining = length;
    let mut chunk = [0u8; 64 * 1024];
    let mut reader = file;
    while remaining > 0 {
        let want = remaining.min(chunk.len() as u64) as usize;
        let read = reader
            .read(&mut chunk[..want])
            .map_err(|err| format!("media read failed: {err}"))?;
        if read == 0 {
            break;
        }
        stream
            .write_all(&chunk[..read])
            .map_err(|err| format!("media write failed: {err}"))?;
        remaining -= read as u64;
    }
    let _ = stream.flush();
    Ok(())
}

fn write_range_unsatisfiable(stream: &mut TcpStream, total: u64) -> Result<(), String> {
    let head = format!(
        "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{total}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(head.as_bytes())
        .map_err(|err| format!("write failed: {err}"))
}

fn write_simple(stream: &mut TcpStream, code: u16, body: &[u8]) -> Result<(), String> {
    let head = format!(
        "HTTP/1.1 {code} {}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        if code == 200 { "OK" } else { "Error" },
        body.len()
    );
    stream
        .write_all(head.as_bytes())
        .and_then(|_| stream.write_all(body))
        .map_err(|err| format!("write failed: {err}"))
}

/// Parses an HTTP `Range` header against a known entity length.
/// Supports `bytes=start-end`, `bytes=start-`, and the suffix form
/// `bytes=-N` (Safari issues suffix ranges; returning None there would
/// make its <video> requests unanswerable).
fn parse_range(value: &str, total: u64) -> Option<(u64, Option<u64>)> {
    let value = value.strip_prefix("bytes=")?;
    let (start_text, end_text) = value.split_once('-')?;
    if start_text.is_empty() {
        // Suffix range: the last N bytes. A zero or unparseable length is
        // unsatisfiable per RFC 7233 — signal it via start >= total so the
        // caller maps to 416.
        let suffix = end_text.parse::<u64>().ok()?;
        if suffix == 0 {
            return Some((u64::MAX, None));
        }
        let start = total.saturating_sub(suffix);
        return Some((start, None));
    }
    let start = start_text.parse::<u64>().ok()?;
    let end = if end_text.is_empty() {
        None
    } else {
        Some(end_text.parse::<u64>().ok()?)
    };
    Some((start, end))
}

fn respond(
    code: u16,
    content_type: &str,
    body: Vec<u8>,
    cache: &str,
    extra: Option<&str>,
) -> Vec<u8> {
    let status_text = match code {
        200 => "OK",
        206 => "Partial Content",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        416 => "Range Not Satisfiable",
        _ => "OK",
    };
    let mut head = format!(
        "HTTP/1.1 {code} {status_text}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: {cache}\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(extra) = extra {
        head.push_str(extra);
    }
    head.push_str("\r\n");
    let mut bytes = head.into_bytes();
    bytes.extend_from_slice(&body);
    bytes
}

fn mime_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("mp4") | Some("m4v") => "video/mp4",
        Some("webm") => "video/webm",
        Some("avi") => "video/x-msvideo",
        Some("mov") => "video/quicktime",
        Some("tsv") => "text/tab-separated-values; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_percent_and_plus_in_paths() {
        // Path components must keep `+` as-is (legal in filenames);
        assert_eq!(
            percent_decode_component("C%3A%5CUsers%5C%ED%95%9C%EA%B8%80%5Ca+b.mp4", false),
            "C:\\Users\\한글\\a+b.mp4"
        );
        assert_eq!(percent_decode_component("plain", false), "plain");
        assert_eq!(percent_decode_component("bad%2", false), "bad%2");
        assert_eq!(percent_decode_component("tail%", false), "tail%");
        // Query values do translate `+` to space.
        assert_eq!(percent_decode_component("a+b%20c", true), "a b c");
        // A bare flag pair no longer aborts later query parsing.
        assert_eq!(
            query_value("flag&path=%2Fev", "path").as_deref(),
            Some("/ev")
        );
    }

    #[test]
    fn parses_media_range_headers() {
        assert_eq!(parse_range("bytes=0-1023", 2048), Some((0, Some(1023))));
        assert_eq!(parse_range("bytes=100-", 2048), Some((100, None)));
        // Suffix ranges: last N bytes of a 2048-byte entity.
        assert_eq!(parse_range("bytes=-500", 2048), Some((1548, None)));
        // A suffix longer than the entity clamps to the whole entity.
        assert_eq!(parse_range("bytes=-99999", 2048), Some((0, None)));
        // A zero suffix is unsatisfiable (start >= total → caller maps 416).
        assert_eq!(parse_range("bytes=-0", 2048), Some((u64::MAX, None)));
        assert_eq!(parse_range("items=1-2", 2048), None);
        assert_eq!(parse_range("bytes=abc-", 2048), None);
        assert_eq!(parse_range("bytes=-abc", 2048), None);
        // Zero-byte entity: a suffix range resolves to start 0, and the
        // caller's `start < total` check maps it to 416 rather than
        // trying to stream a byte that does not exist.
        assert_eq!(parse_range("bytes=-10", 0), Some((0, None)));
    }

    #[test]
    fn progress_json_reports_running_job_with_eta() {
        let case_dir =
            std::env::temp_dir().join(format!("ft-progress-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&case_dir);
        std::fs::create_dir_all(&case_dir).unwrap();

        // No job row yet → null.
        assert_eq!(progress_json(Some(&case_dir), None), "null");

        let job = crate::case_db::start_job(
            &case_dir,
            "scan-folder",
            Path::new("/evidence"),
            Some(100),
            "{}",
        )
        .unwrap();
        crate::case_db::report_job_progress(&case_dir, &job.job_id, Some(100), 50).unwrap();

        let json = progress_json(Some(&case_dir), None);
        assert!(json.contains("\"job_type\":\"scan-folder\""), "{json}");
        assert!(json.contains("\"done\":50"), "{json}");
        assert!(json.contains("\"total\":100"), "{json}");
        assert!(json.contains("\"elapsed_secs\":"), "{json}");

        crate::case_db::complete_job(&case_dir, &job.job_id, 100, "done").unwrap();
        assert_eq!(progress_json(Some(&case_dir), None), "null");

        let _ = std::fs::remove_dir_all(case_dir);
    }

    #[test]
    fn byte_progress_reports_growing_output_file() {
        let case_dir =
            std::env::temp_dir().join(format!("ft-byteprogress-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&case_dir);
        std::fs::create_dir_all(&case_dir).unwrap();
        let raw = case_dir.join("evidence.raw");
        std::fs::write(&raw, vec![0u8; 4096]).unwrap();

        let json = byte_progress_json(Some(&(raw.clone(), None)));
        assert!(json.contains("\"done\":4096"), "{json}");
        assert!(json.contains("\"total\":0"), "{json}");

        // A known larger total yields a real numerator/denominator pair.
        let json = byte_progress_json(Some(&(raw.clone(), Some(8192))));
        assert!(json.contains("\"total\":8192"), "{json}");

        // Missing file or absent hint degrades to null, never an error.
        std::fs::remove_file(&raw).unwrap();
        assert_eq!(byte_progress_json(Some(&(raw, None))), "null");
        assert_eq!(byte_progress_json(None), "null");

        let _ = std::fs::remove_dir_all(case_dir);
    }

    #[test]
    fn extracts_json_body_values() {
        let body = r#"{"input_kind":"e01","source_path":"C:\\a\\b.mp4","with_hash":true,"rows":3,"empty":false}"#;
        assert_eq!(body_value(body, "input_kind").as_deref(), Some("e01"));
        assert_eq!(
            body_value(body, "source_path").as_deref(),
            Some("C:\\a\\b.mp4")
        );
        assert_eq!(body_value(body, "with_hash").as_deref(), Some("true"));
        assert_eq!(body_value(body, "empty").as_deref(), Some("false"));
        assert_eq!(body_value(body, "rows").as_deref(), Some("3"));
        assert_eq!(body_value(body, "missing"), None);
        // Korean and other non-ASCII string escapes must decode.
        assert_eq!(
            body_value(r#"{"notes":"\ud655\uc778"}"#, "notes").as_deref(),
            Some("확인")
        );
        // Surrogate pairs decode to the intended non-BMP scalar.
        assert_eq!(
            body_value(r#"{"notes":"\ud83d\ude00"}"#, "notes").as_deref(),
            Some("😀")
        );
    }

    #[test]
    fn body_parser_rejects_malformed_and_value_embedded_keys() {
        // Malformed JSON (here: a truncated \u escape) rejects the whole
        // body instead of yielding a partial value.
        assert_eq!(body_value(r#"{"notes":"a\u12"}"#, "notes"), None);
        assert_eq!(body_value("not json at all", "path"), None);
        assert_eq!(body_value(r#"{"path":"unterminated"#, "path"), None);
        // The old substring scan treated a `"key":` literal inside a
        // string *value* as a real field — the exact confusion serde_json
        // removes.
        let body = r#"{"note":"see {\"path\":\"C:\\evil\"} quoted","path":"C:\\real"}"#;
        assert_eq!(body_value(body, "path").as_deref(), Some("C:\\real"));
        let body = r#"{"note":"contains \"source_path\":\"C:\\evil\" text"}"#;
        assert_eq!(body_value(body, "source_path"), None);
        // Keys nested inside other objects are not top-level fields.
        let body = r#"{"outer":{"path":"C:\\nested"},"path":null}"#;
        assert_eq!(body_value(body, "path"), None);
        // A real top-level marks_json string holding embedded JSON still
        // round-trips as a plain string.
        let body = r#"{"marks_json":"{\"marks\":[{\"t\":1}]}"}"#;
        assert_eq!(
            body_value(body, "marks_json").as_deref(),
            Some(r#"{"marks":[{"t":1}]}"#)
        );
    }

    #[test]
    fn reads_query_values() {
        assert_eq!(
            query_value("path=C%3A%5Cx.mp4&range=0-1", "path").as_deref(),
            Some("C:\\x.mp4")
        );
        assert_eq!(query_value("a=1", "b"), None);
    }

    #[test]
    fn containment_rejects_prefix_siblings() {
        let base = std::env::temp_dir().join(format!("ft_under_root_{}", std::process::id()));
        let sibling = std::env::temp_dir().join(format!("ft_under_root_{}_x", std::process::id()));
        let child = base.join("sub");
        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::remove_dir_all(&sibling);
        std::fs::create_dir_all(&child).unwrap();
        std::fs::create_dir_all(&sibling).unwrap();
        let root_canon = base.canonicalize().unwrap();
        let child_canon = child.canonicalize().unwrap();
        let sibling_canon = sibling.canonicalize().unwrap();
        assert!(path_is_under(&root_canon, &child_canon));
        assert!(path_is_under(&root_canon, &root_canon));
        // On case-insensitive filesystems (Windows, default APFS) an
        // uppercased root still canonicalizes to the same directory; on
        // case-sensitive filesystems the uppercased path does not exist
        // and canonicalize fails — either way containment holds.
        let upper = PathBuf::from(root_canon.to_string_lossy().to_uppercase());
        if upper.canonicalize().is_ok() {
            assert!(path_is_under(&upper, &child_canon));
        }
        assert!(!path_is_under(&root_canon, &sibling_canon));
        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::remove_dir_all(&sibling);
    }

    #[test]
    fn containment_rejects_case_variant_siblings_on_case_sensitive_fs() {
        let pid = std::process::id();
        let lower = std::env::temp_dir().join(format!("ft_casevar_{pid}"));
        let upper = std::env::temp_dir().join(format!("FT_CASEVAR_{pid}"));
        let _ = std::fs::remove_dir_all(&lower);
        let _ = std::fs::remove_dir_all(&upper);
        std::fs::create_dir_all(&lower).unwrap();
        std::fs::create_dir_all(&upper).unwrap();
        let lower_canon = lower.canonicalize().unwrap();
        let upper_canon = upper.canonicalize().unwrap();
        // On a case-sensitive filesystem the two paths are DIFFERENT
        // directories, so the case-variant sibling must fail containment
        // (the old ASCII case-folded prefix compare would have passed it).
        if lower_canon != upper_canon {
            assert!(!path_is_under(&lower_canon, &upper_canon));
        }
        let _ = std::fs::remove_dir_all(&lower);
        let _ = std::fs::remove_dir_all(&upper);
    }

    #[test]
    fn selection_file_only_takes_indexed_vid_ids() {
        let base = std::env::temp_dir().join(format!("ft_sel_case_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("db")).unwrap();
        let index = r#"{"schema_version":3,"case_id":"FT-1","videos":[
            {"id":"vid_000001","ffprobe":{"streams":[{"id":"0x1","codec_type":"video"}]}},
            {"id":"vid_000002","ffprobe":{"streams":[{"id":"0x2","codec_type":"video"}]}}
        ]}"#;
        std::fs::write(base.join("db/video_index.json"), index).unwrap();
        let out = base.join("selection-all.json");
        let count = build_selection_file(&base, &out).unwrap();
        assert_eq!(count, 2);
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(content.contains("\"selector\":\"vid_000001\""));
        assert!(content.contains("\"selector\":\"vid_000002\""));
        assert!(!content.contains("0x1"));
        assert!(content.contains("\"action\":\"validate\""));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn selection_file_ignores_id_literals_inside_string_values() {
        let base = std::env::temp_dir().join(format!("ft_sel_embed_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("db")).unwrap();
        // A `"id":"vid_..."` literal inside a string value (ffprobe error
        // text quoting JSON) must not produce a phantom selector — the old
        // substring scan would have collected vid_evil.
        let index = r#"{"schema_version":3,"videos":[
            {"id":"vid_000001","ffprobe_error":"bad blob {\"id\":\"vid_evil\"} here"},
            {"id":"vid_000002"}
        ]}"#;
        std::fs::write(base.join("db/video_index.json"), index).unwrap();
        let out = base.join("selection-all.json");
        let count = build_selection_file(&base, &out).unwrap();
        assert_eq!(count, 2);
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(content.contains("\"selector\":\"vid_000001\""));
        assert!(content.contains("\"selector\":\"vid_000002\""));
        assert!(!content.contains("vid_evil"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn selection_file_errors_without_videos() {
        let base = std::env::temp_dir().join(format!("ft_sel_empty_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("db")).unwrap();
        std::fs::write(base.join("db/video_index.json"), r#"{"videos":[]}"#).unwrap();
        let out = base.join("selection-all.json");
        assert!(build_selection_file(&base, &out).is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn origin_gate_requires_exact_loopback_authority() {
        let request = |origin: Option<&str>, host: Option<&str>| Request {
            method: "POST".into(),
            path: "/api/start".into(),
            query: String::new(),
            body: String::new(),
            range: None,
            origin: origin.map(str::to_string),
            host: host.map(str::to_string),
        };
        // Prefix-spoofed authorities that the old starts_with gate let
        // through: a loopback literal as a subdomain prefix, as userinfo,
        // or under a non-http scheme must all be rejected.
        for origin in [
            "http://127.0.0.1.evil.com",
            "http://127.0.0.1@evil.example",
            "http://localhost.attacker.io",
            "https://127.0.0.1",
            "null",
        ] {
            assert!(
                !request_is_localhost_trusted(&request(Some(origin), Some("127.0.0.1"))),
                "origin {origin} must be rejected"
            );
        }
        // Exact loopback hosts with an optional port stay accepted.
        for origin in [
            "http://127.0.0.1",
            "http://127.0.0.1:8477",
            "http://localhost",
            "http://localhost:3000",
            "http://[::1]",
            "http://[::1]:9000",
        ] {
            assert!(
                request_is_localhost_trusted(&request(Some(origin), Some("127.0.0.1"))),
                "origin {origin} must be trusted"
            );
        }
        // Local tools send no Origin at all; the Host gate still applies.
        assert!(request_is_localhost_trusted(&request(
            None,
            Some("127.0.0.1")
        )));
        assert!(!request_is_localhost_trusted(&request(
            None,
            Some("evil.example")
        )));
    }

    /// Helper child for `run_step_drains_large_child_output`: re-runs this
    /// test binary filtered to `spew_stderr_helper`, which floods stderr
    /// well past the OS pipe buffer.
    #[test]
    fn spew_stderr_helper() {
        if std::env::var("FRAMETRACE_TEST_SPEW").as_deref() != Ok("1") {
            return;
        }
        for index in 0..8000 {
            eprintln!("spew line {index}: {}", "x".repeat(80));
        }
    }

    /// Regression: a pipeline child writing more than the ~64 KiB pipe
    /// buffer to stderr must not deadlock the workstation. Before the
    /// reader threads, try_wait() polled forever while the child blocked
    /// on a full pipe — this test would hang instead of failing.
    #[test]
    fn run_step_drains_large_child_output() {
        let exe = std::env::current_exe().unwrap();
        let state: SharedState = Arc::new(Mutex::new(JobState::new()));
        unsafe {
            std::env::set_var("FRAMETRACE_TEST_SPEW", "1");
        }
        let result = run_step(
            &exe,
            &[
                "serve::tests::spew_stderr_helper".to_string(),
                "--exact".to_string(),
                "--nocapture".to_string(),
            ],
            &state,
        );
        unsafe {
            std::env::remove_var("FRAMETRACE_TEST_SPEW");
        }
        let text = result.expect("spewing child must complete, not deadlock");
        assert!(text.contains("spew line"), "{text}");
        // Retained output stays bounded at the tail cap even though the
        // child emitted ~640 KiB.
        assert!(
            text.len() <= STEP_OUTPUT_TAIL_BYTES + 4096,
            "retained output {} exceeded tail cap",
            text.len()
        );
    }

    #[test]
    fn drain_capped_retains_only_the_tail() {
        let input: Vec<u8> = (0..(STEP_OUTPUT_TAIL_BYTES * 2))
            .map(|index| (index % 251) as u8)
            .collect();
        let drained = drain_capped(&mut input.as_slice());
        assert_eq!(drained.len(), STEP_OUTPUT_TAIL_BYTES);
        assert_eq!(
            drained.as_slice(),
            &input[input.len() - STEP_OUTPUT_TAIL_BYTES..]
        );
    }

    #[test]
    fn mimes_map_to_expected_types() {
        assert_eq!(mime_for(Path::new("a.html")), "text/html; charset=utf-8");
        assert_eq!(mime_for(Path::new("b.MP4")), "video/mp4");
        assert_eq!(mime_for(Path::new("c.unknown")), "application/octet-stream");
    }

    #[test]
    fn server_answers_status_env_and_guards() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let addr = listener.local_addr().unwrap();
        let state: SharedState = Arc::new(Mutex::new(JobState::new()));
        thread::spawn(move || serve_on(listener, state));
        let response = |request: &str| {
            let mut stream = TcpStream::connect(addr).unwrap();
            stream.write_all(request.as_bytes()).unwrap();
            let mut buffer = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                let read = stream.read(&mut chunk).unwrap();
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..read]);
            }
            String::from_utf8_lossy(&buffer).to_string()
        };
        let status = response("GET /api/status HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        assert!(status.contains("200 OK"));
        assert!(status.contains("\"has_job\":false"));
        assert!(status.contains("\"phase\":\"idle\""));
        let env = response("GET /api/env HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        assert!(env.contains("\"ok\":true"));
        assert!(env.contains("\"ffmpeg\":"));
        let review = response("GET /review/nope.html HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        assert!(review.contains("404"));
        let media = response("GET /media HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        assert!(media.contains("400"));
        // DNS-rebinding gate: a non-loopback Host must be rejected on every
        // route, including the evidence-streaming endpoint.
        let rebinding = response("GET /api/status HTTP/1.1\r\nHost: attacker.example\r\n\r\n");
        assert!(rebinding.contains("403"), "{rebinding}");
        // Cross-site form POST: a foreign Origin on a state-changing route
        // must be rejected without a preflight.
        let csrf = response(
            "POST /api/open-folder HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: http://evil.example\r\nContent-Type: text/plain\r\nContent-Length: 23\r\n\r\n{\"path\":\"C:\\\\evil.exe\"}",
        );
        assert!(csrf.contains("403"), "{csrf}");
        let traversal = response(
            "POST /api/start HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: 33\r\n\r\n{\"source_path\":\"C:\\nope\\missing\"}",
        );
        assert!(traversal.contains("\"ok\":false"));
    }

    #[test]
    fn examiner_page_auto_detects_image_extension() {
        // Regression guard: the page must switch inputKind to "e01" when the
        // pasted source path ends in a forensic-image extension, so an E01 is
        // never routed into the folder pipeline by a stale radio selection.
        assert!(EXAMINER_PAGE.contains("looksImage"));
        assert!(EXAMINER_PAGE.contains("\\.(e01|ex01|l01|s01|e02|ex02|l02|s02)$"));
        assert!(EXAMINER_PAGE.contains("input[name='inputKind'][value='e01']"));
    }
}
