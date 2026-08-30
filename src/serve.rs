//! Local examiner workstation launcher.
//!
//! Running the bare `frametrace.exe` binary starts a 127.0.0.1-only HTTP
//! server that hosts the INPUT wizard, drives the existing CLI pipeline as
//! subprocesses (so audit logging and job tracking stay identical), serves the
//! generated review/report pages, and streams evidence media with Range
//! support so the browser can play it without file:// restrictions.

use crate::util::json_escape;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
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
    let port = match options.port {
        Some(port) => port,
        None => first_free_port().ok_or("사용 가능한 로컬 포트를 찾지 못했습니다")?,
    };
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|err| format!("failed to bind 127.0.0.1:{port}: {err}"))?;
    let url = format!("http://127.0.0.1:{port}/");
    println!("FrameTrace examiner workstation is running.");
    println!("  {url}");
    println!("Close this window to stop the workstation.");
    let _ = std::io::stdout().flush();
    open_in_browser(&url);
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

fn first_free_port() -> Option<u16> {
    for port in 8477..=8486 {
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return Some(port);
        }
    }
    for port in [0u16] {
        if let Ok(listener) = TcpListener::bind(("127.0.0.1", port)) {
            return listener.local_addr().ok().map(|addr| addr.port());
        }
    }
    None
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
}

fn handle_connection(mut stream: TcpStream, state: SharedState) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|err| format!("read timeout: {err}"))?;
    let request = read_request(&mut stream)?;
    if request.method == "GET" && request.path == "/media" {
        return serve_media(&mut stream, &request, &state);
    }
    let response = route(&request, &state);
    stream
        .write_all(&response)
        .map_err(|err| format!("write failed: {err}"))?;
    let _ = stream.flush();
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
        Some((path, query)) => (percent_decode(path), query.to_string()),
        None => (percent_decode(&target), String::new()),
    };
    Ok(Request {
        method,
        path,
        query,
        body: String::from_utf8_lossy(&body).to_string(),
        range,
    })
}

fn percent_decode(input: &str) -> String {
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
            b'+' => {
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
        let (name, value) = pair.split_once('=')?;
        if name == key {
            return Some(percent_decode(value));
        }
    }
    None
}

/// Minimal JSON-string / bool / number field extractor for the tiny
/// fixed-shape bodies this server accepts ({"key": value, ...}).
fn body_value(body: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":");
    let mut search = 0;
    while let Some(hit) = body[search..].find(&needle) {
        let absolute = search + hit;
        let rest = body[absolute + needle.len()..].trim_start();
        if let Some(rest) = rest.strip_prefix('"') {
            let mut decoded = String::new();
            let mut chars = rest.chars();
            while let Some(ch) = chars.next() {
                match ch {
                    '\\' => match chars.next() {
                        Some('n') => decoded.push('\n'),
                        Some('t') => decoded.push('\t'),
                        Some('r') => decoded.push('\r'),
                        Some(other) => decoded.push(other),
                        None => break,
                    },
                    '"' => return Some(decoded),
                    other => decoded.push(other),
                }
            }
            return Some(decoded);
        }
        for (token, value) in [("true", "true"), ("false", "false")] {
            if rest.starts_with(token) {
                return Some(value.to_string());
            }
        }
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return Some(digits);
        }
        search = absolute + needle.len();
    }
    None
}

fn json_string(value: &str) -> String {
    format!("\"{}\"", json_escape(value))
}

fn route(request: &Request, state: &SharedState) -> Vec<u8> {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => page(EXAMINER_PAGE.as_bytes().to_vec()),
        ("GET", "/api/env") => json(api_env()),
        ("GET", "/api/status") => json(api_status(state)),
        ("POST", "/api/start") => json(api_start(request, state)),
        ("POST", "/api/finalize") => json(api_finalize(state)),
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

fn api_env() -> String {
    format!(
        "{{\"ok\":true,\"ffmpeg\":{},\"ffprobe\":{},\"ewf\":{}}}",
        json_bool(tool_available("ffmpeg", "-version")),
        json_bool(tool_available("ffprobe", "-version")),
        json_bool(tool_available("ewfinfo", "-V"))
    )
}

fn tool_available(name: &str, version_arg: &str) -> bool {
    // resolve_tool_binary covers PATH plus the portable tools/bin layout, so
    // a bare binary dropped next to frametrace-app.exe is detected.
    let Ok(resolved) = crate::tool_policy::resolve_tool_binary(name, &[name]) else {
        return false;
    };
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
        "{{\"ok\":true,\"has_job\":{},\"phase\":{},\"steps\":[{}],\"step_names\":[{}],\"current\":\"{current}\",\"logs\":[{}],\"case_dir\":{},\"package_dir\":{},\"error\":{error}}}",
        json_bool(guard.phase != "idle"),
        json_string(guard.phase),
        steps.join(","),
        names.join(","),
        logs.join(","),
        opt(&guard.case_dir),
        opt(&guard.package_dir),
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
    set_step(&state, 1, StepStatus::Running);
    if raw_path.exists() {
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
        match run_step(&exe, &args) {
            Ok(output) => {
                log(&state, output);
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
    match run_step(&exe, &scan_args) {
        Ok(output) => {
            log(&state, output);
            set_step(&state, 3, StepStatus::Done);
        }
        Err(err) => {
            log(&state, format!("논리 색인 건너뜀: {err}"));
            set_step(&state, 3, StepStatus::Done);
        }
    }

    // Step 5: review bundle.
    set_step(&state, 4, StepStatus::Running);
    match run_step(&exe, &["make-review".into(), case_text.clone()]) {
        Ok(output) => {
            log(&state, output);
            set_step(&state, 4, StepStatus::Done);
            let mut guard = state_lock(&state);
            guard.phase = "review-ready";
            guard.busy = false;
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
    match run_step(&exe, &scan_args) {
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

    set_step(&state, 4, StepStatus::Running);
    match run_step(&exe, &["make-review".into(), case_text.clone()]) {
        Ok(output) => {
            log(&state, output);
            set_step(&state, 4, StepStatus::Done);
            let mut guard = state_lock(&state);
            guard.phase = "review-ready";
            guard.busy = false;
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
    guard.error = Some(message.to_string());
    guard.logs.push(format!("오류: {message}"));
}

fn run_step(exe: &Path, args: &[String]) -> Result<String, String> {
    let output = Command::new(exe)
        .args(args)
        .output()
        .map_err(|err| format!("{} 실행 실패: {err}", exe.display()))?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        text.push_str(&stderr);
    }
    if !output.status.success() {
        let mut tail_lines: Vec<&str> = text.lines().rev().take(4).collect();
        tail_lines.reverse();
        let tail = tail_lines.join(" | ");
        return Err(format!(
            "{} 실패 (exit {:?}): {}",
            args.first().map(String::as_str).unwrap_or("command"),
            output.status.code(),
            tail
        ));
    }
    Ok(text.trim_end().to_string())
}

/// Extract every `"id":"..."` value from db/video_index.json and write a
/// validate-batch selection file covering all of them.
fn build_selection_file(case_dir: &Path, output: &Path) -> Result<usize, String> {
    let index_path = case_dir.join("db/video_index.json");
    let index = std::fs::read_to_string(&index_path)
        .map_err(|err| format!("failed to read {}: {err}", index_path.display()))?;
    let bytes = index.as_bytes();
    let mut ids: Vec<String> = Vec::new();
    let mut cursor = 0usize;
    while let Some(hit) = index[cursor..].find("\"id\"") {
        let absolute = cursor + hit;
        let mut after = absolute + 4;
        while bytes.get(after).is_some_and(|b| b.is_ascii_whitespace()) {
            after += 1;
        }
        if bytes.get(after) == Some(&b':') {
            after += 1;
            while bytes.get(after).is_some_and(|b| b.is_ascii_whitespace()) {
                after += 1;
            }
            if bytes.get(after) == Some(&b'"') {
                let start = after + 1;
                let mut end = start;
                while let Some(&byte) = bytes.get(end) {
                    if byte == b'"' && bytes.get(end.wrapping_sub(1)) != Some(&b'\\') {
                        break;
                    }
                    end += 1;
                }
                if end > start
                    && let Ok(raw) = std::str::from_utf8(&bytes[start..end])
                    && raw.starts_with("vid_")
                {
                    ids.push(raw.to_string());
                }
                cursor = end + 1;
                continue;
            }
        }
        cursor = absolute + 4;
    }
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
    ) {
        return format!("{{\"ok\":false,\"error\":{}}}", json_string(&err));
    }
    if let Err(err) = run_step(&exe, &["make-report".into(), case_text.clone()]) {
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
    let report = run_step(&exe, &["make-report".into(), case_text.clone()]);
    let packaging = if report.is_ok() {
        run_step(&exe, &["package-case".into(), case_text.clone()])
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
    let root_text = root_canonical.to_string_lossy();
    let candidate_text = candidate.to_string_lossy();
    let root_len = root_text.len();
    candidate_text.len() >= root_len
        && candidate_text[..root_len].eq_ignore_ascii_case(&root_text)
        && (candidate_text.len() == root_len
            || candidate_text[root_len..].starts_with('\\')
            || candidate_text[root_len..].starts_with('/'))
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
    let (start, end, code) = match request.range.as_deref().and_then(parse_range) {
        Some((start, end)) if start < total => {
            let end = end.unwrap_or(total - 1).min(total - 1);
            if end < start {
                return write_range_unsatisfiable(stream, total);
            }
            (start, end, 206u16)
        }
        Some(_) => return write_range_unsatisfiable(stream, total),
        None => (0u64, total.saturating_sub(1), 200u16),
    };
    let length = end - start + 1;
    let mut file = match std::fs::File::open(&canonical) {
        Ok(file) => file,
        Err(_) => return write_simple(stream, 404, b"media not found"),
    };
    if file.seek(SeekFrom::Start(start)).is_err() {
        return write_simple(stream, 500, b"seek failed");
    }
    let mut head = format!(
        "HTTP/1.1 {code} {}\r\nContent-Type: {}\r\nContent-Length: {length}\r\nAccept-Ranges: bytes\r\nCache-Control: no-store\r\nConnection: close\r\n",
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

fn parse_range(value: &str) -> Option<(u64, Option<u64>)> {
    let value = value.strip_prefix("bytes=")?;
    let (start_text, end_text) = value.split_once('-')?;
    if start_text.is_empty() {
        return None; // suffix ranges are rare in browsers; ignore safely
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
        "HTTP/1.1 {code} {status_text}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: {cache}\r\nConnection: close\r\n",
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
        assert_eq!(
            percent_decode("C%3A%5CUsers%5C%ED%95%9C%EA%B8%80%5Ca+b.mp4"),
            "C:\\Users\\한글\\a b.mp4"
        );
        assert_eq!(percent_decode("plain"), "plain");
        assert_eq!(percent_decode("bad%2"), "bad%2");
        assert_eq!(percent_decode("tail%"), "tail%");
    }

    #[test]
    fn parses_media_range_headers() {
        assert_eq!(parse_range("bytes=0-1023"), Some((0, Some(1023))));
        assert_eq!(parse_range("bytes=100-"), Some((100, None)));
        assert_eq!(parse_range("bytes=-500"), None);
        assert_eq!(parse_range("items=1-2"), None);
        assert_eq!(parse_range("bytes=abc-"), None);
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
        // case-insensitive drive/path comparison on Windows
        let upper = PathBuf::from(root_canon.to_string_lossy().to_uppercase());
        assert!(path_is_under(&upper, &child_canon));
        assert!(!path_is_under(&root_canon, &sibling_canon));
        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::remove_dir_all(&sibling);
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
        let status = response("GET /api/status HTTP/1.1\r\nHost: x\r\n\r\n");
        assert!(status.contains("200 OK"));
        assert!(status.contains("\"has_job\":false"));
        assert!(status.contains("\"phase\":\"idle\""));
        let env = response("GET /api/env HTTP/1.1\r\nHost: x\r\n\r\n");
        assert!(env.contains("\"ok\":true"));
        assert!(env.contains("\"ffmpeg\":"));
        let review = response("GET /review/nope.html HTTP/1.1\r\nHost: x\r\n\r\n");
        assert!(review.contains("404"));
        let media = response("GET /media HTTP/1.1\r\nHost: x\r\n\r\n");
        assert!(media.contains("400"));
        let traversal = response(
            "POST /api/start HTTP/1.1\r\nHost: x\r\nContent-Type: application/json\r\nContent-Length: 33\r\n\r\n{\"source_path\":\"C:\\nope\\missing\"}",
        );
        assert!(traversal.contains("\"ok\":false"));
    }
}
