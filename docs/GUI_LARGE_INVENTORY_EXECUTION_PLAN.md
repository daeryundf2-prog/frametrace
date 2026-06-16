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

Known gaps:

- Too few rows are visible in the file list.
- File list columns are hidden aggressively at normal widths.
- No inventory-focused mode exists.
- No grouped source/folder/parser drill-down exists.
- Search/filter is browser-array based, not engine-backed.
- No paging or virtualization contract exists in implementation.
- No bulk-action preview exists before review/report/hash/validation mutations.
- Generated review HTML risks embedding too much case index data if scaled naively.

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
