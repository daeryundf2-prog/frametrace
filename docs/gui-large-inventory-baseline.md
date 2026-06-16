# GUI Large Inventory Baseline

Measured on 2026-06-16 in the local FrameTrace static evidence viewer.

## Scope

This baseline captures the current `gui/evidence-viewer` prototype before the large inventory redesign begins. It is the Phase 0 evidence for `docs/GUI_LARGE_INVENTORY_EXECUTION_PLAN.md`.

## Method

Viewer under test:

```text
gui/evidence-viewer/index.html
```

The Playwright CLI blocks direct `file://` navigation, so the prototype was served read-only through a temporary localhost server:

```text
python3 -m http.server 8766 --bind 127.0.0.1
```

Cleanup receipt:

```text
Temporary server stopped with keyboard interrupt after measurement.
```

Screenshot evidence:

```text
output/playwright/frametrace-inventory-baseline-1920.png
```

## Measurements

### 1440 x 1000

```json
{
  "viewport": { "width": 1440, "height": 1000 },
  "totalDataRows": 6,
  "visibleDataRows": 6,
  "fileRowsClientHeight": 348,
  "fileRowsScrollHeight": 348,
  "rowHeights": [58],
  "headerColumns": ["상태", "시각", "파일", "유형", "미리보기", "크기"],
  "firstRowVisibleCells": [
    { "index": 1, "display": "flex", "meaning": "status" },
    { "index": 2, "display": "block", "meaning": "time" },
    { "index": 3, "display": "block", "meaning": "file path/name" },
    { "index": 4, "display": "none", "meaning": "type" },
    { "index": 5, "display": "none", "meaning": "preview" },
    { "index": 6, "display": "none", "meaning": "size" }
  ]
}
```

### 1920 x 1080

```json
{
  "viewport": { "width": 1920, "height": 1080 },
  "totalDataRows": 6,
  "visibleDataRows": 6,
  "fileRowsClientHeight": 348,
  "fileRowsScrollHeight": 348,
  "rowHeights": [58],
  "headerColumns": ["상태", "시각", "파일", "유형", "미리보기", "크기"],
  "firstRowVisibleCells": [
    { "index": 1, "display": "flex", "meaning": "status" },
    { "index": 2, "display": "block", "meaning": "time" },
    { "index": 3, "display": "block", "meaning": "file path/name" },
    { "index": 4, "display": "block", "meaning": "type" },
    { "index": 5, "display": "block", "meaning": "preview" },
    { "index": 6, "display": "block", "meaning": "size" }
  ]
}
```

## Findings

- The current prototype renders only 6 mock records, so it cannot demonstrate large-case behavior.
- The current DOM row count equals the data row count. There is no virtualization evidence yet.
- The row height is 58 px, which is too tall for the large inventory targets.
- At 1440 px, type, preview, and size cells are hidden. This confirms the user's concern that the file list shows too little information.
- At 1920 px, all current columns are visible, but only 6 rows exist and the layout still does not prove 12/18/30-row density targets.
- The current list uses a browser-local array and does not prove SQLite-backed search, facets, stable sort, paging, or bounded memory.

## Redesign Implications

- Phase 1 must reduce default row height and add density modes before visual polish.
- Phase 2 must introduce deterministic 10,000-row mock data and bounded DOM virtualization.
- Phase 3 must add grouped facets and bulk preview because the current list cannot explain large row sets.
- Phase 4 must move production inventory reads to SQLite-backed paging/search before any large-case support claim.

## Baseline Verdict

Current GUI inventory status:

```text
Not large-case ready.
```

This is expected for the prototype and is now documented as the baseline to improve against.
