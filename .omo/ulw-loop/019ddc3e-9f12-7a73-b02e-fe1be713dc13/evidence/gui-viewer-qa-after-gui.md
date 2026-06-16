# GUI Viewer QA Evidence

checkedAt: 2026-06-16T16:27:33Z

## Static Checks

- `node --check gui/evidence-viewer/app.js`: pass
- `rg -n -P '<button(?![^>]*type=)' gui/evidence-viewer/index.html gui/evidence-viewer/app.js`: no matches
- Browser console after interactions: 0 errors, 0 warnings

## Large Inventory Browser Checks

- URL: `http://127.0.0.1:8766/index.html`
- Test dataset represented in UI: `10000개 일치`
- 1920x1080 default mode:
  - rendered DOM rows: 34
  - visible window: `1-34 표시`
  - toolbar overflow: false
  - button overflow: false
  - screenshot: `output/playwright/frametrace-inventory-1920-final.png`
- 1440x900 inventory focus mode:
  - query: `carved`
  - matched rows: `1666개 일치`
  - rendered DOM rows: 30
  - visible window: `1-30 표시`
  - viewer pane display: `none`
  - checked row after search: `carve_000009`
  - screenshot: `output/playwright/frametrace-inventory-focus-1440-final.png`
- Scroll virtualization:
  - scrollTop: 316800
  - visible window: `7193-7222 표시`
  - rendered DOM rows: 30
  - first rendered ID: `carve_007193`
  - last rendered ID: `img_007222`
- Search virtualization:
  - query: `carved`
  - matched rows: `1666개 일치`
  - rendered DOM rows: 30
  - latency: `3 ms`
  - selected and checked row synchronized to `carve_000009`
- Bulk preview:
  - action: validation queue
  - selected row: `carve_000009`
  - filter summary: `source=all; filter=all; query=carved`
  - mutation preview only: `validation job queue preview`
  - audit target shown: `evidence/logs/queue-validation-preview.jsonl`
  - activity log updated: `작업 미리보기 생성`

## Rust/CLI Regression Gates

- `cargo fmt --check`: pass
  - artifact: `.omo/ulw-loop/019ddc3e-9f12-7a73-b02e-fe1be713dc13/evidence/cargo-fmt-after-gui.txt`
- `cargo test -- --nocapture`: pass
  - artifact: `.omo/ulw-loop/019ddc3e-9f12-7a73-b02e-fe1be713dc13/evidence/full-cargo-test-after-gui.txt`
  - summary: 82 lib tests, 1 cli_inventory test, 3 cli_smoke tests, 0 failures
- `cargo clippy --all-targets --all-features -- -D warnings`: pass
  - artifact: `.omo/ulw-loop/019ddc3e-9f12-7a73-b02e-fe1be713dc13/evidence/full-cargo-clippy-after-gui.txt`

## Legal Claim Scan

- Remaining exact matches are only the report-defense denylist and its negative test fixture:
  - `src/qa_report_defense.rs`
  - `src/qa_tests.rs`
- No matching overclaim wording found in active GUI copy or docs outside denylist/test fixture.

## Cleanup Receipt

- Browser session: `frametrace-gui` closed.
- HTTP server: `python3 -m http.server 8766 --bind 127.0.0.1` stopped with keyboard interrupt.
- Port cleanup: `lsof -nP -iTCP:8766 -sTCP:LISTEN` returned no listeners.
- Browser cleanup: `playwright-cli list` returned `(no browsers)`.
