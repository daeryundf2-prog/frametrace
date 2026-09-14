// Repeatable evidence-viewer data-path benchmark.
//
// Measures the per-render cost of the viewer's filter + sort + facet passes
// on a synthetic 10k-record set — the operations that run on every UI
// interaction (search keystroke, chip click, page change). Card DOM is
// bounded by the 1000/page cap, so this is the scaling-sensitive part.
//
// Threshold: renders must stay under RENDER_BUDGET_MS on this machine.
// If this regresses past the budget, consider memoizing filteredRecords()
// or capping the page size before reaching for virtualization.
//
// Run: node scripts/bench-viewer.mjs
import { performance } from "node:perf_hooks";

const ROWS = 10_000;
const RENDERS = 100;
const RENDER_BUDGET_MS = 30;

const records = Array.from({ length: ROWS }, (_, i) => ({
  id: "vid_" + String(i).padStart(6, "0"),
  kind: "video",
  name: `CAM${i % 32}_2024-01-${String((i % 28) + 1).padStart(2, "0")}_${i}.mp4`,
  path: `E:/evidence/cam${i % 32}/file_${i}.mp4`,
  recDay: `2024-01-${String((i % 28) + 1).padStart(2, "0")}`,
  recTime: 1706000000 + i * 60,
  recType: ["normal", "event", "deleted-candidate"][i % 3],
  status: ["ffprobe-confirmed", "candidate-unvalidated", "validation-failed"][i % 4 === 3 ? 2 : i % 2],
  size: 10_000_000 + i,
  hasAnomaly: i % 17 === 0,
  parser: "generic",
  vendor: "hikvision",
  sha256: "abc" + i,
}));

const state = {
  kind: "", recType: "", status: "", chip: "",
  dateFrom: "", dateTo: "", query: "cam1",
  sortBy: "time-desc", tags: {}, marks: {},
};

// Mirrors filteredRecords() in assets/evidence_viewer.js — keep in sync.
function filteredRecords() {
  const list = records.filter(record => {
    if (state.kind && record.kind !== state.kind) return false;
    if (state.recType && (record.recType || "unclassified") !== state.recType) return false;
    if (state.status && record.status !== state.status) return false;
    if (state.chip === "anomaly" && !record.hasAnomaly) return false;
    if (state.dateFrom || state.dateTo) {
      if (!record.recDay) return false;
      if (state.dateFrom && record.recDay < state.dateFrom) return false;
      if (state.dateTo && record.recDay > state.dateTo) return false;
    }
    if (!state.query) return true;
    const haystack = [record.id, record.name, record.path, record.parser, record.vendor, record.sha256, record.status];
    return haystack.some(value => String(value ?? "").toLowerCase().includes(state.query));
  });
  const sorted = [...list];
  sorted.sort((a, b) => (b.recTime ?? -Infinity) - (a.recTime ?? -Infinity) || a.id.localeCompare(b.id));
  return sorted;
}

// One render() does: grid filter+sort, histogram pass over filtered,
// three metric filters, one facet pass over all records.
function simulateRender() {
  const filtered = filteredRecords();
  filtered.slice(0, 1000); // page cap
  const days = new Map();
  filteredRecords().forEach(record => {
    if (record.recDay) days.set(record.recDay, (days.get(record.recDay) || 0) + 1);
  });
  records.filter(record => record.status === "ffprobe-confirmed").length;
  records.filter(record => record.status === "validation-failed").length;
  records.filter(record => record.hasAnomaly).length;
  const facets = new Map();
  records.forEach(record => facets.set(record.recType, (facets.get(record.recType) || 0) + 1));
}

simulateRender(); // warmup
const t0 = performance.now();
for (let i = 0; i < RENDERS; i++) simulateRender();
const perRender = (performance.now() - t0) / RENDERS;

console.log(`viewer data path: ${ROWS} rows, ${perRender.toFixed(2)} ms/render over ${RENDERS} renders (budget ${RENDER_BUDGET_MS} ms)`);
if (perRender > RENDER_BUDGET_MS) {
  console.error("FAIL: render exceeds budget — memoize filteredRecords or reduce page size");
  process.exit(1);
}
console.log("OK");
