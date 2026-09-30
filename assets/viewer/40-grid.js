const els = {
  caseLine: document.getElementById("caseLine"),
  resultCount: document.getElementById("resultCount"),
  triageStatus: document.getElementById("triageStatus"),
  metricVideos: document.getElementById("metricVideos"),
  metricCarved: document.getElementById("metricCarved"),
  metricVerified: document.getElementById("metricVerified"),
  metricFailed: document.getElementById("metricFailed"),
  metricAnomaly: document.getElementById("metricAnomaly"),
  query: document.getElementById("query"),
  kind: document.getElementById("kind"),
  status: document.getElementById("status"),
  pageSize: document.getElementById("pageSize"),
  presetChips: document.getElementById("presetChips"),
  recordGrid: document.getElementById("recordGrid"),
  prevPage: document.getElementById("prevPage"),
  nextPage: document.getElementById("nextPage"),
  pageStatus: document.getElementById("pageStatus"),
  mediaTitle: document.getElementById("mediaTitle"),
  mediaStatus: document.getElementById("mediaStatus"),
  mediaStage: document.getElementById("mediaStage"),
  detailBadges: document.getElementById("detailBadges"),
  summaryList: document.getElementById("summaryList"),
  metaList: document.getElementById("metaList"),
  validationList: document.getElementById("validationList"),
  selectionBar: document.getElementById("selectionBar"),
  selectionCount: document.getElementById("selectionCount"),
  videoMode: document.getElementById("videoMode"),
  videoZoom: document.getElementById("videoZoom"),
  playRate: document.getElementById("playRate"),
  facetTree: document.getElementById("facetTree"),
  sortBy: document.getElementById("sortBy"),
  groupBy: document.getElementById("groupBy"),
  tagFilter: document.getElementById("tagFilter"),
  dateFrom: document.getElementById("dateFrom"),
  dateTo: document.getElementById("dateTo"),
  dayHistogram: document.getElementById("dayHistogram"),
  periodLabel: document.getElementById("periodLabel"),
  dflSummary: document.getElementById("dflSummary"),
  caseWarnings: document.getElementById("caseWarnings")
};

// [value, label key, secondary] — secondary chips hide behind the
// "더보기" expander; their filters still work via the status/mark selects.
const PRESET_CHIPS = [
  ["", "chip.all"],
  ["kind:recovery", "chip.recovery"],
  ["status:candidate-unvalidated", "chip.candidate"],
  ["mark:important", "chip.markImportant"],
  ["mark:none", "chip.markNone"],
  ["marked:any", "chip.marked", true],
  ["anomaly", "chip.anomaly", true],
  ["warning", "chip.warning", true],
  ["status:validation-failed", "chip.failed", true],
  ["status:duplicate-candidate", "chip.duplicate", true],
  ["mark:reviewed", "chip.markReviewed", true],
];

function filteredRecords() {
  const list = records.filter(record => {
    if (state.kind && record.kind !== state.kind) return false;
    if (state.recType && (record.recType || "unclassified") !== state.recType) return false;
    if (state.status && record.status !== state.status) return false;
    if (state.chip === "anomaly" && !record.hasAnomaly) return false;
    if (state.chip === "warning" && !(record.warnings || []).length) return false;
    if (state.chip === "kind:recovery" && record.kind !== "candidate" && record.kind !== "filesystem") return false;
    if (state.chip === "marked:any" && !state.marks[record.id] && !(state.tags[record.id] || []).length && !(state.notes[record.id] || "").trim()) return false;
    if (state.dateFrom || state.dateTo) {
      if (!record.recDay) return false;
      if (state.dateFrom && record.recDay < state.dateFrom) return false;
      if (state.dateTo && record.recDay > state.dateTo) return false;
    }
    if (state.chip.startsWith("tag:")) {
      const wanted = state.chip.slice(4);
      const tags = state.tags[record.id] || [];
      if (!tags.includes(wanted)) return false;
    }
    if (state.chip.startsWith("mark:")) {
      const wanted = state.chip.slice(5);
      const mark = state.marks[record.id];
      if (wanted === "none") { if (mark) return false; }
      else if (!mark || mark.status !== wanted) return false;
    }
    if (!state.query) return true;
    const haystack = [record.id, record.name, record.path, record.originalPath, record.parser, record.vendor, record.sha256, record.note, record.status];
    return haystack.some(value => String(value ?? "").toLowerCase().includes(state.query));
  });
  const sorted = [...list];
  switch (state.sortBy) {
    case "time-desc":
      sorted.sort((a, b) => (b.recTime ?? -Infinity) - (a.recTime ?? -Infinity) || a.id.localeCompare(b.id));
      break;
    case "time-asc":
      sorted.sort((a, b) => (a.recTime ?? Infinity) - (b.recTime ?? Infinity) || a.id.localeCompare(b.id));
      break;
    case "name":
      sorted.sort((a, b) => a.name.localeCompare(b.name, "ko") || a.id.localeCompare(b.id));
      break;
    case "size-desc":
      sorted.sort((a, b) => (b.size || 0) - (a.size || 0) || a.id.localeCompare(b.id));
      break;
    default:
      sorted.sort((a, b) => a.id.localeCompare(b.id));
  }
  return sorted;
}

function groupKeyFor(record) {
  switch (state.groupBy) {
    case "day": return record.recDay || t("group.noTime");
    case "kind": return ({ video: t("filter.kind.video"), carved: t("filter.kind.carved"), filesystem: t("filter.kind.filesystem"), candidate: t("filter.kind.candidate") })[record.kind] || record.kind;
    case "recType": return recTypeLabel(record.recType);
    case "status": return statusLabel(record.status);
    case "mark": return markOf(record) ? markLabel(markOf(record).status) : t("group.noMark");
    case "prefix": return record.prefix;
    case "channel": return record.channel || t("group.noChannel");
    default: return "";
  }
}

function selectedRecord() { return records.find(record => record.id === state.activeId); }
function markOf(record) { return state.marks[record.id] || null; }

function saveLayout() {
  const { videoMode, videoZoom, theater, dual, playerH, colSplit, rate } = state.layout;
  storageSet(LAYOUT_KEY, { videoMode, videoZoom, theater, dual, playerH, colSplit, rate });
}

let layoutSaveTimer = null;
function saveLayoutSoon() {
  clearTimeout(layoutSaveTimer);
  layoutSaveTimer = setTimeout(saveLayout, 250);
}

function applyLayout() {
  document.body.classList.toggle("theater", !!state.layout.theater);
  const player = document.querySelector(".player");
  const h = Number(state.layout.playerH) || 0;
  player.style.height = h >= 160 ? h + "px" : "";
  applyColumnSplit();
  applyVideoScale();
  els.videoMode.value = state.layout.videoMode;
  els.videoZoom.value = String(state.layout.videoZoom);
  els.playRate.value = String(state.layout.rate || 1);
  const rateVideo = els.mediaStage.querySelector("video");
  if (rateVideo) rateVideo.playbackRate = state.layout.rate || 1;
  document.getElementById("btnDual")?.classList.toggle("on", !!state.layout.dual);
}

// Column split between the browse pane and the media pane. The default lives
// in CSS (55%/45%); a dragged value is stored as the left pane percentage.
function applyColumnSplit() {
  const main = document.querySelector(".main");
  const pct = Number(state.layout.colSplit) || 0;
  if (pct >= 28 && pct <= 72) {
    main.style.gridTemplateColumns = `minmax(0, ${pct}%) 10px minmax(0, 1fr)`;
  } else {
    main.style.gridTemplateColumns = "";
  }
}

function setupColumnSplitter() {
  const splitter = document.getElementById("vsplitter");
  const main = document.querySelector(".main");
  let dragging = false;
  splitter.addEventListener("pointerdown", event => {
    dragging = true;
    splitter.classList.add("dragging");
    splitter.setPointerCapture(event.pointerId);
    event.preventDefault();
  });
  splitter.addEventListener("pointermove", event => {
    if (!dragging) return;
    const rect = main.getBoundingClientRect();
    const pct = ((event.clientX - rect.left) / rect.width) * 100;
    state.layout.colSplit = Math.max(28, Math.min(72, pct));
    applyColumnSplit();
  });
  const end = () => {
    if (!dragging) return;
    dragging = false;
    splitter.classList.remove("dragging");
    saveLayout();
  };
  splitter.addEventListener("pointerup", end);
  splitter.addEventListener("dblclick", () => {
    state.layout.colSplit = 0;
    saveLayout();
    applyLayout();
  });
}

function setupHeightSplitter() {
  const splitter = document.getElementById("hsplitter");
  const player = document.querySelector(".player");
  let startY = 0;
  let startH = 0;
  let dragging = false;
  splitter.addEventListener("pointerdown", event => {
    dragging = true;
    startY = event.clientY;
    startH = player.offsetHeight;
    splitter.classList.add("dragging");
    splitter.setPointerCapture(event.pointerId);
    event.preventDefault();
  });
  splitter.addEventListener("pointermove", event => {
    if (!dragging) return;
    state.layout.playerH = Math.max(160, Math.min(window.innerHeight - 260, startH + event.clientY - startY));
    applyLayout();
  });
  const end = () => {
    if (!dragging) return;
    dragging = false;
    splitter.classList.remove("dragging");
    saveLayout();
  };
  splitter.addEventListener("pointerup", end);
  splitter.addEventListener("dblclick", () => {
    state.layout.playerH = 0;
    saveLayout();
    applyLayout();
  });
}

function applyVideoScale() {
  const mode = state.layout.videoMode;
  const zoom = Math.max(30, Math.min(300, Number(state.layout.videoZoom) || 100));
  els.mediaStage.classList.toggle("mode-fit", mode === "fit");
  els.mediaStage.classList.toggle("mode-zoom", mode !== "fit");
  const video = els.mediaStage.querySelector("video");
  if (video && mode !== "fit") {
    const native = video.videoWidth || 1280;
    const width = Math.round(native * zoom / 100);
    video.style.width = Math.min(width, 8000) + "px";
    video.style.height = "auto";
  } else if (video) {
    video.style.width = "";
    video.style.height = "";
  }
}

function renderGrid(filtered) {
  if (!filtered.some(record => record.id === state.activeId)) {
    state.activeId = filtered[0]?.id || null;
  }
  const pageCount = Math.max(1, Math.ceil(filtered.length / state.pageSize));
  state.currentPage = Math.min(Math.max(1, state.currentPage), pageCount);
  const start = (state.currentPage - 1) * state.pageSize;
  const pageRows = filtered.slice(start, start + state.pageSize);

  els.resultCount.textContent = `${filtered.length}`;
  const rangeEnd = Math.min(filtered.length, start + pageRows.length);
  const rangeStart = filtered.length ? start + 1 : 0;
  els.pageStatus.textContent = `${rangeStart}–${rangeEnd} / ${filtered.length} · ${state.currentPage}/${pageCount}`;
  let reviewed = 0, important = 0, pending = 0;
  records.forEach(record => {
    const status = state.marks[record.id]?.status;
    if (status === "reviewed") reviewed += 1;
    else if (status === "important") important += 1;
    else if (status === "needs_verification") pending += 1;
  });
  if (els.triageStatus) {
    els.triageStatus.textContent = tf("triage.line", { done: reviewed, total: records.length, imp: important, pend: pending });
  }
  if (els.dflSummary) {
    const dist = { high: 0, medium: 0, low: 0, unknown: 0, failed: 0 };
    let screened = 0;
    records.forEach(record => {
      const dfl = record.dfl;
      if (!dfl) return;
      if (dfl.error) { dist.failed += 1; screened += 1; return; }
      const band = String(dfl.band || "unknown");
      if (band in dist) dist[band] += 1; else dist.unknown += 1;
      screened += 1;
    });
    els.dflSummary.textContent = screened
      ? tf("dfl.dist", { n: screened, total: records.length, high: dist.high, medium: dist.medium, low: dist.low, unknown: dist.unknown, failed: dist.failed, none: records.length - screened })
      : "";
  }
  els.prevPage.disabled = state.currentPage <= 1;
  els.nextPage.disabled = state.currentPage >= pageCount;

  const groupCounts = new Map();
  if (state.groupBy !== "none") {
    filtered.forEach(record => {
      const key = groupKeyFor(record);
      groupCounts.set(key, (groupCounts.get(key) || 0) + 1);
    });
  }

  const cardsHtml = [];
  let lastGroup = null;
  pageRows.forEach(record => {
    if (state.groupBy !== "none") {
      const key = groupKeyFor(record);
      if (key !== lastGroup) {
        lastGroup = key;
        const collapsed = state.collapsedGroups.has(key);
        const kindAttr = state.groupBy === "kind" ? ` data-gkind="${escapeHtml(record.kind || "")}"` : "";
        cardsHtml.push(`<div class="group-header"${kindAttr} data-group="${escapeHtml(key)}" tabindex="0" role="button"><span>${escapeHtml(key)}</span><span class="muted">${groupCounts.get(key) || 0}${t("unit.count")}${collapsed ? ` · ${t("group.collapsed")}` : ""}</span></div>`);
        if (collapsed) return;
      }
      if (state.collapsedGroups.has(key)) return;
    }
    cardsHtml.push(renderCard(record));
  });
  els.recordGrid.innerHTML = cardsHtml.join("") || `<div class="fallback">${t("grid.empty")}</div>`;
}

// Tri-state audio flag: probed codec present → speaker, probed with no
// audio stream → muted, unprobed (carved/candidate) → nothing so we never
// claim audio state we haven't verified.
function audioFlag(record) {
  const codec = String(record.audioCodec || "").toLowerCase();
  if (codec && codec !== "none" && codec !== "-") {
    return `<span class="audio-flag has" title="${escapeHtml(record.audioCodec)}">🔊</span>`;
  }
  if (record.probeOk === true) {
    return `<span class="audio-flag none" title="${escapeHtml(t("audio.absent"))}">🔇</span>`;
  }
  return "";
}

function renderCard(record) {
  const mark = markOf(record);
  const markChip = mark ? `<span class="mark-chip ${escapeHtml(mark.status)}">${escapeHtml(markLabel(mark.status))}</span>` : "";
  const staleTag = record.indexStatus === "stale" ? '<span class="muted">(stale)</span>' : "";
  const thumb = record.thumb
    ? `<img src="${escapeHtml(record.thumb)}" loading="lazy" decoding="async" fetchpriority="low">`
    : `<div class="ph">${record.status === "validation-failed" ? t("thumb.damaged") : t("thumb.missing")}</div>`;
  const recTypeTag = record.recType ? `<span class="rec-type">${escapeHtml(recTypeLabel(record.recType))}</span>` : "";
  const channel = record.channel ? `<span class="channel-badge">${escapeHtml(record.channel)}</span>` : "";
  const displayName = record.originalName && record.originalName !== record.name
    ? record.originalName
    : record.name;
  const recTime = record.recTime
    ? new Date(record.recTime * 1000).toLocaleString("ko-KR", { year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" })
    : "";
  const tagsHtml = tagListFor(record).length
    ? `<div class="tag-row">${tagListFor(record).map(tag => `<span class="tag-chip">${escapeHtml(tag)}</span>`).join("")}</div>`
    : "";
  const anomalyChip = record.hasAnomaly
    ? `<span class="badge anomaly" title="${escapeHtml(record.anomalies.map(item => item.kind).join(", "))}">${escapeHtml(t("header.anomaly"))}</span>`
    : "";
  const kindChip = `<span class="kind-badge kind-${escapeHtml(record.kind || "video")}" title="${t("tree.kind")}">${escapeHtml(KIND_SHORT[record.kind] || record.kind || "?")}</span>`;
  const warnChip = (record.warnings || []).length
    ? `<span class="badge warn" title="${escapeHtml(record.warnings.join("\n"))}">${tf("warn.count", { n: record.warnings.length })}</span>`
    : "";
  const dflChip = record.dfl && record.dfl.band
    ? `<span class="badge dfl dfl-${escapeHtml(record.dfl.band)}" title="${escapeHtml(tf("dfl.title", { score: record.dfl.score ?? "" }))}">${escapeHtml(tf("dfl.badge", { band: record.dfl.band_label || record.dfl.band }))}</span>`
    : (record.dfl && record.dfl.error
      ? `<span class="badge dfl dfl-low" title="${escapeHtml(tf("dfl.failTitle", { error: record.dfl.error }))}">${escapeHtml(t("dfl.fail"))}</span>`
      : "");
  return `<div class="card ${record.id === state.activeId ? "active" : ""}" data-id="${escapeHtml(record.id)}" tabindex="${record.id === state.activeId ? 0 : -1}" role="option" aria-selected="${state.selectedIds.has(record.id)}">
    <div class="thumb">${thumb}<input type="checkbox" aria-label="${escapeHtml(t("aria.select"))}" ${state.selectedIds.has(record.id) ? "checked" : ""} data-check="${escapeHtml(record.id)}">${recTypeTag}${kindChip}<span class="dur">${fmtDuration(record.duration)}</span></div>
    <div class="meta">
      <div class="name-row"><span class="name" title="${escapeHtml(displayName)}">${highlightEscape(displayName, state.query)}</span>${channel ? `<span class="channel-badge">${escapeHtml(record.channel)}</span>` : ""}</div>
      <div class="time-row"><span class="time-text">${recTime ? escapeHtml(recTime) : t("time.unknown")}</span><span class="badge ${statusClass(record.status)}" title="${escapeHtml(record.status)}">${escapeHtml(statusLabel(record.status))}</span>${anomalyChip}${warnChip}${dflChip}</div>
      ${tagsHtml}
      <div class="sub-row"><span class="sub">${fmtBytes(record.size)}${audioFlag(record)}${markChip}${staleTag}</span></div>
    </div>
  </div>`;
}

function renderHistogram(filtered) {
  const days = new Map();
  filtered.forEach(record => {
    if (record.recDay) days.set(record.recDay, (days.get(record.recDay) || 0) + 1);
  });
  const top = [...days.entries()].sort((a, b) => (a[0] < b[0] ? -1 : 1)).slice(-16);
  const max = Math.max(1, ...top.map(entry => entry[1]));
  els.dayHistogram.innerHTML = top.map(([day, count]) => `
    <button type="button" class="${state.dateFrom === day && state.dateTo === day ? "selected" : ""}" data-day="${escapeHtml(day)}" title="${escapeHtml(tf("hist.item", { day, count }))}" aria-label="${escapeHtml(tf("hist.item", { day, count }))}">
      <span class="bar" style="height:${Math.round((count * 40) / max)}px"></span>
      <span class="lbl">${escapeHtml(day.slice(5))}</span>
    </button>`).join("")
    || `<span class="muted">${t("hist.empty")}</span>`;
  els.dayHistogram.querySelectorAll("button[data-day]").forEach(button => {
    button.addEventListener("click", () => {
      const already = state.dateFrom === button.dataset.day && state.dateTo === button.dataset.day;
      state.dateFrom = already ? "" : button.dataset.day;
      state.dateTo = already ? "" : button.dataset.day;
      els.dateFrom.value = state.dateFrom;
      els.dateTo.value = state.dateTo;
      state.currentPage = 1;
      render();
    });
  });
  const known = records.filter(record => record.recDay).map(record => record.recDay).sort();
  els.periodLabel.textContent = known.length
    ? tf("hist.range", { from: known[0], to: known[known.length - 1], known: known.length, total: records.length })
    : t("hist.none");
}
