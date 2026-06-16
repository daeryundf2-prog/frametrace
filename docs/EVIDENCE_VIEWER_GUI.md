# Evidence Viewer GUI Plan

FrameTrace should be a viewer-first workstation. The examiner needs to know how many files are present, which files are viewable, which files still need verification, and how each viewed frame links back to immutable source evidence.

## Primary Screen

The primary screen is the Evidence Viewer, not a marketing dashboard.

- Left: evidence sources and review queues.
- Center-left: high-density file inventory workstation with counts, grouped drill-down, virtualized rows, saved filters, thumbnails on demand, parser lane, time, size, status, hash state, source, and recovery context.
- Center: video/photo viewer with timeline, playback, zoom, range selection, and derived-output controls.
- Right: forensic inspector with source path, hash state, parser, validation state, notes, and audit trail.

The screen answers these questions immediately:

- How many files were indexed?
- How many are video, photo, carved candidates, or verification risks?
- Which files are unreviewed, important, report-selected, or already exported?
- What is the current file's source, hash, parser lane, and validation boundary?

## Large File Inventory Redesign

The file list must be treated as a first-class review surface, not a small companion list. Real cases may contain tens of thousands of logical files, recovered candidates, thumbnails, proxies, carved fragments, and validation outputs. The GUI must let an examiner understand the whole file population before opening individual media.

### Inventory Layout

- Top inventory strip: total files, displayed matches, selected rows, hidden-by-filter count, video/photo/candidate/derived counts, hash-complete count, validation-risk count, and current query latency.
- Left inventory tree: source -> partition/container -> folder -> parser lane -> validation state. Each node shows total count, unreviewed count, report-selected count, and risk count.
- Center inventory grid: dense tabular rows with sticky header, resizable columns, sortable columns, keyboard navigation, multi-select, and virtual scrolling.
- Right detail drawer: selected row metadata, source path, parent/child derived artifact links, hash status, offsets/inodes, validation logs, and report inclusion history.
- Bottom selection bar: bulk actions for mark reviewed, add/remove report set, queue hash, queue validation, queue proxy/thumbnail generation, export selected manifest, and clear selection.

### Required Grid Columns

Default columns:

- Status
- Review state
- File ID
- Name
- Relative path
- Source
- Type
- Parser lane
- Validation state
- Timestamp
- Size
- Hash state
- Report flag

Optional columns:

- Full path
- SHA-256
- Inode / metadata address
- Byte offset
- Partition offset
- Duration
- Codec/container
- Camera/channel
- Parent artifact
- Duplicate-of
- Job ID
- Last action
- Notes

Column presets:

- Triage: status, name, path, type, validation, timestamp, size.
- Recovery: status, name, source, inode/offset, validation, duplicate-of, hash.
- Report: report flag, name, timestamp, source, hash, parent artifact, last action.
- Hash/Audit: file ID, source, path, hash state, SHA-256, job ID, last action.

### Large-Case Interaction Rules

- The viewer must not render all rows into the DOM. Use a virtual list/grid with a fixed row-height contract and overscan.
- The viewer must not load a full 100k+ or 1M-row JSON index into browser memory for production. Query SQLite through a paged API or generated chunk index.
- Search must be server/engine-backed for production and must return both total match count and first page of ordered rows.
- Filters must be composable: source, folder/path prefix, file type, parser lane, validation state, review state, hash state, report flag, size range, time range, and recovered/candidate state.
- Sorting must be explicit and stable. Default sort is `risk desc`, `timestamp asc`, `file_id asc`.
- Opening a media item must not reset current filter, scroll position, selection set, or grouped tree expansion.
- Multi-select actions must generate an auditable action preview before mutating case state.
- Thumbnail/proxy generation must be lazy and queue-backed. Missing thumbnails should not slow list scrolling.
- Very long paths must show basename, relative path, and source badge in the row; full path appears in the detail drawer and copy menu.
- Empty states must distinguish no evidence indexed, no search matches, and current filters hiding all rows.

### Density Targets

- Standard desktop width >= 1440 px: at least 12 visible inventory rows while the viewer and inspector remain visible.
- Wide desktop width >= 1920 px: at least 18 visible inventory rows with default columns.
- Inventory-focused mode: at least 30 visible rows by collapsing the media viewer into a preview strip.
- Row height targets: compact 34 px, normal 44 px, media-preview 64 px.
- Path/name cells must use middle truncation for long paths, not only end truncation.

### Data Contract

Production inventory rows should come from SQLite, not from prototype-local JavaScript arrays. Each row must include:

```text
file_id
source_id
source_label
type
parser_lane
validation_state
review_state
report_state
display_name
relative_path
full_path
timestamp_start
timestamp_source
size_bytes
hash_state
sha256
inode
byte_offset
partition_offset
parent_artifact_id
duplicate_of
last_action_unix
```

The API/query layer must support:

- `list_inventory(page_token, page_size, sort, filters, visible_columns)`
- `search_inventory(query, filters, page_token, page_size)`
- `inventory_facets(filters)` for source/type/parser/validation/review/hash counts
- `get_file_detail(file_id)` for full metadata and audit links
- `bulk_preview(file_ids, action)` before any bulk mutation

### Acceptance Criteria

- 10,000-row fixture: initial inventory render P95 <= 2 seconds, search P95 <= 500 ms, scroll P95 frame time <= 32 ms.
- 100,000-row fixture: initial inventory render P95 <= 2.5 seconds, search P95 <= 1 second, no UI freeze > 2 seconds.
- 1,000,000-row synthetic SQLite fixture: search P99 <= 3 seconds with engine-backed query; browser memory does not grow with total row count.
- At least 12 rows are visible at 1440 px width with default layout.
- At least 30 rows are visible in inventory-focused mode.
- Every row can expose source path, hash state, parser lane, validation state, and report state without opening a separate report.
- Bulk action preview records selected count, filters used, operator action, expected mutation, and audit output path.
- Row selection, scroll position, active filters, and grouped tree state survive media preview changes and locale changes.

### Implementation Plan

Use `docs/GUI_LARGE_INVENTORY_EXECUTION_PLAN.md` as the implementation sequence for this inventory redesign.
Use `docs/gui-large-inventory-traceability.md` to confirm every large inventory requirement has an implementation surface, acceptance criteria, and verification evidence before claiming completion.

1. Prototype the dense inventory layout in `gui/evidence-viewer/index.html`, `gui/evidence-viewer/styles.css`, and `gui/evidence-viewer/app.js` with generated 10k mock rows and virtualization.
2. Add a production inventory query contract over the existing SQLite case database before wiring the final Windows shell.
3. Add query-plan evidence for default filters, facet counts, search, and stable sorting.
4. Add generated review HTML support for paged/chunked inventory instead of embedding a full large JSON array.
5. Add keyboard and examiner workflow QA: arrow navigation, enter-to-open, shift multi-select, filter focus, and copy path/hash.
6. Add performance tests for 10k, 100k, and 1M synthetic tiers and record results in the Phase 9 large-scale report.
7. Only after those pass, carry the pattern into the WinUI 3 shell.

## Viewer Rules

- Original evidence is never modified.
- Proxy, thumbnail, frame capture, clip export, and contrast/zoom outputs are derived artifacts.
- Every derived artifact must link to source file ID, source hash when available, command parameters, operator, timestamp, and output hash.
- Carved files are displayed as candidates until playback/container validation is recorded.
- The viewer must distinguish `candidate-unvalidated`, `duplicate-candidate`, `ffprobe-video-stream-confirmed`, and `derived artifact`.

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

`make-review` also generates a real-case viewer at:

```text
case/review/evidence-viewer.html
```

That page reads the current video index, carving log, and validation log at generation time. It is still serverless, but unlike the GUI prototype it is tied to actual case data.

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
- A carved candidate must not become `ffprobe-video-stream-confirmed` until a core validation command records tool, method, operator, timestamp, source artifact, and audit chain.

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
