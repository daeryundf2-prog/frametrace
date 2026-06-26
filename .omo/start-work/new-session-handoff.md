# FrameTrace Start-Work Handoff

Created: 2026-06-26
Updated: 2026-06-27

## Workspace

- Repo: `/Users/shinyoohag/Desktop/frametrace`
- Branch: `codex/frametrace-forensic-hardening`
- Active plan: `.omo/plans/frametrace-production-hardening-review-plan.md`
- Boulder state: `.omo/boulder.json`
- Ledger: `.omo/start-work/ledger.jsonl`
- Evidence root: `.omo/evidence/frametrace-production-hardening-review-plan/`

## Current State

- T1 through T12 are checked complete in the plan.
- T13 is the first unchecked task: `Validate Rust engine on Windows 10/11 x64 before GUI work`.
- Boulder status is `blocked`, not complete.
- Blocked task: T13.
- Blocked reason: no real Windows 10/11 x64 host or safely triggerable Windows runner is available from the current macOS session.
- Blocker receipt: `.omo/evidence/frametrace-production-hardening-review-plan/task-13-windows-engine/BLOCKED-missing-windows-runner.json`
- Independent blocker gate: `.omo/evidence/frametrace-production-hardening-review-plan/task-13-windows-engine/t13-gate-review.md`
- T14, T15, T16, and T17 must remain stopped until T13 passes on Windows.
- The GUI evidence-viewer IA/Korean/preview/window-mode ULW work is complete as local GUI work, but it does not satisfy the T13 Windows engine gate.
- Generated OMO database evidence under `.omo/evidence/**/db/` is intentionally gitignored because some synthetic QA files exceed GitHub's 100MB object limit. The textual receipts/transcripts remain the durable evidence to commit.

## Resume Instructions For A New Codex Session

Open a new session in `/Users/shinyoohag/Desktop/frametrace` after pulling the latest branch and paste:

```text
[$omo:start-work](/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/start-work/SKILL.md)

Continue the active FrameTrace start-work plan from `.omo/plans/frametrace-production-hardening-review-plan.md`.
Do not re-plan.
Read `.omo/boulder.json`, `.omo/start-work/ledger.jsonl`, and `.omo/start-work/new-session-handoff.md`.
If the Boulder session id is from the previous Codex session, preserve it and add the current `codex:<session_id>` before continuing.
T1-T12 are complete; the next task is T13.
The current Boulder state is blocked because no real Windows 10/11 x64 runner was available. Preserve the blocker until a real Windows transcript exists.
Do not start T14-T17 until T13 has independent gate verification.
Follow the start-work rule: root orchestrates only, implementation and QA go through subagents, and every checkbox needs independent gate verification before completion.
```

## First Verification In New Session

Run/read:

```bash
pwd
git status --short --branch
grep -n '^- \\[[ x]\\] T' .omo/plans/frametrace-production-hardening-review-plan.md
cat .omo/boulder.json
tail -n 20 .omo/start-work/ledger.jsonl
```

Expected:

- current directory is `/Users/shinyoohag/Desktop/frametrace`
- T1-T12 are `[x]`
- T13 is the first `[ ]`
- `.omo/boulder.json` has `status: "blocked"` with `blocked_task` set to T13
- T13 blocker receipt exists at `.omo/evidence/frametrace-production-hardening-review-plan/task-13-windows-engine/BLOCKED-missing-windows-runner.json`

## Windows T13 Continuation

On a real Windows 10/11 x64 host with Rust MSVC, run the T13 checks before any WinUI or installer work:

```powershell
git fetch --all --prune
git checkout codex/frametrace-forensic-hardening
git pull --ff-only

rustc -vV
cargo fmt --all -- --check
cargo check --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --release --locked
.\target\release\frametrace.exe --help
.\target\release\frametrace.exe init-case C:\Temp\frametrace-empty-case --title "Windows T13 empty case"
.\target\release\frametrace.exe workstation-status C:\Temp\frametrace-empty-case
```

Then run or repair the release validation surface:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\windows\validate-release.ps1 -CaseRoot C:\Temp\frametrace-release-case -PerformanceRows 1000
```

Important: `scripts\windows\validate-release.ps1` currently also checks for a concrete `gui\winui` project and WinUI tests. If this fails only because WinUI does not exist yet, do not mark T13 complete from that script alone. For T13, capture a Windows/MSVC Rust engine transcript, tool-discovery receipt, synthetic MP4 workflow, E01/raw workflow if tools are installed, Unicode/long-path checks, repeated scan checks, file-lock behavior, and `reports\qa\windows-prerequisites.json`. If those pass, record the evidence under:

```text
.omo/evidence/frametrace-production-hardening-review-plan/task-13-windows-engine/
```

After T13 has a passing Windows transcript, run an independent gate review. Only then mark T13 complete, update `.omo/boulder.json` out of `blocked`, append the ledger, and continue to T14.

If no Windows runner is available, keep the blocker receipt and stop T14-T17.
