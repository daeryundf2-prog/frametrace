# GUI Large Inventory Verification Evidence

Captured: 2026-06-17

## Browser Visual QA

Target: `gui/evidence-viewer/index.html`

Browser: Google Chrome headless through Playwright.

| Scenario | Result | Artifact |
| --- | --- | --- |
| 1440 px default inventory | `recordsLength=10000`, `dataRowsInDom=30`, `estimatedVisibleRows=13`, `rowHeight=40`, no page errors | `output/playwright/frametrace-inventory-1440-current.png` |
| 1920 px default inventory with `size-desc` sort | `recordsLength=10000`, `dataRowsInDom=35`, `estimatedVisibleRows=18`, `rowHeight=40`, no page errors | `output/playwright/frametrace-inventory-1920-current.png` |
| 1440 px focus mode | `recordsLength=10000`, `dataRowsInDom=47`, `estimatedVisibleRows=30`, `rowHeight=24`, `bodyClass=inventory-focused`, no page errors | `output/playwright/frametrace-inventory-focus-1440-current.png` |
| Bulk preview | Drawer visible; `height=154`; preview includes preview ID, operator, filters, expected mutation, audit path, and warning; no page errors | `output/playwright/frametrace-inventory-bulk-preview-current.png` |

Edge/regression QA artifact:

- `.omo/ulw-loop/frame-gui-20260617102845/evidence/gui-edge-regression-qa.json`
- Result: passed; empty search rendered `0` rows; scroll to `200000` kept DOM rows at `32`; no-selection export preview displayed `선택된 행 없음`; no page errors.

## SQLite Large-Case QA

| Scenario | Command | Result |
| --- | --- | --- |
| 100k SQLite performance QA | `target/debug/frametrace qa performance /tmp/frametrace-inventory-bench-100k --rows 100000` | passed; max query 4 ms; full scan count 0; max RSS 9,240,576 bytes |
| 1M SQLite performance QA | `target/debug/frametrace qa performance /tmp/frametrace-inventory-bench-1m --rows 1000000` | passed; max query 570 ms; full scan count 0; max RSS 9,338,880 bytes |

Durable JSON artifacts:

- `.omo/ulw-loop/frame-gui-20260617102845/evidence/performance-report-100k.json`
- `.omo/ulw-loop/frame-gui-20260617102845/evidence/performance-report-1m.json`

## Test Commands

- `cargo fmt --all -- --check`
- `node --check gui/evidence-viewer/app.js`
- `cargo test inventory_tests -- --nocapture`
- `cargo test --test cli_inventory -- --nocapture`
- `cargo test review_bundle -- --nocapture`
- `cargo test evidence_viewer_discloses_bounded_inventory_subset -- --nocapture`
- `cargo test`

## Scope Boundary

WinUI 3 shell implementation was intentionally not started in this phase.
