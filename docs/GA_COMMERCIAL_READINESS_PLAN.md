# FrameTrace GA / Commercial Readiness Plan

FrameTrace is not ready for GA or commercial release yet. The current honest state is: local Rust engine and browser evidence-review surfaces are strong, but Windows field validation, WinUI shell, packaging, clean-VM install, corpus proof, support operations, and commercial governance remain gated.

This document is the short handoff for a fresh Windows checkout. The full executable plan is `.omo/plans/frametrace-ga-commercial-readiness-20260628.md`.

## Current Release Boundary

- Current decision: `BLOCKED`, not `FIELD_PILOT_GO`.
- Current blocker: real Windows 10/11 x64 MSVC engine validation has not run.
- Do not claim GA, production readiness, legal admissibility, certification, or automatic integrity guarantees.
- Treat the product as a local-first forensic video review workstation candidate until Windows and field-pilot gates pass.

## Windows First Command

After cloning on Windows 10/11 x64 with MSVC Rust, FFmpeg/ffprobe, PowerShell, and required forensic tools installed:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\windows\validate-engine.ps1 `
  -CaseRoot C:\Temp\frametrace-engine-case `
  -PerformanceRows 100000
```

If this does not pass, stop and fix the Windows engine blocker before WinUI, packaging, clean VM, or field-pilot work.

## Field-Pilot Path

1. Pass Windows engine validation.
2. Implement the WinUI 3 shell as an engine-command-only workstation.
3. Build and test the WinUI shell on Windows.
4. Build MSIX as the primary package, with unsigned ZIP only as a lab fallback.
5. Validate install, launch, workflow, uninstall, and reinstall on a clean Windows VM.
6. Run synthetic and mixed real-world-like corpus validation.
7. Run final `qa release` and produce `FIELD_PILOT_GO`, `NO_GO`, or `BLOCKED`.

## GA / Commercial Path

After field-pilot evidence exists, add:

- Code signing, timestamping, SBOM, checksum, dependency, and release provenance gates.
- Security, privacy, logging redaction, crash dump, and local-only data-retention policies.
- Support triage, incident response, hotfix, rollback, compatibility matrix, and regression cadence.
- External practitioner review and field-pilot feedback disposition.
- Commercial packaging: EULA/license, pricing, support SLA, offline activation policy, and release notes.
- Long-run validation for large inventories, interrupted jobs, Unicode/long paths, locked files, corrupted media, and unsupported vendor formats.

## Git Hygiene

Commit source, scripts, tests, docs, and durable plans. Do not commit local `.omo/evidence` browser profiles, generated case DBs, screenshots, caches, server logs, or QA scratch outputs. They should be regenerated on the machine that runs the gate.
