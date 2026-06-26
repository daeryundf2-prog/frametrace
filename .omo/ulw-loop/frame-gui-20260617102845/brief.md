FrameTrace 2차 작업: 대용량 forensic inventory GUI / SQLite query layer / HTML prototype.
Prerequisite gate passed before GUI work: ULW npm test, FrameTrace cargo test, status --json session-scoped paths, reconcile command, Stop hook/state/quality/bootstrap evidence confirmed.
Goals:
1. Fix production SQLite inventory query/API/audit contract in repo docs/tests.
2. Implement or harden SQLite-backed inventory query layer so list/search/facets/detail/bulk preview/export semantics are bounded and audit-aware.
3. Implement gui/evidence-viewer HTML prototype with dense inventory, virtualization, Korean-first examiner workflow, 10k mock rows, search/filter/facet/sort/bulk preview behavior.
4. Prove 100k/1M production path does not load whole JSON in browser and generated review HTML large-case policy remains bounded/disclosed.
5. Do not implement WinUI 3 shell in this phase.
