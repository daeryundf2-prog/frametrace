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
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const EXAMINER_PAGE: &str = include_str!("../assets/examiner_app.html");
// Bounded so a malformed client cannot exhaust memory; export payloads
// carry one small JSON record per selected item.
const MAX_BODY: usize = 4 * 1024 * 1024;

pub struct ServeOptions {
    pub case_dir: Option<PathBuf>,
    pub port: Option<u16>,
}

#[derive(Clone, Copy, PartialEq)]
enum InputKind {
    Folder,
    E01,
    /// Same evidence kind as E01 but triaged directly on the segment set.
    E01Direct,
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
            InputKind::E01Direct => &[
                "케이스 준비",
                "E01 메타데이터 (ewfinfo)",
                "파일시스템 조사 (export 생략)",
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
    /// Triage mode: inspect-e01 --filesystem reads the segment set
    /// directly instead of exporting hundreds of GiB to raw first.
    e01_direct: bool,
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
        ("GET", "/api/env") => json(api_env(request)),
        ("GET", "/api/browse") => json(api_browse(request)),
        ("GET", "/api/status") => json(api_status(state)),
        ("POST", "/api/start") => json(api_start(request, state)),
        ("POST", "/api/finalize") => json(api_finalize(state)),
        ("POST", "/api/recover-deleted") => json(api_recover_deleted(state)),
        ("POST", "/api/export-selected") => json(api_export_selected(request, state)),
        ("POST", "/api/verify-audit") => json(api_verify_audit(state)),
        ("POST", "/api/cancel") => json(api_cancel(state)),
        ("POST", "/api/open-case") => json(api_open_case(request, state)),
        ("POST", "/api/import-marks") => json(api_import_marks(request, state)),
        ("POST", "/api/capture-frame") => json(api_capture_frame(request, state)),
        ("POST", "/api/export-clip") => json(api_export_clip(request, state)),
        ("POST", "/api/proxy") => json(api_proxy(request, state)),
        ("POST", "/api/advanced") => json(api_advanced(request, state)),
        ("POST", "/api/carve") => json(api_carve(state)),
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

/// The env probe spawns every probed binary with a version flag, which
/// costs seconds on a cold call — the page's header badges used to stay
/// empty for the whole probe window and read as "broken". Tool presence
/// does not change during a workstation session, so the result is cached
/// after the first probe; `?refresh=1` forces a re-probe after the
/// examiner installs a missing tool.
fn api_env(request: &Request) -> String {
    static ENV_CACHE: Mutex<Option<String>> = Mutex::new(None);
    let refresh = query_value(&request.query, "refresh").as_deref() == Some("1");
    let mut guard = ENV_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if refresh || guard.is_none() {
        *guard = Some(probe_env());
    }
    guard
        .clone()
        .unwrap_or_else(|| "{\"ok\":false}".to_string())
}

fn probe_env() -> String {
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

/// Directory listing for the in-app path picker. An empty `path` returns
/// drive roots; a directory returns its children — subdirectories always,
/// first-segment E01-family files only when `files=e01`. Existence and
/// entry type are always reported, so the same endpoint also validates
/// paths typed directly into the form.
fn api_browse(request: &Request) -> String {
    let raw = query_value(&request.query, "path").unwrap_or_default();
    let want_files = query_value(&request.query, "files").as_deref() == Some("e01");
    browse_json(raw.trim(), want_files)
}

fn browse_json(raw: &str, want_files: bool) -> String {
    if raw.is_empty() {
        let entries = drive_roots()
            .iter()
            .map(|root| browse_entry_json(root, root, true, false))
            .collect::<Vec<_>>()
            .join(",");
        return format!(
            "{{\"ok\":true,\"exists\":true,\"is_dir\":true,\"path\":\"\",\"entries\":[{entries}]}}"
        );
    }
    let path = PathBuf::from(raw);
    let display = path.display().to_string();
    let Ok(meta) = std::fs::metadata(&path) else {
        return format!(
            "{{\"ok\":true,\"exists\":false,\"path\":{}}}",
            json_string(&display)
        );
    };
    if meta.is_file() {
        return format!(
            "{{\"ok\":true,\"exists\":true,\"is_file\":true,\"is_dir\":false,\"path\":{},\"name\":{}}}",
            json_string(&display),
            json_string(
                &path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ),
        );
    }
    let read = match std::fs::read_dir(&path) {
        Ok(read) => read,
        Err(error) => {
            return format!(
                "{{\"ok\":false,\"exists\":true,\"is_dir\":true,\"path\":{},\"error\":{}}}",
                json_string(&display),
                json_string(&format!("폴더를 읽을 수 없습니다: {error}")),
            );
        }
    };
    // Cap keeps the picker responsive on huge directories; the status
    // line tells the examiner the listing was truncated.
    const MAX_ENTRIES: usize = 500;
    let mut dirs: Vec<(String, String)> = Vec::new();
    let mut files: Vec<(String, String)> = Vec::new();
    let mut truncated = false;
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        let shown = is_dir || (want_files && is_e01_first_segment(&name));
        if !shown {
            continue;
        }
        if dirs.len() + files.len() >= MAX_ENTRIES {
            truncated = true;
            break;
        }
        let full = path.join(&name).display().to_string();
        if is_dir {
            dirs.push((name, full));
        } else {
            files.push((name, full));
        }
    }
    let by_name =
        |a: &(String, String), b: &(String, String)| a.0.to_lowercase().cmp(&b.0.to_lowercase());
    dirs.sort_by(by_name);
    files.sort_by(by_name);
    let entries = dirs
        .iter()
        .map(|(name, full)| {
            // A directory holding db/case.db is an openable case — the
            // picker surfaces it as one-click openable.
            let is_case = crate::case_db::case_db_path(Path::new(full)).is_file();
            browse_entry_json(name, full, true, is_case)
        })
        .chain(
            files
                .iter()
                .map(|(name, full)| browse_entry_json(name, full, false, false)),
        )
        .collect::<Vec<_>>()
        .join(",");
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| format!("\"parent\":{},", json_string(&parent.display().to_string())))
        .unwrap_or_default();
    let is_case = crate::case_db::case_db_path(&path).is_file();
    format!(
        "{{\"ok\":true,\"exists\":true,\"is_dir\":true,\"is_case\":{},\"path\":{},{parent}\"entries\":[{entries}],\"truncated\":{}}}",
        json_bool(is_case),
        json_string(&display),
        json_bool(truncated),
    )
}

fn browse_entry_json(name: &str, path: &str, dir: bool, is_case: bool) -> String {
    format!(
        "{{\"name\":{},\"path\":{},\"dir\":{},\"is_case\":{}}}",
        json_string(name),
        json_string(path),
        json_bool(dir),
        json_bool(is_case)
    )
}

/// `true` for the first segment of a split forensic image family. The
/// examiner picks `.E01`/`.Ex01`/`.L01`/`.S01`; later segments (.E02…)
/// are opened implicitly by libewf, so the picker hides them.
fn is_e01_first_segment(name: &str) -> bool {
    matches!(
        name.rsplit('.')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "e01" | "ex01" | "l01" | "s01"
    )
}

#[cfg(windows)]
fn drive_roots() -> Vec<String> {
    (b'A'..=b'Z')
        .map(|letter| format!("{}:\\", letter as char))
        .filter(|root| Path::new(root).exists())
        .collect()
}

#[cfg(not(windows))]
fn drive_roots() -> Vec<String> {
    vec!["/".to_string()]
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
    let e01_direct = body_value(&request.body, "e01_direct").as_deref() == Some("true");
    let input_kind = match body_value(&request.body, "input_kind").as_deref() {
        Some("e01") if e01_direct => InputKind::E01Direct,
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
    if matches!(input_kind, InputKind::E01 | InputKind::E01Direct)
        && !tool_available("ewfinfo", "-V")
    {
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
        e01_direct,
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
            InputKind::E01 | InputKind::E01Direct => vec![case_dir.clone()],
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
        InputKind::E01 | InputKind::E01Direct => run_e01_pipeline(state, job),
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

    if job.e01_direct {
        // Triage mode: a single inspect-e01 --filesystem run does ewfinfo
        // metadata plus mmls/fls straight off the segment set — no raw
        // export, so even a ~1 TB image is reviewable in minutes.
        set_step(&state, 1, StepStatus::Running);
        match run_step(
            &exe,
            &[
                "inspect-e01".into(),
                case_text.clone(),
                source_text.clone(),
                "--filesystem".into(),
            ],
            &state,
        ) {
            Ok(output) => {
                log(&state, output);
                set_step(&state, 1, StepStatus::Done);
                set_step(&state, 2, StepStatus::Done);
            }
            Err(err) => {
                set_step(&state, 1, StepStatus::Failed);
                fail(&state, &err);
                return;
            }
        }
        run_e01_pipeline_tail(state, job);
        return;
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

    run_e01_pipeline_tail(state, job);
}

/// Shared E01 tail: refresh the logical index over the case evidence
/// tree, run the non-fatal anomaly scan, then emit the review bundle.
fn run_e01_pipeline_tail(state: SharedState, job: PipelineJob) {
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
                "검토 화면이 준비되었습니다. 삭제 영상 후보는 '삭제 영상 후보 복구' 버튼으로, 카빙·개별 inode 복구는 CLI(carve-file/recover-inode)로 수행한 뒤 재검토하십시오.".into(),
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

/// `POST /api/capture-frame`: store an examiner-captured video frame as a
/// case artifact. The viewer sends the canvas JPEG base64; we verify the
/// magic bytes, hash the capture, and chain an audit event so the frame is
/// traceable back to the record and playback position.
fn api_capture_frame(request: &Request, state: &SharedState) -> String {
    let case_dir = {
        let guard = state_lock(state);
        guard.case_dir.clone()
    };
    let Some(case_dir) = case_dir else {
        return "{\"ok\":false,\"error\":\"먼저 케이스를 열거나 분석을 실행하십시오.\"}"
            .to_string();
    };
    let id = body_value(&request.body, "id").unwrap_or_default();
    let image_b64 = body_value(&request.body, "image").unwrap_or_default();
    if image_b64.is_empty() || image_b64.len() > MAX_BODY {
        return "{\"ok\":false,\"error\":\"프레임 이미지가 비었거나 너무 큽니다.\"}".to_string();
    }
    let Some(bytes) = crate::audit_key::base64_decode(&image_b64) else {
        return "{\"ok\":false,\"error\":\"이미지 디코딩에 실패했습니다.\"}".to_string();
    };
    if !bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return "{\"ok\":false,\"error\":\"JPEG 프레임이 아닙니다.\"}".to_string();
    }
    let stamp = match crate::util::now_unix() {
        Ok(stamp) => stamp,
        Err(err) => return format!("{{\"ok\":false,\"error\":{}}}", json_string(&err)),
    };
    let dir = case_dir.join("artifacts/captures");
    if let Err(err) = std::fs::create_dir_all(&dir) {
        return format!(
            "{{\"ok\":false,\"error\":{}}}",
            json_string(&err.to_string())
        );
    }
    let target = crate::util::unique_available_path(&dir.join(format!(
        "{}__{}.jpg",
        export_safe_name(&id),
        stamp
    )));
    if let Err(err) = std::fs::write(&target, &bytes) {
        return format!(
            "{{\"ok\":false,\"error\":{}}}",
            json_string(&err.to_string())
        );
    }
    let digest = audit::digest_file(&target).unwrap_or_default();
    let rel = target
        .strip_prefix(&case_dir)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| target.to_string_lossy().to_string());
    let time = body_value(&request.body, "time").unwrap_or_default();
    let _ = audit::append_chained_jsonl(
        &case_dir.join("evidence/logs/capture-log.jsonl"),
        &format!(
            "{{\"event\":\"frame-capture\",\"record_id\":{},\"output\":{},\"sha256\":{},\"video_time_seconds\":{},\"unix\":{stamp}}}",
            json_string(&id),
            json_string(&rel),
            json_string(&digest),
            json_string(&time)
        ),
    );
    format!(
        "{{\"ok\":true,\"path\":{},\"sha256\":{}}}",
        json_string(&rel),
        json_string(&digest)
    )
}

/// `POST /api/export-clip`: export the viewer's in/out range of one record
/// through `export-video`, producing a deliverable clip under
/// artifacts/clips/ with its own chained export-log entry.
fn api_export_clip(request: &Request, state: &SharedState) -> String {
    let (case_dir, busy) = {
        let guard = state_lock(state);
        (guard.case_dir.clone(), guard.busy)
    };
    let Some(case_dir) = case_dir else {
        return "{\"ok\":false,\"error\":\"먼저 케이스를 열거나 분석을 실행하십시오.\"}"
            .to_string();
    };
    if busy {
        return "{\"ok\":false,\"error\":\"분석 작업이 진행 중입니다 — 완료 후 다시 시도하십시오.\"}".to_string();
    }
    let id = body_value(&request.body, "id").unwrap_or_default();
    let start = body_value(&request.body, "start")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(-1.0);
    let duration = body_value(&request.body, "duration")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(-1.0);
    // NaN/negative/parse failures are all rejected here.
    if id.trim().is_empty()
        || !start.is_finite()
        || start < 0.0
        || !duration.is_finite()
        || duration <= 0.0
    {
        return "{\"ok\":false,\"error\":\"구간 값이 올바르지 않습니다 (id, start, duration).\"}"
            .to_string();
    }
    // export-video resolves vid_* ids and indexed source paths. Carved /
    // filesystem records carry non-indexed ids (carve_*, fls_*) — for
    // those the viewer also sends the record's file path, which we only
    // honor when it sits inside the approved media roots.
    let selector = if id.starts_with("vid_") {
        id.clone()
    } else {
        let alt = body_value(&request.body, "path").unwrap_or_default();
        let candidate = PathBuf::from(alt.trim());
        let roots = state_lock(state).media_roots.clone();
        let allowed = candidate
            .canonicalize()
            .map(|canonical| {
                roots
                    .iter()
                    .filter_map(|root| root.canonicalize().ok())
                    .any(|root| path_is_under(&root, &canonical))
            })
            .unwrap_or(false);
        if !allowed {
            return "{\"ok\":false,\"error\":\"색인된 영상 또는 허용된 경로의 파일만 클립으로보낼 수 있습니다.\"}"
                .to_string();
        }
        candidate.to_string_lossy().to_string()
    };
    let dir = case_dir.join("artifacts/clips");
    if let Err(err) = std::fs::create_dir_all(&dir) {
        return format!(
            "{{\"ok\":false,\"error\":{}}}",
            json_string(&err.to_string())
        );
    }
    let output = crate::util::unique_available_path(&dir.join(format!(
        "{}__{}s+{}s.mp4",
        export_safe_name(&id),
        start as u64,
        duration as u64
    )));
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
    let args = vec![
        "export-video".into(),
        case_text,
        selector,
        "--format".into(),
        "mp4".into(),
        "--start".into(),
        format!("{start:.3}"),
        "--duration".into(),
        format!("{duration:.3}"),
        "--output".into(),
        output.to_string_lossy().to_string(),
    ];
    match run_step(&exe, &args, state) {
        Ok(_) => {
            let rel = output
                .strip_prefix(&case_dir)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| output.to_string_lossy().to_string());
            format!("{{\"ok\":true,\"path\":{}}}", json_string(&rel))
        }
        Err(err) => format!("{{\"ok\":false,\"error\":{}}}", json_string(&err)),
    }
}

/// `POST /api/proxy`: return (or lazily generate via `make-proxy`) the
/// review proxy for an indexed video so the viewer can offer smooth
/// playback of heavy originals without forcing a full proxy pass.
fn api_proxy(request: &Request, state: &SharedState) -> String {
    let (case_dir, busy) = {
        let guard = state_lock(state);
        (guard.case_dir.clone(), guard.busy)
    };
    let Some(case_dir) = case_dir else {
        return "{\"ok\":false,\"error\":\"먼저 케이스를 열거나 분석을 실행하십시오.\"}"
            .to_string();
    };
    if busy {
        return "{\"ok\":false,\"error\":\"분석 작업이 진행 중입니다 — 완료 후 다시 시도하십시오.\"}".to_string();
    }
    let id = body_value(&request.body, "id").unwrap_or_default();
    if id.trim().is_empty() {
        return "{\"ok\":false,\"error\":\"증거 id가 비어 있습니다.\"}".to_string();
    }
    // make-proxy resolves vid_* ids and indexed source paths. Carved /
    // filesystem records carry non-indexed ids (carve_*, fls_*) — for
    // those the viewer also sends the record's file path, which we only
    // honor when it sits inside the approved media roots.
    let selector = if id.starts_with("vid_") {
        id.clone()
    } else {
        let alt = body_value(&request.body, "path").unwrap_or_default();
        let candidate = PathBuf::from(alt.trim());
        let roots = state_lock(state).media_roots.clone();
        let allowed = candidate
            .canonicalize()
            .map(|canonical| {
                roots
                    .iter()
                    .filter_map(|root| root.canonicalize().ok())
                    .any(|root| path_is_under(&root, &canonical))
            })
            .unwrap_or(false);
        if !allowed {
            return "{\"ok\":false,\"error\":\"색인된 영상 또는 허용된 경로의 파일만 프록시로 재생할 수 있습니다.\"}"
                .to_string();
        }
        candidate.to_string_lossy().to_string()
    };
    let dir = case_dir.join("artifacts/proxies");
    let find_existing = |dir: &Path| -> Option<PathBuf> {
        // generate_proxy names outputs with video_export::sanitize_filename,
        // not export_safe_name — match that or the cache lookup misses.
        let prefix = format!(
            "{}_proxy_",
            crate::video_export::sanitize_filename(&selector)
        );
        std::fs::read_dir(dir).ok()?.flatten().find_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(&prefix) && name.ends_with(".mp4") {
                Some(entry.path())
            } else {
                None
            }
        })
    };
    let proxy = match find_existing(&dir) {
        Some(path) => path,
        None => {
            let exe = match std::env::current_exe() {
                Ok(exe) => exe,
                Err(err) => {
                    return format!(
                        "{{\"ok\":false,\"error\":{}}}",
                        json_string(&err.to_string())
                    );
                }
            };
            let args = vec![
                "make-proxy".into(),
                case_dir.to_string_lossy().to_string(),
                selector.clone(),
            ];
            if let Err(err) = run_step(&exe, &args, state) {
                return format!("{{\"ok\":false,\"error\":{}}}", json_string(&err));
            }
            match find_existing(&dir) {
                Some(path) => path,
                None => {
                    return "{\"ok\":false,\"error\":\"프록시 생성 후 파일을 찾지 못했습니다.\"}"
                        .to_string();
                }
            }
        }
    };
    format!(
        "{{\"ok\":true,\"path\":{}}}",
        json_string(proxy.to_string_lossy().as_ref())
    )
}

/// `POST /api/advanced`: run a CLI-only case tool from the workstation.
/// `tool` is an allow-listed name; `extra` is an optional path argument
/// (hash list for known-hash, peer case dir for merge/compare) that is
/// validated to exist — never a free-form command line.
fn api_advanced(request: &Request, state: &SharedState) -> String {
    let (case_dir, busy) = {
        let guard = state_lock(state);
        (guard.case_dir.clone(), guard.busy)
    };
    let Some(case_dir) = case_dir else {
        return "{\"ok\":false,\"error\":\"먼저 케이스를 열거나 분석을 실행하십시오.\"}"
            .to_string();
    };
    if busy {
        return "{\"ok\":false,\"error\":\"분석 작업이 진행 중입니다 — 완료 후 다시 시도하십시오.\"}".to_string();
    }
    let tool = body_value(&request.body, "tool").unwrap_or_default();
    let extra = body_value(&request.body, "extra").unwrap_or_default();
    let case_text = case_dir.to_string_lossy().to_string();
    let (args, output_rel): (Vec<String>, &str) = match tool.as_str() {
        "dfxml" => (
            vec!["export-dfxml".into(), case_text],
            "reports/case-index.dfxml",
        ),
        "timeline" => (vec!["timeline".into(), case_text], "db/timeline.jsonl"),
        "qa-consistency" => (
            vec!["qa".into(), "consistency".into(), case_text],
            "reports/qa/consistency-report.html",
        ),
        "qa-defense" => (
            vec!["qa".into(), "report-defense".into(), case_text],
            "reports/qa/report-defense-checklist.md",
        ),
        "known-hash" => {
            if extra.trim().is_empty() || !PathBuf::from(&extra).is_file() {
                return "{\"ok\":false,\"error\":\"sha256 해시 목록 파일 경로를 지정하십시오 (한 줄에 하나, 또는 'sha256,설명').\"}".to_string();
            }
            (
                vec!["known-hash-filter".into(), case_text, extra],
                "reports/known-hash-filter.json",
            )
        }
        "compare" | "merge" => {
            if extra.trim().is_empty() || !crate::case_db::case_db_path(Path::new(&extra)).is_file()
            {
                return "{\"ok\":false,\"error\":\"비교/병합할 다른 케이스 폴더를 지정하십시오 (db/case.db 포함).\"}".to_string();
            }
            let sub = if tool == "compare" {
                "compare-cases"
            } else {
                "merge-cases"
            };
            (
                vec![sub.into(), case_text, extra],
                "reports/case-compare.json",
            )
        }
        _ => {
            return "{\"ok\":false,\"error\":\"지원하지 않는 도구입니다.\"}".to_string();
        }
    };
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(err) => {
            return format!(
                "{{\"ok\":false,\"error\":{}}}",
                json_string(&err.to_string())
            );
        }
    };
    match run_step(&exe, &args, state) {
        Ok(out) => {
            let url = if case_dir.join(output_rel).is_file() {
                output_rel.to_string()
            } else {
                String::new()
            };
            let tail: Vec<&str> = out.lines().rev().take(6).collect();
            let mut tail = tail;
            tail.reverse();
            format!(
                "{{\"ok\":true,\"url\":{},\"output\":{}}}",
                json_string(&url),
                json_string(&tail.join("\n"))
            )
        }
        Err(err) => format!("{{\"ok\":false,\"error\":{}}}", json_string(&err)),
    }
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

/// Newest `db/filesystem/tsk-inspection-*.json` summary — carries the
/// image path and auto-selected partition offset the recover step needs.
fn newest_tsk_inspection(case_dir: &Path) -> Option<(PathBuf, u64)> {
    let path = std::fs::read_dir(case_dir.join("db/filesystem"))
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("tsk-inspection-") && name.ends_with(".json"))
        })
        .max()?;
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let image = value.get("image_path")?.as_str()?;
    let offset = value.get("partition_offset")?.as_u64()?;
    Some((PathBuf::from(image), offset))
}

/// `POST /api/recover-deleted`: run recover-batch --deleted-videos on the
/// image from the latest filesystem inspection, then regenerate the
/// review bundle so the recovered items appear in the viewer.
fn api_recover_deleted(state: &SharedState) -> String {
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
    let Some((image_path, partition_offset)) = newest_tsk_inspection(&case_dir) else {
        return "{\"ok\":false,\"error\":\"파일시스템 조사 결과가 없습니다 — 먼저 E01/이미지 분석을 실행하십시오.\"}"
            .to_string();
    };
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
        guard.logs.push(format!(
            "삭제 영상 후보 복구 중… ({} @ 오프셋 {partition_offset})",
            image_path.display()
        ));
    }
    let case_text = case_dir.to_string_lossy().to_string();
    let recover = run_step(
        &exe,
        &[
            "recover-batch".into(),
            case_text.clone(),
            image_path.to_string_lossy().to_string(),
            "--deleted-videos".into(),
            "--partition-offset".into(),
            partition_offset.to_string(),
        ],
        state,
    );
    let review = if recover.is_ok() {
        run_step(&exe, &["make-review".into(), case_text.clone()], state)
    } else {
        recover.clone()
    };
    let mut guard = state_lock(state);
    match (recover, review) {
        (Ok(recover_out), Ok(_)) => {
            guard.phase = "review-ready";
            guard.busy = false;
            guard.logs.push(recover_out.clone());
            guard
                .logs
                .push("복구 완료 — 뷰어를 새로 열면 복구된 항목이 보입니다.".into());
            "{\"ok\":true}".to_string()
        }
        (recover, review) => {
            let error = recover.err().or_else(|| review.err()).unwrap_or_default();
            guard.phase = "review-ready";
            guard.busy = false;
            guard.error = Some(error.clone());
            format!("{{\"ok\":false,\"error\":{}}}", json_string(&error))
        }
    }
}

/// `POST /api/carve`: run bounded signature carving over the case's
/// exported raw image (evidence/images/evidence.raw), then regenerate the
/// review bundle so carved candidates show up in the viewer. Requires the
/// full E01 pipeline (direct triage never exports a raw image).
fn api_carve(state: &SharedState) -> String {
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
    let raw = case_dir.join("evidence/images/evidence.raw");
    if !raw.is_file() {
        return "{\"ok\":false,\"error\":\"카빙할 raw 이미지가 없습니다 — E01을 '빠른 검토' 없이 분석(전체 export)하면 사용할 수 있습니다.\"}".to_string();
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
        guard
            .logs
            .push(format!("시그니처 카빙 실행 중… ({})", raw.display()));
    }
    let case_text = case_dir.to_string_lossy().to_string();
    let carve = run_step(
        &exe,
        &[
            "carve-file".into(),
            case_text.clone(),
            raw.to_string_lossy().to_string(),
        ],
        state,
    );
    let review = if carve.is_ok() {
        run_step(&exe, &["make-review".into(), case_text.clone()], state)
    } else {
        carve.clone()
    };
    let mut guard = state_lock(state);
    match (carve, review) {
        (Ok(carve_out), Ok(_)) => {
            guard.phase = "review-ready";
            guard.busy = false;
            guard.logs.push(carve_out.clone());
            guard
                .logs
                .push("카빙 완료 — 뷰어를 새로 열면 카빙 후보가 보입니다.".into());
            "{\"ok\":true}".to_string()
        }
        (carve, review) => {
            let error = carve.err().or_else(|| review.err()).unwrap_or_default();
            guard.phase = "review-ready";
            guard.busy = false;
            guard.error = Some(error.clone());
            format!("{{\"ok\":false,\"error\":{}}}", json_string(&error))
        }
    }
}

/// Copies reviewer-selected evidence into an organized handoff folder:
/// `case/exports/selection-<unix>/` holds the files plus a hash manifest
/// (CSV + JSONL + README) — the material package an examiner hands to a
/// requester. Only paths under the approved media roots are copied, and
/// items without a file (e.g. pre-recovery candidates) are recorded in
/// the manifest as skipped instead of silently vanishing.
fn api_export_selected(request: &Request, state: &SharedState) -> String {
    let (case_dir, roots) = {
        let guard = state_lock(state);
        (guard.case_dir.clone(), guard.media_roots.clone())
    };
    let Some(case_dir) = case_dir else {
        return "{\"ok\":false,\"error\":\"열린 케이스가 없습니다.\"}".to_string();
    };
    let Ok(body) = serde_json::from_str::<serde_json::Value>(&request.body) else {
        return "{\"ok\":false,\"error\":\"invalid export payload\"}".to_string();
    };
    let items = body
        .get("items")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    if items.is_empty() {
        return "{\"ok\":false,\"error\":\"선택된 항목이 없습니다.\"}".to_string();
    }
    if items.len() > 5000 {
        return "{\"ok\":false,\"error\":\"한 번에 최대 5000개 항목까지 내보낼 수 있습니다.\"}"
            .to_string();
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|span| span.as_secs())
        .unwrap_or(0);
    let export_dir =
        crate::util::unique_dir(&case_dir.join("exports").join(format!("selection-{stamp}")));
    if let Err(err) = std::fs::create_dir_all(&export_dir) {
        return format!(
            "{{\"ok\":false,\"error\":{}}}",
            json_string(&format!("내보내기 폴더 생성 실패: {err}"))
        );
    }
    let roots: Vec<PathBuf> = roots
        .iter()
        .filter_map(|root| root.canonicalize().ok())
        .collect();
    let field = |item: &serde_json::Value, key: &str| {
        item.get(key)
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string()
    };
    let joined = |item: &serde_json::Value, key: &str| {
        item.get(key)
            .and_then(|value| value.as_array())
            .map(|list| {
                list.iter()
                    .filter_map(|entry| entry.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .unwrap_or_default()
    };
    let mut csv_rows = vec![
        "id,exported_file,kind,status,mark,tags,sha256_source,sha256_copy,size_bytes,recorded_time,original_path,source_path,warnings,note".to_string(),
    ];
    let mut jsonl_rows: Vec<String> = Vec::new();
    let mut copied = 0usize;
    let mut skipped = 0usize;
    for item in &items {
        let id = field(item, "id");
        let path_text = field(item, "path");
        let mut exported = String::new();
        let mut copy_hash = String::new();
        let mut size = String::new();
        let mut skip_reason = String::new();
        if path_text.is_empty() {
            skip_reason = "no-file(pre-recovery candidate)".to_string();
        } else {
            match PathBuf::from(&path_text).canonicalize() {
                Err(_) => skip_reason = "source-missing".to_string(),
                Ok(canonical) if !roots.iter().any(|root| path_is_under(root, &canonical)) => {
                    skip_reason = "outside-approved-roots".to_string();
                }
                Ok(canonical) => {
                    let filename = canonical
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| field(item, "name"));
                    let target = crate::util::unique_available_path(&export_dir.join(format!(
                        "{}__{}",
                        export_safe_name(&id),
                        export_safe_name(&filename)
                    )));
                    match std::fs::copy(&canonical, &target) {
                        Ok(_) => {
                            exported = target
                                .file_name()
                                .map(|name| name.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            copy_hash = audit::digest_file(&target).unwrap_or_default();
                            size = std::fs::metadata(&target)
                                .map(|meta| meta.len().to_string())
                                .unwrap_or_default();
                            copied += 1;
                        }
                        Err(err) => skip_reason = format!("copy-failed: {err}"),
                    }
                }
            }
        }
        if !skip_reason.is_empty() {
            skipped += 1;
        }
        let base_warnings = joined(item, "warnings");
        let warnings = if skip_reason.is_empty() {
            base_warnings
        } else if base_warnings.is_empty() {
            skip_reason.clone()
        } else {
            format!("{base_warnings}; {skip_reason}")
        };
        csv_rows.push(csv_row(&[
            id.clone(),
            exported.clone(),
            field(item, "kind"),
            field(item, "status"),
            field(item, "mark"),
            joined(item, "tags"),
            field(item, "sha256"),
            copy_hash.clone(),
            size,
            field(item, "rec_time"),
            field(item, "original_path"),
            path_text.clone(),
            warnings,
            field(item, "note"),
        ]));
        let mut row = item.clone();
        row["exported_file"] = serde_json::Value::String(exported);
        row["sha256_copy"] = serde_json::Value::String(copy_hash);
        row["skipped"] = if skip_reason.is_empty() {
            serde_json::Value::Bool(false)
        } else {
            serde_json::Value::String(skip_reason)
        };
        jsonl_rows.push(row.to_string());
    }
    let audit_log = case_dir.join("evidence/logs/export-log.jsonl");
    let _ = audit::append_chained_jsonl(
        &audit_log,
        &format!(
            "{{\"event\":\"selection-export\",\"export_dir\":{},\"items\":{},\"copied\":{copied},\"skipped\":{skipped},\"unix\":{stamp}}}",
            json_string(&export_dir.to_string_lossy()),
            items.len()
        ),
    );
    let write = |name: &str, contents: String| -> Result<(), String> {
        std::fs::write(export_dir.join(name), contents)
            .map_err(|err| format!("manifest write failed for {name}: {err}"))
    };
    let head = audit::chain_head(&audit_log)
        .ok()
        .flatten()
        .unwrap_or_default();
    let result = write("manifest.csv", format!("\u{feff}{}\n", csv_rows.join("\n")))
        .and_then(|_| write("manifest.jsonl", jsonl_rows.join("\n") + "\n"))
        .and_then(|_| {
            write(
                "manifest-audit.json",
                format!(
                    "{{\"audit_log\":{},\"chain_head_sha256\":{},\"unix\":{stamp}}}\n",
                    json_string("evidence/logs/export-log.jsonl"),
                    json_string(&head)
                ),
            )
        })
        .and_then(|_| {
            write(
                "README.txt",
                format!(
                    "FrameTrace 선별 증거 묶음\n\n\
                     생성 시각(unix): {stamp}\n\
                     항목: {}개 — 복사 {copied}개, 제외 {skipped}개\n\n\
                     - 복사된 파일은 원본의 사본입니다. manifest의 sha256_copy가\n  \
                     sha256_source와 동일하면 복사가 정확히 이루어진 것입니다.\n\
                     - manifest.csv: Excel로 여는 항목별 메타데이터 (UTF-8 BOM).\n\
                     - manifest.jsonl: 같은 내용의 기계 판독용 JSONL.\n\
                     - manifest-audit.json: 이 묶음을 만든 감사 체인 상태\n  \
                     (chain_head_sha256가 케이스 export-log의 마지막 줄 해시와\n  \
                     일치하면 해당 시점 감사 기록에서 생성된 묶음입니다).\n\
                     - 복구 전 후보처럼 파일이 없는 항목은 복사되지 않고\n  \
                     manifest에 skipped 사유가 기록됩니다.\n",
                    items.len()
                ),
            )
        });
    if let Err(err) = result {
        return format!("{{\"ok\":false,\"error\":{}}}", json_string(&err));
    }
    format!(
        "{{\"ok\":true,\"export_dir\":{},\"copied\":{copied},\"skipped\":{skipped}}}",
        json_string(&export_dir.to_string_lossy())
    )
}

fn csv_row(fields: &[String]) -> String {
    fields
        .iter()
        .map(|field| {
            if field.contains([',', '"', '\n', '\r']) {
                format!("\"{}\"", field.replace('"', "\"\""))
            } else {
                field.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Windows-safe filename that keeps non-ASCII (e.g. Korean) evidence
/// names — only path separators, forbidden characters, and control
/// characters are replaced; trailing dots/spaces are trimmed.
fn export_safe_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|ch| {
            if matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || ch.is_control()
            {
                '_'
            } else {
                ch
            }
        })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches(['.', ' ']).to_string();
    if trimmed.is_empty() {
        "item".to_string()
    } else {
        trimmed
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
    // `?download=1` forces a browser save-as with the evidence filename
    // instead of inline playback. Same approved-roots containment applies.
    if query_value(&request.query, "download").is_some() {
        let name = canonical
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("evidence.bin");
        let safe: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        head.push_str(&format!(
            "Content-Disposition: attachment; filename=\"{safe}\"\r\n"
        ));
    }
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

    /// Every API response is hand-assembled via `format!` — the safety
    /// invariant is that all dynamic values pass `json_string`. This test
    /// feeds hostile characters (quotes, backslashes, newlines, unicode)
    /// through reachable response paths and asserts each still parses as
    /// valid JSON via serde_json.
    #[test]
    fn api_responses_stay_valid_json_with_hostile_strings() {
        let state: SharedState = Arc::new(Mutex::new(JobState::new()));
        let post = |path: &str, body: &str| Request {
            method: "POST".into(),
            path: path.into(),
            query: String::new(),
            body: body.into(),
            range: None,
            origin: None,
            host: None,
        };
        let parse = |label: &str, json: &str| -> serde_json::Value {
            serde_json::from_str(json)
                .unwrap_or_else(|err| panic!("{label} produced invalid JSON: {err}\n{json}"))
        };

        // Idle status + error paths with no case loaded.
        parse("api_status", &api_status(&state));
        parse("api_carve", &api_carve(&state));
        parse("api_recover_deleted", &api_recover_deleted(&state));
        parse(
            "api_export_selected",
            &api_export_selected(&post("/api/export-selected", "{}"), &state),
        );
        parse(
            "api_advanced(unknown)",
            &api_advanced(&post("/api/advanced", r#"{"tool":"nope"}"#), &state),
        );
        parse(
            "api_advanced(hostile extra)",
            &api_advanced(
                &post(
                    "/api/advanced",
                    r#"{"tool":"known-hash","extra":"C:\\evil \"quoted\" \\path"}"#,
                ),
                &state,
            ),
        );
        parse(
            "api_export_clip",
            &api_export_clip(
                &post("/api/export-clip", r#"{"id":"v\"1","start":0,"end":1}"#),
                &state,
            ),
        );
        parse(
            "api_proxy",
            &api_proxy(&post("/api/proxy", r#"{"id":"v\"1"}"#), &state),
        );

        // Logs/errors containing quotes and newlines still serialize.
        {
            let mut guard = state_lock(&state);
            guard.logs.push("line \"one\"\nline \\ two".into());
            guard.error = Some("error: \"bad\" \\ path".into());
        }
        let value = parse("api_status(hostile)", &api_status(&state));
        assert_eq!(value["error"].as_str().unwrap(), "error: \"bad\" \\ path");
        parse("api_cancel", &api_cancel(&state));
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
    fn browse_lists_dirs_and_first_segment_images() {
        let base = std::env::temp_dir().join(format!("ft_browse_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("sub")).unwrap();
        std::fs::write(base.join("img.E01"), b"e").unwrap();
        std::fs::write(base.join("img.E02"), b"e").unwrap();
        std::fs::write(base.join("note.txt"), b"n").unwrap();

        let listing = browse_json(&base.display().to_string(), true);
        assert!(listing.contains("\"name\":\"sub\""));
        assert!(listing.contains("\"name\":\"img.E01\""));
        // Continuation segments and unrelated files stay hidden — the
        // examiner picks the first segment and libewf opens the rest.
        assert!(!listing.contains("img.E02"));
        assert!(!listing.contains("note.txt"));

        let dirs_only = browse_json(&base.display().to_string(), false);
        assert!(dirs_only.contains("\"name\":\"sub\""));
        assert!(!dirs_only.contains("img.E01"));

        let file = browse_json(&base.join("img.E01").display().to_string(), true);
        assert!(file.contains("\"is_file\":true"));

        let missing = browse_json(&base.join("nope").display().to_string(), false);
        assert!(missing.contains("\"exists\":false"));

        let roots = browse_json("", false);
        assert!(roots.contains("\"ok\":true"));
        assert!(roots.contains("\"entries\":["));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn export_selected_copies_allowed_files_and_writes_manifest() {
        let base = std::env::temp_dir().join(format!("ft_export_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let case_dir = base.join("case");
        let source_dir = base.join("source");
        std::fs::create_dir_all(case_dir.join("evidence/logs")).unwrap();
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::write(source_dir.join("clip.mp4"), b"video-bytes").unwrap();
        std::fs::write(base.join("outside.bin"), b"out").unwrap();

        let state: SharedState = Arc::new(Mutex::new(JobState::new()));
        {
            let mut guard = state.lock().unwrap();
            guard.case_dir = Some(case_dir.clone());
            guard.media_roots = vec![case_dir.clone(), source_dir.clone()];
        }
        let body = format!(
            "{{\"items\":[\
                {{\"id\":\"vid_1\",\"name\":\"clip.mp4\",\"path\":{},\"kind\":\"video\",\"status\":\"ffprobe-video-stream-confirmed\",\"mark\":\"important\",\"tags\":[\"사고\"],\"warnings\":[]}},\
                {{\"id\":\"fls:7\",\"name\":\"\",\"path\":\"\",\"kind\":\"candidate\",\"status\":\"candidate-unvalidated\"}},\
                {{\"id\":\"evil\",\"name\":\"outside.bin\",\"path\":{},\"kind\":\"video\"}}\
            ]}}",
            json_string(&source_dir.join("clip.mp4").display().to_string()),
            json_string(&base.join("outside.bin").display().to_string())
        );
        let request = Request {
            method: "POST".into(),
            path: "/api/export-selected".into(),
            query: String::new(),
            body,
            range: None,
            origin: None,
            host: None,
        };
        let out = api_export_selected(&request, &state);
        assert!(out.contains("\"ok\":true"), "{out}");
        assert!(out.contains("\"copied\":1"), "{out}");
        assert!(out.contains("\"skipped\":2"), "{out}");

        let export_dir = std::fs::read_dir(case_dir.join("exports"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert!(export_dir.join("vid_1__clip.mp4").is_file());
        let manifest = std::fs::read_to_string(export_dir.join("manifest.csv")).unwrap();
        assert!(manifest.contains("vid_1"));
        assert!(manifest.contains("no-file(pre-recovery candidate)"));
        // A path outside the approved roots is refused, not copied.
        assert!(manifest.contains("outside-approved-roots"));
        assert!(!export_dir.join("evil__outside.bin").exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn export_safe_names_keep_unicode_and_strip_forbidden_chars() {
        assert_eq!(export_safe_name("블랙박스 영상.mp4"), "블랙박스 영상.mp4");
        assert_eq!(
            export_safe_name("a/b\\c:d*e?f\"g<h>i|j"),
            "a_b_c_d_e_f_g_h_i_j"
        );
        assert_eq!(export_safe_name("name."), "name");
        assert_eq!(export_safe_name("..."), "item");
        assert_eq!(export_safe_name(""), "item");
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
