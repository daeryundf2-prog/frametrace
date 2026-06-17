# GUI Large Inventory Traceability Matrix

This matrix connects the user's large-file-list concern to executable FrameTrace GUI work, acceptance criteria, and verification evidence.

## Source Concern

The GUI must be redesigned because real cases can contain many files and the current file list shows too little information.

Baseline evidence:

- `docs/gui-large-inventory-baseline.md`
- `output/playwright/frametrace-inventory-baseline-1920.png`

## Requirement Matrix

| ID | Requirement | Current Evidence | Implementation Surface | Acceptance Criteria | Verification Evidence |
| --- | --- | --- | --- | --- | --- |
| GUI-INV-000 | Block GUI and WinUI work until completion/evidence/Stop-hook contracts cannot record false PASS. | ULW-loop and Stop hook P1 work is outside the GUI prototype. | ULW-loop CLI, Stop hook runtime, P1 verification artifact. | Structured PASS evidence, cleanup contract, HEAVY quality gate, bootstrap fallback, Stop hook blockers, and reconcile backup/receipt tests pass before GUI Phase 1. | P1 test transcript, manual CLI reconcile receipt, reviewer artifact. |
| GUI-INV-001 | Show more files without hiding viewer and inspector context. | Baseline shows only 6 mock rows and 58 px row height. | `gui/evidence-viewer/styles.css`, `gui/evidence-viewer/index.html` | >= 12 rows at 1440 px, >= 18 rows at 1920 px. | Browser row-count measurement and screenshots. |
| GUI-INV-002 | Provide inventory-focused mode for file-heavy review. | No current inventory-focused mode. | `gui/evidence-viewer/index.html`, `gui/evidence-viewer/styles.css`, `gui/evidence-viewer/app.js` | >= 30 visible rows while preserving selected file context. | Browser measurement plus state-preservation QA. |
| GUI-INV-003 | Keep critical file columns visible or available. | At 1440 px, type, preview, and size are hidden. | Inventory grid and detail drawer. | Status, review state, file ID, name, relative path, source, type, parser, validation, timestamp, size, hash state, report flag are available without report generation. | Column visibility check and detail drawer QA. |
| GUI-INV-004 | Avoid rendering all rows into DOM. | Current DOM row count equals data row count. | Virtualized inventory grid. | DOM rows <= viewport rows + overscan for 10k fixture. | DOM count measurement during scroll. |
| GUI-INV-005 | Prove prototype scale before production shell work. | Current prototype has only 6 mock records. | Deterministic mock generator in prototype. | 10,000 deterministic rows with stable IDs and mixed states. | Browser performance log and mock count check. |
| GUI-INV-006 | Use engine-backed search for production. | Current search filters local JavaScript array. | SQLite query layer, case DB indexes, generated viewer data contract. | 100k P95 search <= 1 second; 1M P99 search <= 3 seconds. | Query-plan evidence and performance report. |
| GUI-INV-007 | Provide grouped forensic drill-down. | Current source list is shallow. | Source tree and facet query layer. | source -> partition/container -> folder -> parser lane -> validation state counts exist. | Facet count QA with known fixture counts. |
| GUI-INV-008 | Preserve examiner context while previewing media. | Current behavior not measured for large inventory. | Selection, filter, scroll, source tree, and detail drawer state. | Preview changes do not reset selection, filters, scroll position, or grouped tree expansion. | Browser workflow QA. |
| GUI-INV-009 | Require auditable bulk preview before mutation. | Current buttons mutate prototype state immediately. | Bulk action preview UI and engine command preview. | Every bulk action displays selected count, filters, expected mutation, operator action, warnings, and audit output path before mutation. | Bulk preview QA checklist. |
| GUI-INV-010 | Prevent unbounded static review HTML. | `make-review` now builds generated review data from the first bounded SQLite inventory page and falls back to bounded `db/videos.jsonl` only for legacy cases. | `src/review_bundle.rs`, `src/cli/handlers.rs`, `src/html_report.rs`, generated review assets. | Production review output embeds at most 500 video rows and discloses `video_count`, `embedded_video_count`, `inventory_truncated`, `inventory_limit`, and the paging CLI contract. | `cargo test --test cli_inventory -- --nocapture`; `cargo test review_bundle -- --nocapture`; `cargo test evidence_viewer_discloses_bounded_inventory_subset -- --nocapture`. |
| GUI-INV-011 | Tie report selection back to forensic identifiers. | Current report flag is prototype state. | Case DB row projection, report set model, inspector. | Report-selected rows expose file ID, source ID, path, hash, parent artifact, and action history. | Report manifest QA. |
| GUI-INV-012 | Make large-case release claims measurable. | Phase 9 metrics exist. | Phase 9 performance and UX report. | All large inventory UI metrics pass or release claims are reduced. | `performance-report.md`, `performance-report.json`, Large file inventory QA report. |
| GUI-INV-013 | Use one SQLite-backed production inventory API contract. | Prototype still uses in-memory records for UI flows. | Rust engine, `case.db`, API/query functions, WinUI shell adapter. | `list_inventory`, `search_inventory`, `inventory_facets`, `get_file_detail`, `bulk_preview`, `apply_bulk_action`, and `export_manifest` all page results and return query/audit metadata. | API contract tests and SQLite fixture query-plan evidence. |
| GUI-INV-014 | Record audit evidence for every durable GUI mutation. | Prototype state changes are not durable forensic mutations. | Audit event writer, bulk preview/apply commands, report/export commands. | Durable mutations record operator, timestamp, tool version, preview ID, source/file IDs, hashes when available, before/after states, output paths, and output hashes. | Audit fixture diff and generated manifest/report review. |

## Implementation Order

1. Pass the P1 completion/evidence/Stop-hook contract gate.
2. Complete Phase 0 baseline and keep it as the comparison point.
3. Implement density modes and inventory-focused mode.
4. Implement deterministic 10k mock rows and virtual scrolling.
5. Implement filters, grouping, sorting, and bulk preview in prototype.
6. Define and implement SQLite-backed inventory query contract.
7. Update generated review HTML to avoid unbounded JSON.
8. Run forensic review QA and Phase 9 performance validation.

## Verification Checklist

- [ ] P1 completion/evidence/Stop-hook contract gate passed.
- [ ] Baseline document exists.
- [ ] 1440 px default row count measured.
- [ ] 1920 px default row count measured.
- [ ] Inventory-focused row count measured.
- [ ] 10k mock fixture exists.
- [ ] DOM row bound measured.
- [ ] 100k search latency measured.
- [ ] 1M SQLite search latency measured.
- [ ] Bulk preview audited.
- [ ] Report linkage audited.
- [x] Generated review HTML size bounded.
- [ ] Phase 9 evidence attached.

## Release Decision Rule

FrameTrace must not claim large-case GUI support unless every requirement in this matrix has passing evidence or the release notes explicitly reduce the supported scale.
