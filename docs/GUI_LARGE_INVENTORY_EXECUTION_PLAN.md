# GUI Large Inventory Execution Plan

FrameTrace needs a file-heavy forensic workstation GUI. The current prototype proves the viewer workflow, but the file inventory must become a dense, scalable review surface for cases with thousands to millions of rows.

## Objective

Build a GUI inventory experience that lets an examiner quickly answer:

- How many files exist in the case?
- Which files are videos, photos, carved candidates, recovered files, derived artifacts, or validation risks?
- Which source, folder, parser lane, and validation state produced each file?
- Which files are already reviewed, selected for report, hashed, validated, exported, duplicated, or unresolved?
- Which exact row set will be changed before a bulk action is recorded?

## Current Gap

The prototype currently uses a small in-memory JavaScript `records` array and renders visible evidence rows directly into the DOM. That is acceptable for workflow demonstration, but not for real cases.

Known gaps before this slice:

- Too few rows are visible in the file list.
- File list columns are hidden aggressively at normal widths.
- No grouped source/folder/parser drill-down exists.
- Generated review HTML risks embedding too much case index data if scaled naively.

Current implementation boundaries:

- The HTML prototype uses deterministic 10,000-row mock data, fixed-height virtualization, density modes, inventory-focused mode, stable sort controls, and non-mutating bulk preview UI.
- The Rust/SQLite query layer exposes bounded page metadata for production paths.
- The generated review HTML embeds only a bounded inventory subset and discloses the paging command for full review.
- The remaining production gap is the SQLite-backed GUI adapter; the prototype must not be treated as the production large-case transport. Current SQLite performance QA passed at 100k and 1M synthetic rows.

## Pre-GUI P1 Contract Gate

Large inventory GUI work is blocked until the completion, evidence, resume, and Stop hook contracts are passing. This gate is intentionally before SQLite-backed viewer work and before any WinUI shell implementation.

Purpose:

- Prevent the project from claiming a finished GUI or forensic workflow while ULW-loop, Stop hook, or resume state can still record false completion.

Entry criteria:

- `omo ulw-loop status --json` or an equivalent ULW-loop CLI surface returns session-scoped `briefPath`, `goalsPath`, `ledgerPath`, and `evidenceDir`.
- No workflow instruction depends on hand-editing JSON state as a normal success path.
- The official Stop hook reconciliation command exists and requires a session ID, complete canonical ULW-loop state, and fresh verification evidence.

Required validation:

- PASS evidence tests prove that text-only evidence, missing cleanup receipts, missing `cleanup:not-applicable` reasons, and remaining process/browser/worker flags cannot pass.
- Checkpoint tests use the same PASS validator as `record-evidence`.
- Quality gate tests prove `tier` is required, HEAVY cannot use the LIGHT self-review bypass, and HEAVY requires structured reviewer fields plus reviewer artifact.
- Bootstrap smoke tests prove `omo`, `omo-ulw-loop`, executable wrappers, and `cli.js` fallback are probed distinctly.
- Stop hook tests prove blocking output includes the blocking state file path, session ID, active status, and phase.
- Reconciliation tests prove session mismatch, missing evidence, stale evidence, and incomplete canonical state do not mutate hook state.
- Reconciliation tests prove every JSON mutation creates a timestamped backup, uses atomic write, and writes a receipt.

Exit criteria:

- A fresh verification artifact is recorded in the canonical ULW-loop `evidenceDir`.
- The reconciliation receipt records the canonical `briefPath`, `goalsPath`, `ledgerPath`, and `evidenceDir`.
- GUI Phase 1 or WinUI work does not start until this P1 gate is marked passing.

## Authoritative References

- `docs/EVIDENCE_VIEWER_GUI.md`
- `docs/FORENSIC_HARDENING_PLAN.md`, Phase 9
- `docs/gui-large-inventory-baseline.md`
- `docs/gui-large-inventory-traceability.md`
- `gui/evidence-viewer/index.html`
- `gui/evidence-viewer/styles.css`
- `gui/evidence-viewer/app.js`
- `src/case_db/*`
- `src/html_report.rs`

## Execution Sequence

### Phase P1: Completion/Evidence/Hook Contract Gate

Purpose:

- Close the false-PASS and stale Stop hook failure modes before GUI implementation.

Tasks:

1. Run ULW-loop PASS evidence, checkpoint, quality gate, bootstrap, and reconcile command tests.
2. Run Stop hook runtime tests for structured blockers and reconciliation backup/receipt behavior.
3. Capture one manual CLI scenario showing a complete canonical ULW-loop run reconciling stale session-scoped hook state.
4. Record the evidence path, cleanup status, and reviewer result.

Exit criteria:

- P1 validation passes with fresh command output.
- Any remaining P1 failure blocks GUI Phase 1 and WinUI work.

### Phase 0: Baseline Measurement

Purpose:

- Measure the current prototype before redesign.

Tasks:

1. Add or run a local GUI measurement harness that opens `gui/evidence-viewer/index.html`.
2. Capture baseline visible row count at 1440 px and 1920 px.
3. Capture current search/render behavior with the existing mock records.
4. Record a screenshot and metrics in `docs/gui-large-inventory-baseline.md`.

Exit criteria:

- Baseline row count, layout screenshot path, and current limitations are documented.
- `docs/gui-large-inventory-baseline.md` exists and records the measurement method, viewport results, screenshot path, and cleanup receipt.
- `docs/gui-large-inventory-traceability.md` maps each large inventory requirement to implementation surface, acceptance criteria, and evidence.

### Phase 1: Dense Inventory Prototype

Purpose:

- Make the file list show substantially more evidence without hiding forensic context.

Tasks:

1. Add compact/normal/media row density modes.
2. Add inventory-focused mode that collapses the viewer into a preview strip.
3. Replace the narrow current table with a dense grid: status, review, file ID, name, relative path, source, type, parser, validation, timestamp, size, hash, report.
4. Add sticky header, resizable column placeholders, and stable row height.
5. Preserve viewer and inspector visibility in default mode.

Exit criteria:

- 1440 px default layout shows at least 12 rows.
- 1920 px default layout shows at least 18 rows.
- Inventory-focused mode shows at least 30 rows.

### Phase 2: Virtualized List and Mock Scale

Purpose:

- Prove the browser can handle large inventory sets without rendering every row.

Tasks:

1. Generate deterministic 10,000-row mock inventory data in the prototype.
2. Implement fixed-height virtual scrolling with overscan.
3. Ensure scroll position and selected row survive filtering, sorting, preview changes, and locale changes.
4. Lazy-render thumbnails only for visible rows.
5. Add visible match count and current query latency display.

Exit criteria:

- DOM row count remains bounded by viewport plus overscan.
- 10,000-row render P95 <= 2 seconds.
- Scroll P95 frame time <= 32 ms in prototype measurement.

### Phase 3: Filtering, Facets, and Bulk Preview

Purpose:

- Let examiners reduce huge cases into defensible working sets.

Tasks:

1. Add composable filters: source, path prefix, type, parser lane, validation state, review state, hash state, report flag, size range, time range, recovered/candidate state.
2. Add grouped tree counts by source -> partition/container -> folder -> parser lane -> validation state.
3. Add stable sorting with default `risk desc`, `timestamp asc`, `file_id asc`.
4. Add multi-select and selection bar.
5. Add `bulk_preview` UI for mark reviewed, report set, queue hash, queue validation, queue proxy/thumbnail, and export manifest.

Exit criteria:

- Bulk actions show selected count, active filters, expected mutation, operator action, and audit output path before any state change.
- Empty states distinguish no evidence, no search matches, and all rows hidden by filters.

### Phase 4: SQLite-Backed Inventory Contract

Purpose:

- Move production inventory behavior away from full JSON-in-browser loading.

Tasks:

1. Define inventory row projection over `case.db`.
2. Add query functions for `list_inventory`, `search_inventory`, `inventory_facets`, `get_file_detail`, and `bulk_preview`.
3. Add indexes for source, type, parser lane, validation state, review state, report state, timestamp, size, hash state, and path search.
4. Add query-plan evidence for default views and common forensic filters.
5. Ensure generated review HTML uses paged/chunked inventory data or a bounded static subset with disclosed limits.

Implemented guardrail:

- `make-review` must use the bounded review bundle from `case.db` first and must not embed the full legacy `db/video_index.json` array. The generated HTML discloses total rows, embedded rows, truncation state, the 500-row embed limit, and the paging command for full SQLite inventory review.

Exit criteria:

- 100,000-row fixture search P95 <= 1 second.
- 1,000,000-row synthetic SQLite search P99 <= 3 seconds.
- Browser memory does not scale with total row count.

## Production Data/API/Audit/Performance Contract

Source of truth:

- Rust engine and SQLite `case.db` are the authoritative production state.
- Browser HTML, JSON, JSONL, TSV, thumbnails, proxies, frame captures, clips, and reports are derived artifacts.
- Production search, filter, facet, sort, and bulk preview are engine-backed queries; they must not load 100k or 1M inventory rows into browser memory.

Inventory API surfaces:

| Surface | Purpose | Required behavior |
| --- | --- | --- |
| `list_inventory` | Page inventory rows. | Accepts filters, sort, offset, and `limit <= 500`; returns rows, `next_cursor`, `total_rows`, `query_id`, `duration_ms`, and truncation state. CLI: `frametrace inventory <case_dir> --limit <n> --offset <n> --sort <sort>`. |
| `search_inventory` | Keyword and field search. | Uses SQLite indexes/FTS where available; returns page-sized results and exact or disclosed approximate counts. |
| `inventory_facets` | Group counts. | Returns counts for source, type, parser lane, validation state, review state, report state, extension, and hash state. CLI: `frametrace inventory <case_dir> --facets`. |
| `get_file_detail` | Inspect a row. | Returns full row projection for path, source, validation, hash, report, and timestamp fields. CLI: `frametrace inventory <case_dir> --file-id <file_id>`. |
| `bulk_preview` | Preview mutations. | Returns selected count, missing IDs, warnings, expected mutation, audit preview ID, and output path before mutation. CLI: `frametrace inventory-bulk-preview ...`. |
| `apply_bulk_action` | Apply reviewed mutation. | Requires a valid preview ID and writes audit events before durable case state changes. Not implemented in this HTML prototype slice. |
| `export_manifest` | Export selected set. | Writes a manifest with selected file IDs, source IDs, hashes, paths, output SHA-256, missing IDs, filters, and large-case policy. CLI: `frametrace inventory-export-manifest ...`. |

Audit contract:

- Every durable mutation records operator, timestamp, command/tool version, case ID, preview ID when applicable, source file IDs, source hashes when available, row IDs or query/filter snapshot, before/after states, output paths, and output hashes.
- Bulk action preview is mandatory before mark-reviewed, report-set, hash-queue, validation-queue, proxy/thumbnail generation, clip export, and manifest export mutations.
- Candidate carved files remain `candidate-unvalidated` until a validation command records tool, method, operator, timestamp, source artifact, source hash when available, validation artifact, and audit chain.
- Direct original media playback is an audited exception; the default playback target is a validated proxy.

Generated review HTML policy:

- `make-review` embeds at most 500 inventory rows.
- It must disclose total rows, embedded rows, truncation status, embed limit, and the full SQLite paging command.
- It must not be used as the transport for full 100k or 1M inventory browsing.
- Large-case browsing must use SQLite-backed page/search/facet queries.

Performance contract:

| Scenario | Pass/fail target |
| --- | --- |
| 10k prototype mock initial render | P95 <= 2 seconds |
| 10k prototype scroll | P95 frame time <= 32 ms |
| 100k SQLite fixture search | P95 <= 1 second |
| 1M SQLite synthetic search | P99 <= 3 seconds |
| Inventory row selection UI | P95 <= 100 ms |
| Cached page filter/facet refresh | P95 <= 250 ms |
| Browser inventory memory | Does not scale with total row count |
| Bulk preview generation | 100% of durable mutations require preview |

Implementation path:

1. HTML prototype proves density, virtualization, filters, and preview interaction with deterministic mock data.
2. SQLite-backed prototype replaces browser-array search/filter/facet/sort with engine-backed paged queries.
3. WinUI 3 shell consumes the same SQLite-backed API contract and does not introduce a separate source of truth.

### Phase 5: Forensic Review QA

Purpose:

- Prove the redesigned inventory supports examiner work, not just rendering speed.

Tasks:

1. Run keyboard QA: arrow navigation, enter-to-open, shift multi-select, filter focus, copy path, copy hash.
2. Run workflow QA: triage candidates, find unvalidated recovered files, bulk queue validation, add report set, inspect parent/child derived artifact chain.
3. Run preservation QA: opening media does not reset filters, scroll, selection, source tree, or detail drawer state.
4. Run legal/report QA: selected report set can produce a manifest with file IDs, paths, hashes, source IDs, and action history.

Exit criteria:

- `Large file inventory QA report` is attached to Phase 9 evidence.
- All acceptance criteria in `docs/EVIDENCE_VIEWER_GUI.md` pass or the release claim is reduced.

## Data Model Requirements

Inventory row fields:

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

Derived fields:

- `risk_rank`
- `display_path`
- `thumbnail_state`
- `selection_eligible`
- `bulk_action_warnings`

## UI Acceptance Matrix

| Requirement | Target | Evidence |
| --- | --- | --- |
| Default visible rows at 1440 px | >= 12 | Screenshot + row count |
| Default visible rows at 1920 px | >= 18 | Screenshot + row count |
| Inventory-focused visible rows | >= 30 | Screenshot + row count |
| 10k initial render | P95 <= 2 seconds | Browser performance log |
| 10k scroll frame time | P95 <= 32 ms | Browser performance log |
| 100k search | P95 <= 1 second | Query/report log |
| 1M search | P99 <= 3 seconds | SQLite query/report log |
| Browser memory | Does not grow with total row count | Heap/RSS measurement |
| Bulk preview | 100% before mutation | QA checklist |
| State preservation | 100% tested flows pass | QA checklist |

## Release Blockers

- Full large JSON index is loaded into the browser for production.
- DOM renders all inventory rows.
- Bulk actions mutate case state without preview.
- Search/filter lacks total match count.
- Opening media resets filter, scroll, or selection state.
- Report set cannot tie rows back to file IDs, source IDs, hashes, and action history.
- Phase 9 metrics are missing or fail without reduced release claims.

## Planning Quality Gate

This plan is implementation-ready only when the following checks are true:

- The GUI work starts with Phase 0 baseline measurement, not direct visual changes.
- Prototype work is limited to `gui/evidence-viewer/*` until the SQLite-backed contract is designed.
- Engine-backed production work does not begin until inventory row projection and indexes are specified.
- Every performance claim has a named fixture tier: 10k browser mock, 100k fixture, or 1M SQLite synthetic.
- Every bulk action has a preview state before mutation.
- Every mutation produces an audit path, even in prototype form.
- Generated review HTML cannot embed unbounded inventory JSON.
- The final Windows shell cannot claim large-case support before Phase 9 metrics pass.

Hard blockers:

- No target viewport can show at least 12 inventory rows without hiding both viewer and inspector.
- No bounded DOM strategy exists for 10k rows.
- No SQLite-backed search/filter contract exists for 100k+ rows.
- No proof exists that selection/filter/scroll state survives media preview changes.
- No report linkage exists from selected rows back to source IDs, file IDs, hashes, and action history.

## Implementation Notes

- Prototype changes should stay serverless until the query contract is ready.
- Production shell should use engine-backed paging/search.
- Thumbnails are optional row decorations and must not be required for fast scrolling.
- The file inventory must support review without requiring every file to be opened in the media viewer.
- Evidence values such as paths, hashes, parser IDs, and source IDs remain verbatim across localization.
