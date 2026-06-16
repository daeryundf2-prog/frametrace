# SQLite-Bounded Review Bridge Evidence

Date: 2026-06-16

Scope:

- Added a bounded generated-review inventory bundle.
- `make-review` now uses `case.db` inventory rows first and falls back to bounded `db/videos.jsonl` only when no SQLite case DB exists.
- Generated review HTML discloses `video_count`, `embedded_video_count`, `inventory_truncated`, `inventory_limit`, and `inventory_query_contract`.
- Evidence viewer displays a visible truncation notice when the embedded static subset is incomplete.

Changed implementation:

- `src/review_bundle.rs`
- `src/cli/handlers.rs`
- `src/html_report.rs`
- `src/lib.rs`
- `tests/cli_inventory.rs`
- `docs/gui-large-inventory-traceability.md`
- `docs/GUI_LARGE_INVENTORY_EXECUTION_PLAN.md`

Red/green evidence:

- `cargo test review_bundle -- --nocapture`
  - Red before implementation: 2 tests failed against `{}` stub.
  - Green after implementation: 2 passed.
- `cargo test make_review_embeds_bounded_inventory_subset -- --nocapture`
  - Red before `make-review` integration: missing `embedded_video_count`.
  - Green after integration: passed.
- `cargo test make_review_uses_sqlite_inventory_when_jsonl_is_absent -- --nocapture`
  - Red before SQLite-first implementation: missing `inventory_source:"case.db/videos"`.
  - Green after SQLite-first implementation: passed.
- `cargo test evidence_viewer_discloses_bounded_inventory_subset -- --nocapture`
  - Red before viewer notice: missing `id="inventoryNotice"`.
  - Green after viewer notice: passed.

Full verification:

- `cargo test -- --nocapture`
  - 85 library tests passed.
  - 3 `cli_inventory` integration tests passed.
  - 3 `cli_smoke` integration tests passed.
  - Doc tests passed.
- `cargo clippy --all-targets --all-features -- -D warnings`
  - Passed.
- `cargo fmt --check`
  - Passed.
- `git diff --check`
  - Passed.
- `rg -n "court-ready|court-grade|court-proven|legal-grade|legal proof|guaranteed legal readiness|legal readiness" docs gui src tests`
  - Hits are denylist/policy/negative-test references only:
    - `src/qa_report_defense.rs`
    - `src/qa_tests.rs`
    - `docs/FORENSIC_HARDENING_PLAN.md`

Cleanup:

- No HTTP server, browser, tmux session, external worker, or long-running process was spawned for this slice.
- Cleanup status: not-applicable.

Known limits:

- `src/html_report.rs` and `src/cli/handlers.rs` remain oversized legacy files. This slice avoided adding new embedded-review logic there beyond the minimum call-site and visible-notice change.
- This is audit evidence only. The active ULW goal criteria are still placeholder criteria and were not marked PASS.
