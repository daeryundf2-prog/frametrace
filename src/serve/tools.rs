//! Track S4: guided tool installs from the workstation.
//!
//! * `GET  /api/tools` — manifest summary + resolution status (+ the last
//!   install job). On-demand tools (deepfake-lens) are included, flagged
//!   `on_demand: true` (`?exclude_on_demand=1` leaves them out).
//! * `POST /api/tools/install` `{"name": "<tool>", "force"?: true}` —
//!   nonce-gated like every POST; takes the busy lease, then
//!   downloads/extracts on a worker thread. Fix round 16: refused with
//!   `would_replace` / `already_installed` (+ `replaces`) when it would
//!   overwrite files in tools/bin unless `force`, and with
//!   `install_running_elsewhere` while a CLI install holds the lock.
//! * `GET  /api/tools/install-status` — cheap poll of that job.
//! * `POST /api/tools/install-cancel` `{"id"}` — asks a running install to
//!   stop (an `id` that is not the current job → `stale_job`):
//!   honoured while downloading / verifying / extracting (checked between
//!   stages, curl killed mid-download), refused with
//!   `{"ok":false,"code":"not_cancellable"}` once files are being moved
//!   into place or the venv / pip runs.
//!
//! Every refusal of `POST /api/tools/install` is also recorded in
//! `tools/install-audit.jsonl` (`result: "refused"`, `code`), written
//! after the state lock is released.
//!
//! State-lock rule: the lock is held only to check/record the job; tool
//! resolution, network and disk work never run under it, and progress is
//! read from lock-free atomics.

use super::*;
use crate::tool_install::{self, InstallDirs, Manifest, NetPolicy, Progress, Stage};

pub(crate) struct ToolInstallJob {
    pub(crate) id: u64,
    pub(crate) name: String,
    /// running | done | failed
    pub(crate) state: &'static str,
    pub(crate) error: Option<String>,
    pub(crate) progress: Arc<Progress>,
    pub(crate) started_unix: u64,
    pub(crate) finished_unix: Option<u64>,
    pub(crate) installed_files: Vec<String>,
    pub(crate) audit_warning: Option<String>,
    /// A staging dir the finished install could not remove (old binary
    /// still running); the install itself succeeded.
    pub(crate) staging_left: Option<String>,
}

pub(crate) fn install_job_json(job: &ToolInstallJob) -> serde_json::Value {
    let (done, total) = job.progress.bytes();
    let stage = job.progress.stage();
    serde_json::json!({
        "id": job.id,
        "tool": job.name,
        // Track U's coarse state; `stage` is the precise step.
        "state": ui_state(job.state, stage),
        "stage": stage.as_str(),
        "bytes": done,
        "total": total,
        "error": job.error,
        // True only for a job that ended because a cancel was honoured
        // (from any tab or the CLI), so the UI never shows it as a failure.
        "cancelled": job.state == "failed"
            && job.error.as_deref() == Some(tool_install::INSTALL_CANCELLED),
        "started_unix": job.started_unix,
        "finished_unix": job.finished_unix,
        "installed_files": job.installed_files,
        "audit_warning": job.audit_warning,
        "staging_left": job.staging_left,
        // Seconds since the job finished, by the server's clock (null while
        // running), so a tab never judges "recent" by its own clock.
        "age_s": job
            .finished_unix
            .map(|finished| now_unix().saturating_sub(finished)),
    })
}

/// The server's clock (`now_unix` in GET /api/tools and install-status).
fn now_unix() -> u64 {
    crate::util::now_unix().unwrap_or(0)
}

/// The coarse install state track U polls: downloading | verifying |
/// extracting while running (queued counts as downloading, so a fresh
/// job is never `idle`; moving files / venv / pip count as extracting),
/// then done | failed.
pub(crate) fn ui_state(state: &str, stage: Stage) -> &'static str {
    match state {
        "done" => "done",
        "failed" => "failed",
        _ => match stage {
            Stage::Queued | Stage::Download => "downloading",
            Stage::Verify => "verifying",
            _ => "extracting",
        },
    }
}

/// What one install runs against; tests inject a fake source and dirs.
pub(crate) struct InstallEnv {
    pub(crate) manifest: Manifest,
    pub(crate) dirs: InstallDirs,
    pub(crate) net: NetPolicy,
}

impl InstallEnv {
    fn production() -> Result<Self, String> {
        let manifest = tool_install::manifest()?.clone();
        let net = NetPolicy::for_manifest(&manifest);
        Ok(Self {
            manifest,
            dirs: InstallDirs::for_current_exe()?,
            net,
        })
    }
}

/// `{"ok":false,"code":<code>,"error":<message>}` plus `extra` fields.
pub(crate) fn refusal(code: &str, message: &str, extra: serde_json::Value) -> String {
    let mut value = serde_json::json!({"ok": false, "code": code, "error": message});
    if let (Some(map), serde_json::Value::Object(extra)) = (value.as_object_mut(), extra) {
        map.extend(extra);
    }
    value.to_string()
}

fn job_snapshot(state: &SharedState) -> serde_json::Value {
    state_lock(state)
        .tool_install
        .as_ref()
        .map(install_job_json)
        .unwrap_or(serde_json::Value::Null)
}

pub(crate) fn api_tools(request: &Request, state: &SharedState) -> String {
    // On-demand tools (deepfake-lens) are listed with on_demand:true; the
    // UI keeps them out of its panel. ?exclude_on_demand=1 drops them.
    let exclude_on_demand =
        query_value(&request.query, "exclude_on_demand").as_deref() == Some("1");
    let manifest = match tool_install::manifest() {
        Ok(manifest) => manifest,
        Err(err) => return refusal("manifest_invalid", &err, serde_json::json!({})),
    };
    // Fix round 11 L3: an install from the CLI changed tools/bin under a
    // cached /api/env — drop it so both endpoints agree.
    super::system::revalidate_env_cache();
    // Resolution (filesystem) outside the state lock. Fix round 14 L1:
    // the kept rollback backups are listed (and sized) once per request.
    let backups = tool_install::rollback_backups_for_current_exe();
    let tools: Vec<serde_json::Value> = manifest
        .tools
        .iter()
        .filter(|tool| !(exclude_on_demand && tool.on_demand))
        .map(|tool| tool_install::tool_json(tool, &tool_install::status(tool, &backups)))
        .collect();
    serde_json::json!({
        "ok": true,
        "platform": tool_install::PLATFORM,
        "platform_supported": cfg!(all(windows, target_arch = "x86_64")),
        "tools": tools,
        "install": job_snapshot(state),
        "now_unix": now_unix(),
    })
    .to_string()
}

pub(crate) fn api_tools_install_status(state: &SharedState) -> String {
    serde_json::json!({"ok": true, "install": job_snapshot(state), "now_unix": now_unix()})
        .to_string()
}

/// `POST /api/tools/install-cancel` `{"id": <job id>}` (`id` optional for
/// older clients): a cancel naming another job than the current one — a
/// late [중단] from a tab still showing an install that already ended while
/// another tab started the next — is refused with `stale_job`, never
/// applied to the new install.
pub(crate) fn api_tools_install_cancel(request: &Request, state: &SharedState) -> String {
    let wanted = body_value(&request.body, "id");
    let guard = state_lock(state);
    if let (Some(wanted), Some(job)) = (wanted.as_deref(), guard.tool_install.as_ref())
        && wanted.trim() != job.id.to_string()
    {
        return refusal(
            "stale_job",
            "그 설치는 이미 끝났습니다 — 지금 진행 중인 설치는 중단하지 않았습니다.",
            serde_json::json!({"install": install_job_json(job)}),
        );
    }
    // A job whose worker already finished (stage done / failed, the audit
    // line being written before the state is published) is not running:
    // never "중단하는 중…" for a finished install.
    match guard.tool_install.as_ref().filter(|job| {
        job.state == "running" && !matches!(job.progress.stage(), Stage::Done | Stage::Failed)
    }) {
        Some(job) => match job.progress.request_cancel() {
            Ok(()) => serde_json::json!({"ok": true, "install": install_job_json(job)}).to_string(),
            Err(message) => refusal(
                "not_cancellable",
                message,
                serde_json::json!({"install": install_job_json(job)}),
            ),
        },
        None => refusal(
            "no_install",
            "진행 중인 도구 설치가 없습니다.",
            serde_json::json!({}),
        ),
    }
}

pub(crate) fn api_tools_install(request: &Request, state: &SharedState) -> String {
    let name = body_value(&request.body, "name").unwrap_or_default();
    // Fix round 16 M5: the person confirmed replacing what is in tools/bin.
    let force = body_value(&request.body, "force").as_deref() == Some("true");
    let env = match InstallEnv::production() {
        Ok(env) => env,
        Err(err) => return refusal("manifest_invalid", &err, serde_json::json!({})),
    };
    start_install(state, name.trim(), env, force)
}

/// Fix round 16 L-busy: whether this process's busy lease is a tool
/// install's (else a job's) — `busy_kind` in /api/status.
pub(crate) const BUSY_KIND_TOOL_INSTALL: &str = "tool-install";
pub(crate) const BUSY_KIND_JOB: &str = "job";

/// Fix round 16 (job ids): the first install job id of this server
/// process — a random 32-bit value, so a tab that followed job N of an
/// earlier server (before a restart) never takes the new server's job N
/// for its own. Ids then count up from it; still a JSON number.
pub(crate) fn initial_install_seq() -> u64 {
    let mut bytes = [0u8; 4];
    if getrandom::fill(&mut bytes).is_err() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        bytes = (nanos ^ std::process::id()).to_le_bytes();
    }
    u64::from(u32::from_le_bytes(bytes))
}

/// Fix round 16 follow-up L4: refusal code when this process holds the
/// install lock without an install job (the server-start sweep).
pub(crate) const TOOLS_BUSY: &str = "tools_busy";
pub(crate) const TOOLS_BUSY_MESSAGE: &str =
    "도구 폴더를 정리하는 중입니다 — 잠시 뒤 다시 시도하십시오.";

/// The refusal (code, message) of the in-memory checks, run once before
/// the disk checks and again while the busy lease is taken.
fn locked_refusal(guard: &JobState) -> Option<(&'static str, String)> {
    if guard
        .tool_install
        .as_ref()
        .is_some_and(|job| job.state == "running")
    {
        Some(("install_running", tool_install::INSTALL_LOCKED.to_string()))
    } else if let Some(reason) = shutdown_refusal(guard) {
        Some(("shutdown", reason))
    } else if let Some(reason) = side_work_refusal(guard, "도구를 설치하십시오") {
        // Clip exports / joins run ffmpeg from tools/bin: never swap
        // it underneath them.
        Some(("busy", reason))
    } else if guard.busy {
        Some(("busy", guard.busy_refusal(super::state::BUSY_ERROR)))
    } else {
        None
    }
}

/// A refusal that is also appended to the install audit log (never under
/// the state lock). A failed audit write is reported in `audit_warning`.
fn audited_refusal(
    env: &InstallEnv,
    tool: &str,
    code: &str,
    message: &str,
    extra: serde_json::Value,
) -> String {
    let mut extra = extra;
    if let Err(warning) = tool_install::audit_refusal(&env.dirs, tool, code, message)
        && let serde_json::Value::Object(map) = &mut extra
    {
        map.insert("audit_warning".into(), warning.into());
    }
    refusal(code, message, extra)
}

/// Validates, takes the busy lease and starts the install worker.
///
/// Fix round 16: before accepting, (M5) an install that would replace
/// files already in `tools/bin` is refused with `would_replace` /
/// `already_installed` and `replaces` unless `force`; (M6) the install
/// lock is taken here, so a CLI install holding it is refused with
/// `install_running_elsewhere` instead of a job that fails later.
pub(crate) fn start_install(
    state: &SharedState,
    name: &str,
    env: InstallEnv,
    force: bool,
) -> String {
    let Some(tool) = tool_install::find_tool(&env.manifest, name).cloned() else {
        return audited_refusal(
            &env,
            name,
            "unknown_tool",
            &format!("알 수 없는 도구입니다: {name}"),
            serde_json::json!({}),
        );
    };
    if let Some(reason) = tool_install::not_installable_reason(&tool) {
        let code = if cfg!(all(windows, target_arch = "x86_64")) {
            "not_installable"
        } else {
            "unsupported_platform"
        };
        return audited_refusal(
            &env,
            &tool.name,
            code,
            &format!("{} 자동 설치 불가: {reason}", tool.name),
            serde_json::json!({"manual_hint": tool.manual_hint, "homepage": tool.homepage}),
        );
    }
    // Resolution only (PATH stat), before the lock.
    if let Some(reason) = tool_install::python_missing_reason(&tool) {
        return audited_refusal(
            &env,
            &tool.name,
            "python_required",
            &reason,
            serde_json::json!({}),
        );
    }
    // In-memory checks first: a refusal there touches no disk.
    let early = locked_refusal(&state_lock(state));
    if let Some((code, message)) = early {
        return audited_refusal(&env, &tool.name, code, &message, serde_json::json!({}));
    }
    if !force && let Some(check) = tool_install::replace_check(&tool, &env.dirs) {
        return audited_refusal(
            &env,
            &tool.name,
            check.code,
            &check.message(&tool.name),
            serde_json::json!({"replaces": check.replaces}),
        );
    }
    // M6: the cross-process lock, outside the state lock (disk work).
    let install_lock = match tool_install::InstallLock::acquire(&env.dirs.tools_dir) {
        Ok(lock) => Some(lock),
        Err(err) if err == tool_install::INSTALL_LOCKED => {
            let (code, message) =
                if tool_install::InstallLock::held_by_this_process(&env.dirs.tools_dir) {
                    // Fix round 16 follow-up L4: held here with no install
                    // job running — the server-start sweep's brief lock:
                    // nothing to follow, so "try again shortly".
                    let job_running = state_lock(state)
                        .tool_install
                        .as_ref()
                        .is_some_and(|job| job.state == "running");
                    if job_running {
                        ("install_running", err)
                    } else {
                        (TOOLS_BUSY, TOOLS_BUSY_MESSAGE.to_string())
                    }
                } else {
                    (
                        "install_running_elsewhere",
                        tool_install::INSTALL_RUNNING_ELSEWHERE.to_string(),
                    )
                };
            return audited_refusal(&env, &tool.name, code, &message, serde_json::json!({}));
        }
        // Not lockable for another reason (the folder is not writable):
        // the worker tries again and reports it as the job's error.
        Err(_) => None,
    };
    let progress = Arc::new(Progress::default());
    // Locked checks only decide; the refusal is audited after the guard
    // is dropped (disk work never runs under the state lock).
    let started = {
        let mut guard = state_lock(state);
        match locked_refusal(&guard) {
            Some(refusal) => Err(refusal),
            None => match lease_busy_locked(&mut guard, state) {
                Ok(lease) => {
                    guard.busy_kind = BUSY_KIND_TOOL_INSTALL;
                    Ok((guard, lease))
                }
                Err(err) => Err(("busy", err)),
            },
        }
    };
    let (mut guard, lease) = match started {
        Ok(started) => started,
        Err((code, message)) => {
            drop(install_lock);
            return audited_refusal(&env, &tool.name, code, &message, serde_json::json!({}));
        }
    };
    let (lease, id, snapshot) = {
        guard.tool_install_seq += 1;
        let id = guard.tool_install_seq;
        let job = ToolInstallJob {
            id,
            name: tool.name.clone(),
            state: "running",
            error: None,
            progress: Arc::clone(&progress),
            started_unix: crate::util::now_unix().unwrap_or(0),
            finished_unix: None,
            installed_files: Vec::new(),
            audit_warning: None,
            staging_left: None,
        };
        let snapshot = install_job_json(&job);
        guard.tool_install = Some(job);
        drop(guard);
        (lease, id, snapshot)
    };
    let spawned = thread::Builder::new()
        .name("ft-tool-install".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                tool_install::install_entry_locked(
                    &tool,
                    &env.dirs,
                    &env.net,
                    &progress,
                    install_lock,
                )
            }))
            .unwrap_or_else(|_| Err("내부 오류로 설치가 중단되었습니다.".to_string()));
            if result.is_ok() {
                super::system::invalidate_env_cache();
            }
            lease.release_with(|guard| {
                if let Some(job) = guard.tool_install.as_mut().filter(|job| job.id == id) {
                    job.finished_unix = crate::util::now_unix().ok();
                    match result {
                        Ok(outcome) => {
                            job.state = "done";
                            job.installed_files = outcome
                                .installed_files
                                .iter()
                                .map(|path| path.display().to_string())
                                .collect();
                            job.audit_warning = outcome.audit_warning;
                            job.staging_left =
                                outcome.staging_left.map(|path| path.display().to_string());
                        }
                        Err(err) => {
                            job.state = "failed";
                            job.error = Some(err);
                        }
                    }
                }
            });
        });
    if spawned.is_err() {
        // The closure (and the lease) was dropped: `busy` is free again.
        let mut guard = state_lock(state);
        if let Some(job) = guard.tool_install.as_mut().filter(|job| job.id == id) {
            job.state = "failed";
            job.error = Some("설치 작업 스레드를 시작하지 못했습니다.".into());
            job.finished_unix = crate::util::now_unix().ok();
        }
        return refusal(
            "spawn_failed",
            "설치 작업 스레드를 시작하지 못했습니다.",
            serde_json::json!({}),
        );
    }
    serde_json::json!({"ok": true, "install": snapshot}).to_string()
}

/// Distinct refusal for a deepfake screening start while deepfake-lens is
/// missing (code `tool_missing`), so the UI can offer the on-demand
/// install right there. `None` when the tool resolves.
pub(crate) fn deepfake_missing_refusal() -> Option<String> {
    if crate::deepfake::lens_available() {
        return None;
    }
    let tool = tool_install::manifest()
        .ok()
        .and_then(|manifest| tool_install::find_tool(manifest, "deepfake-lens"));
    let reason = match tool {
        Some(tool) => tool_install::not_installable_reason(tool),
        None => Some("deepfake-lens is not in the tool manifest".to_string()),
    };
    Some(refusal(
        "tool_missing",
        "합성 영상 스크리닝 도구(deepfake-lens)가 설치되어 있지 않습니다.",
        serde_json::json!({
            "tool_missing": "deepfake-lens",
            "installable": reason.is_none(),
            "not_installable_reason": reason,
            "manual_hint": tool.and_then(|tool| tool.manual_hint.clone()),
            "install": {"method": "POST", "path": "/api/tools/install", "body": {"name": "deepfake-lens"}},
        }),
    ))
}
