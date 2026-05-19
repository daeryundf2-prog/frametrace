# Evidence Viewer GUI Plan

FrameTrace should be a viewer-first workstation. The examiner needs to know how many files are present, which files are viewable, which files still need verification, and how each viewed frame links back to immutable source evidence.

## Primary Screen

The primary screen is the Evidence Viewer, not a marketing dashboard.

- Left: evidence sources and review queues.
- Center-left: searchable file inventory with counts, thumbnails, parser lane, time, size, and status.
- Center: video/photo viewer with timeline, playback, zoom, range selection, and derived-output controls.
- Right: forensic inspector with source path, hash state, parser, validation state, notes, and audit trail.

The screen answers these questions immediately:

- How many files were indexed?
- How many are video, photo, carved candidates, or verification risks?
- Which files are unreviewed, important, report-selected, or already exported?
- What is the current file's source, hash, parser lane, and validation boundary?

## Viewer Rules

- Original evidence is never modified.
- Proxy, thumbnail, frame capture, clip export, and contrast/zoom outputs are derived artifacts.
- Every derived artifact must link to source file ID, source hash when available, command parameters, operator, timestamp, and output hash.
- Carved files are displayed as candidates until playback/container validation is recorded.
- The viewer must distinguish `candidate-unvalidated`, `duplicate-candidate`, `verified playable`, and `derived artifact`.

## Video Review

- Playback, pause, frame-step, previous/next file, and speed controls.
- Timeline with selected export range and event markers.
- Current timecode visible inside the viewing area.
- Front/rear or multi-camera synchronized review path.
- One-click MP4/AVI export from selected range.
- One-click frame capture as a derived photo artifact.

## Photo Review

- Fit, zoom, pan, and actual-pixel inspection.
- Hash and metadata display next to the image.
- Report selection and derived-output audit trail.

## Prototype

The static prototype lives at:

```text
gui/evidence-viewer/index.html
```

It is intentionally serverless so it can be opened directly from disk on a forensic workstation. It uses mocked case data and canvas-rendered sample frames to validate the workflow before the final Windows shell is implemented.

## Localization

The prototype defaults to Korean because examiner-facing review is expected to happen on Korean Windows workstations. The top-right `EN` / `KO` control switches the visible UI language without changing evidence values such as paths, hashes, parser IDs, or vendor names.

Production localization should keep these rules:

- Case data, file paths, hashes, parser IDs, and raw metadata stay verbatim.
- UI labels, queue names, activity labels, validation states, and inspector field names are localized.
- The selected locale is a user preference, not a case-evidence mutation.
- Reports should record the report language separately from the original evidence metadata.

## QC Decisions

The prototype was reviewed from both UI/UX and forensic-workflow perspectives. These guardrails are intentional:

- The viewer is weighted above dashboard chrome: source/browser/inspector panes are supporting context.
- Timestamps remain visible in the inventory at normal desktop widths; thumbnails become secondary when space is constrained.
- The `2ch` control renders a synchronized split review mock for paired front/rear clips.
- Output and validation controls are queue-style prototype states, not durable forensic actions.
- The activity panel is session activity only. Production audit must render chained engine logs.
- A carved candidate must not become `verified playable` until a core validation command records tool, method, operator, timestamp, source artifact, and audit chain.

## OpenDesign Handoff

The repo includes an OpenDesign-compatible design system at:

```text
opendesign/design-systems/frametrace-forensic-workstation/
```

It captures the current viewer tokens, Korean-first forensic wording, large-case layout rules, and QC checklist. If OpenDesign is installed later, it should use this design system instead of inventing a new product style.

## Production Path

The final production shell should be WinUI 3 on Windows 10/11 x64. The Rust engine remains the source of truth. The GUI should call the engine for:

- source registration
- scan/import/carve jobs
- SQLite case state reads
- review annotations and report-set persistence
- carved/container validation with durable audit
- proxy/thumbnail/frame/export generation
- frame capture as a distinct derived artifact
- report/package creation

Only after the command contract is stable should the shell move from prototype to signed Windows installer.
