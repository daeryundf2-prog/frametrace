# WinUI 3 Shell Contract

FrameTrace의 최종 Windows shell은 C#/WinUI 3 UI다. 하지만 durable state의 source of truth는 Rust engine, SQLite case database, chained JSONL audit logs다. GUI는 상태를 재구현하지 않고 engine command를 호출하거나 bounded JSON/status output을 읽는다.

## Prerequisite Gate

WinUI 구현은 아래가 모두 fresh evidence로 통과한 뒤 시작한다.

| Gate | Required Evidence |
| --- | --- |
| PASS evidence rejects invalid proof | ULW-loop component tests |
| checkpoint rejects invalid PASS | ULW-loop component tests |
| `status --json` returns session-scoped canonical paths | ULW-loop status smoke |
| reconcile-hook-state command and stale Stop hook regression exist | ULW-loop component tests |
| HEAVY quality gate cannot use magic phrase bypass | ULW-loop component tests |
| bootstrap fallback supports known wrapper forms | ULW-loop component tests |
| Production inventory row/API contract exists | Rust inventory contract tests |
| SQLite inventory uses bounded paging/search/facets/sort | Rust inventory tests and performance report |
| 10k virtualized HTML prototype exists | Browser/manual QA evidence |
| 100k/1M full JSON browser load is forbidden | GUI large inventory plan and performance evidence |
| Candidate cannot become confirmed without validation | media contract tests |
| ffprobe stream confirmation and playback confirmation are separate | `media_contract` tests and `confirm-playback` CLI smoke |
| Derived artifacts are recorded as derived outputs | media/audit/report tests |
| Report wording lint forbids banned legal readiness claims | QA report-defense tests |

## Source Of Truth

| State | Owner | GUI Rule |
| --- | --- | --- |
| Case manifest | `case.json` created by Rust engine | Read only |
| File inventory | `db/case.db` | Read through engine-backed paged query |
| Jobs/progress | `db/case.db` `jobs` and `job_events` | Display engine state; mutation only through engine |
| Validation | `evidence/logs/validation-log.jsonl` | Display chain status and validation states |
| Derived artifacts | `artifacts/*` plus chained JSONL logs | Display only after engine records output hash/provenance |
| Report/package | `reports/*`, `review/*`, package manifest | Generate through engine commands |

The GUI must not write source evidence paths, create derived outputs outside the case folder, or mark candidates confirmed without engine validation evidence.

## Engine Command Contract

WinUI shell actions call these commands instead of reimplementing forensic logic:

| Shell Action | Engine Command |
| --- | --- |
| Create case | `init-case` |
| Register folder/drive/image source | `register-source` |
| Scan copied folder/read-only volume | `scan-folder` |
| Inspect/import E01 | `inspect-e01`, `import-e01` |
| Inspect raw image and recover inode | `inspect-image`, `recover-inode` |
| Carve contiguous candidates | `carve-file` |
| Confirm container/video stream | `validate-artifact` |
| Confirm examiner playback review | `confirm-playback` |
| Generate proxy/thumbnail/frame/clip | `make-proxy`, `make-thumbnail`, `capture-frame`, `export-video` |
| List/search/facet inventory | `inventory` |
| Preview bulk action | `inventory-bulk-preview` |
| Export selected manifest | `inventory-export-manifest` |
| Generate review/report/package | `make-review`, `make-report`, `package-case` |
| Run release QA | `qa release` |
| Read shell status | `workstation-status` |

## `workstation-status` JSON

`frametrace workstation-status <case_dir>` is the shell bootstrap/status surface. It returns bounded JSON:

```json
{
  "schema_version": 1,
  "view": "workstation-status",
  "engine_source_of_truth": true,
  "gui_durable_state_allowed": false,
  "sqlite": {
    "exists": true,
    "video_count": 0,
    "active_job_count": 0
  },
  "inventory": {
    "transport": "sqlite-bounded-query",
    "full_json_load_allowed": false,
    "max_page_size": 500
  },
  "validation": {
    "ffprobe_video_stream_confirmed_count": 0,
    "playback_confirmed_count": 0,
    "ffprobe_and_playback_are_separate_states": true
  },
  "windows_prerequisites": {
    "host_os": "windows",
    "required_tools": ["rustc", "cargo", "ffmpeg", "ffprobe", "dotnet"],
    "missing_tools": [],
    "winui_project_present": true,
    "winui_project_files": ["gui/winui/FrameTrace/FrameTrace.csproj"],
    "release_validation_host_ready": true,
    "blockers": []
  },
  "winui_contract": {
    "durable_mutation": "engine-command-only",
    "state_owner": "rust-engine-sqlite-audit",
    "inventory_transport": "paged-sqlite-query",
    "large_case_full_json_load_allowed": false,
    "candidate_promotion": "validate-artifact then confirm-playback",
    "release_language": "report-defensible"
  }
}
```

The shell should call this at case open, after command completion, and after resume. Large inventory rows still come from `inventory` with bounded page size.

## Long-Running Job Rules

1. GUI starts a Rust engine command.
2. Engine creates a `jobs` row.
3. GUI polls status through `workstation-status`, `inspect`, or future bounded job status command.
4. GUI never marks a job complete directly.
5. If the app exits mid-command, next open must display active/interrupted jobs and require `mark-interrupted-jobs` before release packaging.

## Release Readiness

Windows release validation must run:

```powershell
scripts\windows\validate-release.ps1
```

Minimum pass evidence:

- Windows MSVC `cargo fmt`, `cargo check`, `cargo clippy`, `cargo test`, release build pass.
- Windows prerequisite status reports Windows host, required tools present, and concrete WinUI project evidence (`.sln` or `.csproj`) under `gui/winui`.
- Before `qa release`, Windows validation writes `reports/qa/winui-build.json` with passing `dotnet build` and `dotnet test` evidence. Missing build/test receipt is a `windows_prerequisites` release blocker.
- Synthetic MP4 workflow runs through `validate-artifact` and `confirm-playback`.
- `workstation-status` confirms `engine_source_of_truth`, bounded inventory transport, and separate ffprobe/playback states.
- Report/review/package generation succeeds.
- `qa release` passes with all global release-blocker review-manifest gates satisfied, including privacy, supply-chain, installer/package, Windows workstation validation, support/triage, incident response, corpus governance, feature intake, post-GA monitoring, external review readiness, and regression schedule.
- `qa release` writes `reports/qa/workstation-status.json` and includes PASS `workstation_shell_contract` and `windows_prerequisites` checks. Release readiness is blocked if status output no longer proves engine-owned durable state, SQLite-bounded inventory transport, disabled full-case JSON browser loading, separate ffprobe/playback states, engine-only durable mutation, Windows host readiness, required tools, concrete WinUI project files, or the WinUI build/test receipt.

## Current Boundary

This repository now exposes the production-safe Rust/SQLite status surface for the WinUI shell. On macOS, `workstation-status` and `qa release` must report Windows/WinUI prerequisites as blocked rather than claiming Windows readiness. A real C#/WinUI 3 project is still a Windows/.NET implementation step and must not replace the engine or SQLite source of truth.
