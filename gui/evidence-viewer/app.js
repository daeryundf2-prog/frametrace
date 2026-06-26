const records = window.FrameTraceRecords || [];

const translations = window.FrameTraceTranslations || {};

const filters = ["all", "video", "photo", "candidate", "needs_verification", "important", "report"];
const rowHeight = 40;
const defaultSelectedId = defaultSelectedRecordId();

const state = {
  locale: localStorage.getItem("frametrace.locale") || "ko",
  activeFilter: "all",
  selectedId: defaultSelectedId,
  playback: 0,
  playing: false,
  speed: 1,
  zoom: 1,
  activeOverlay: "metadata",
  lastQueryMs: 0,
  visibleWindow: { start: 0, end: 0 },
  renderQueued: false,
  syncView: false,
  packagePreviewQueued: false,
  previewOpen: false,
  previewWindowMode: false,
  dataVersion: 0,
  queryCache: { key: "", rows: [] },
};

const els = {
  workbenchFlow: document.getElementById("workbenchFlow"),
  filterTabs: document.getElementById("filterTabs"),
  searchInput: document.getElementById("searchInput"),
  resultCount: document.getElementById("resultCount"),
  visibleWindow: document.getElementById("visibleWindow"),
  queryLatency: document.getElementById("queryLatency"),
  fileRows: document.getElementById("fileRows"),
  activeKind: document.getElementById("activeKind"),
  activeName: document.getElementById("activeName"),
  selectedEvidenceRail: document.getElementById("selectedEvidenceRail"),
  canvas: document.getElementById("viewerCanvas"),
  timelineHead: document.getElementById("timelineHead"),
  timelineRail: document.getElementById("timelineRail"),
  timelineRange: document.getElementById("timelineRange"),
  timelineEvents: document.getElementById("timelineEvents"),
  timelineScale: document.getElementById("timelineScale"),
  frameTimecode: document.getElementById("frameTimecode"),
  frameBadge: document.getElementById("frameBadge"),
  statusCard: document.getElementById("statusCard"),
  metaList: document.getElementById("metaList"),
  playButton: document.getElementById("playButton"),
  prevButton: document.getElementById("prevButton"),
  nextButton: document.getElementById("nextButton"),
  stepBackButton: document.getElementById("stepBackButton"),
  stepForwardButton: document.getElementById("stepForwardButton"),
  speedSelect: document.getElementById("speedSelect"),
  zoomInput: document.getElementById("zoomInput"),
  languageButton: document.getElementById("languageButton"),
  preview: document.getElementById("evidencePreview"),
  previewCanvas: document.getElementById("previewCanvas"),
  previewTitle: document.getElementById("previewTitle"),
  previewDetails: document.getElementById("previewDetails"),
  previewModeButton: document.getElementById("previewModeButton"),
  previewCloseButton: document.getElementById("previewCloseButton"),
};

let timer = null;

function t(key, replacements = {}) {
  const locale = translations[state.locale] ? state.locale : "ko";
  const text = translations[locale][key] ?? translations.en[key] ?? key;
  return String(text).replace(/\{(\w+)\}/g, (_, name) => replacements[name] ?? "");
}

function valueLabel(value) {
  const locale = translations[state.locale] ? state.locale : "ko";
  return translations[locale].values?.[value] ?? value;
}

function sourceLabel(value) {
  const locale = translations[state.locale] ? state.locale : "ko";
  return translations[locale].sources?.[value] ?? value;
}

function applyLocalization() {
  document.documentElement.lang = state.locale;
  document.title = t("app.title");
  document.querySelectorAll("[data-i18n]").forEach((element) => {
    element.textContent = t(element.dataset.i18n);
  });
  document.querySelectorAll("[data-i18n-placeholder]").forEach((element) => {
    element.setAttribute("placeholder", t(element.dataset.i18nPlaceholder));
  });
  document.querySelectorAll("[data-i18n-title]").forEach((element) => {
    element.setAttribute("title", t(element.dataset.i18nTitle));
  });
  document.querySelectorAll("[data-i18n-aria-label]").forEach((element) => {
    element.setAttribute("aria-label", t(element.dataset.i18nAriaLabel));
  });
  els.languageButton.textContent = state.locale === "ko" ? "EN" : "KO";
}

function selectedRecord() {
  return records.find((record) => record.id === state.selectedId) || records[0];
}

function filteredRecords() {
  const key = inventoryQueryKey();
  if (state.queryCache.key === key) return state.queryCache.rows;
  const query = els.searchInput.value.trim().toLowerCase();
  const rows = records.filter((record) => {
    if (state.activeFilter === "video" && record.type !== "video") return false;
    if (state.activeFilter === "photo" && record.type !== "photo") return false;
    if (state.activeFilter === "candidate" && record.type !== "candidate") return false;
    if (state.activeFilter === "unreviewed" && record.reviewed) return false;
    if (
      state.activeFilter === "needs_verification"
      && record.status !== "needs_verification"
      && record.status !== "candidate"
    ) return false;
    if (state.activeFilter === "important" && record.status !== "important") return false;
    if (state.activeFilter === "report" && !record.report) return false;
    if (!query) return true;
    const haystack = [
      record.id,
      record.name,
      record.path,
      record.vendor,
      record.parser,
      record.hash,
      record.note,
      record.source,
      record.validation,
    ].join(" ").toLowerCase();
    return haystack.includes(query);
  });
  rows.sort(compareInventoryRows);
  state.queryCache = { key, rows };
  return rows;
}

function inventoryQueryKey() {
  return [
    state.dataVersion,
    state.activeFilter,
    els.searchInput.value.trim().toLowerCase(),
  ].join("|");
}

function compareInventoryRows(a, b) {
  return (
    compareNumber(riskRank(b), riskRank(a))
    || compareNumber(timestampValue(a.timestamp), timestampValue(b.timestamp))
    || compareText(a.id, b.id)
  );
}

function defaultSelectedRecordId() {
  const first = [...records].sort(compareDefaultInventoryRows)[0];
  return first ? first.id : records[0].id;
}

function compareDefaultInventoryRows(a, b) {
  return (
    compareNumber(riskRank(b), riskRank(a))
    || compareNumber(timestampValue(a.timestamp), timestampValue(b.timestamp))
    || compareText(a.id, b.id)
  );
}

function riskRank(record) {
  if (record.status === "candidate") return 50;
  if (record.status === "needs_verification") return 40;
  if (record.status === "important") return 30;
  if (!record.reviewed) return 20;
  return 10;
}

function timestampValue(value) {
  if (!value || value === "unknown") return Number.MAX_SAFE_INTEGER;
  const parsed = Date.parse(value.replace(" ", "T"));
  return Number.isFinite(parsed) ? parsed : Number.MAX_SAFE_INTEGER;
}

function compareNumber(a, b) {
  return a === b ? 0 : a < b ? -1 : 1;
}

function compareText(a, b) {
  return String(a).localeCompare(String(b), "en", { numeric: true, sensitivity: "base" });
}

function renderWorkbenchFlow() {
  const sources = [...new Set(records.map((record) => sourceLabel(record.source)))];
  const sourceNames = sources.slice(0, 3).join(" · ");
  const counts = {
    sources: sources.length,
    candidates: records.filter((record) => record.type === "candidate").length,
    validation: records.filter((record) => record.status === "needs_verification" || record.status === "candidate").length,
    report: records.filter((record) => record.report).length,
  };
  const cards = [
    ["summary.sources", t("summary.sourcesDetail", { count: counts.sources, names: sourceNames })],
    ["summary.candidates", t("summary.candidatesDetail", { count: counts.candidates })],
    ["summary.validation", t("summary.validationDetail", { count: counts.validation })],
    ["summary.export", t("summary.exportDetail")],
    ["summary.report", t("summary.reportDetail", { count: counts.report })],
  ];
  els.workbenchFlow.innerHTML = cards.map(([titleKey, detail]) => `
    <div class="workbench-card">
      <strong>${escapeHtml(t(titleKey))}</strong>
      <span>${escapeHtml(detail)}</span>
    </div>
  `).join("");
}

function renderFilters() {
  els.filterTabs.innerHTML = filters.map((key) => (
    `<button type="button" class="${state.activeFilter === key ? "active" : ""}" data-filter="${key}" role="tab" aria-selected="${state.activeFilter === key}">${t(`filter.${key}`)}</button>`
  )).join("");
  els.filterTabs.querySelectorAll("button").forEach((button) => {
    button.addEventListener("click", () => {
      state.activeFilter = button.dataset.filter;
      els.fileRows.scrollTop = 0;
      ensureSelectionVisible();
      renderAll();
    });
  });
}

function renderFiles() {
  const startedAt = performance.now();
  const visible = filteredRecords();
  state.lastQueryMs = Math.max(0, Math.round(performance.now() - startedAt));
  els.fileRows.style.setProperty("--row-height", `${rowHeight}px`);
  els.fileRows.className = "file-rows";
  if (!visible.length) {
    els.fileRows.innerHTML = `<div class="empty-state">${t("empty.noMatches")}</div>`;
    state.visibleWindow = { start: 0, end: 0 };
    renderInventoryMetrics(0);
    return;
  }
  const viewportHeight = els.fileRows.clientHeight || 520;
  const scrollTop = els.fileRows.scrollTop;
  const overscan = 8;
  const visibleCount = Math.ceil(viewportHeight / rowHeight);
  const startIndex = Math.max(0, Math.floor(scrollTop / rowHeight) - overscan);
  const endIndex = Math.min(visible.length, startIndex + visibleCount + overscan * 2);
  state.visibleWindow = { start: startIndex + 1, end: endIndex };
  const pageRows = visible.slice(startIndex, endIndex);
  els.fileRows.innerHTML = `
    <div class="virtual-spacer" style="height:${visible.length * rowHeight}px">
      <div class="virtual-window" style="transform:translateY(${startIndex * rowHeight}px)">
        ${pageRows.map(fileRowHtml).join("")}
      </div>
    </div>`;
  els.fileRows.querySelectorAll(".data-row").forEach((row) => {
    row.addEventListener("click", () => selectRecord(row.dataset.id));
    row.addEventListener("keydown", activateOnKeyboard(() => selectRecord(row.dataset.id)));
  });
  els.fileRows.querySelectorAll("canvas[data-thumb]").forEach((canvas) => {
    const record = records.find((item) => item.id === canvas.dataset.thumb);
    if (record) drawScene(canvas, record, 0, 1, true);
  });
  renderInventoryMetrics(visible.length);
}

function fileRowHtml(record) {
  return `
    <div class="file-row data-row ${record.id === state.selectedId ? "active" : ""}" role="row" tabindex="0" aria-selected="${record.id === state.selectedId}" data-id="${record.id}">
      <span class="status-pill status-${record.status}">${rowStatusLabel(record.status)}</span>
      <span class="review-cell">${record.reviewed ? t("status.reviewed") : t("status.unreviewed")}</span>
      <code class="id-cell">${escapeHtml(record.id)}</code>
      <span class="file-name"><strong>${escapeHtml(record.name)}</strong><span>${escapeHtml(middleTruncate(record.path, 72))}</span></span>
      <span class="source-cell">${escapeHtml(sourceLabel(record.source))}</span>
      <span class="time-cell">${escapeHtml(shortTimestamp(record.timestamp))}</span>
      <span class="type-cell">${typeLabel(record.type)} / ${escapeHtml(record.parser)}</span>
      <span class="validation-cell">${escapeHtml(valueLabel(record.validation))}</span>
      <span class="size-cell">${escapeHtml(record.size)}</span>
      <span class="hash-cell">${escapeHtml(valueLabel(record.hashStatus))}</span>
      <span class="report-cell">${record.report ? "IN" : "-"}</span>
      <canvas class="thumb" width="132" height="84" data-thumb="${record.id}" aria-label="${escapeAttr(t("table.previewFor", { name: record.name }))}"></canvas>
    </div>`;
}

function renderInventoryMetrics(total) {
  els.resultCount.textContent = t("inventory.results", { count: total });
  els.visibleWindow.textContent = t("inventory.window", state.visibleWindow);
  els.queryLatency.textContent = t("inventory.latency", { ms: state.lastQueryMs });
}

function renderViewer() {
  const record = selectedRecord();
  els.activeKind.textContent = typeLabel(record.type);
  els.activeName.textContent = record.name;
  els.frameBadge.textContent = frameBadge(record);
  if (record.type !== "video" && record.type !== "candidate") {
    state.playing = false;
    stopTimer();
  }
  drawScene(els.canvas, record, state.playback, state.zoom, false);
  const duration = Math.max(record.duration || 1, 1);
  const pct = Math.min(100, Math.max(0, (state.playback / duration) * 100));
  els.timelineHead.style.left = `${pct}%`;
  els.timelineRange.style.left = record.report ? "22%" : "8%";
  els.timelineRange.style.width = record.report ? "28%" : "16%";
  els.timelineEvents.innerHTML = (record.events || []).map((event) => (
    `<span class="event-mark" style="left:${Math.min(98, Math.max(0, (event / duration) * 100))}%"></span>`
  )).join("");
  els.timelineScale.innerHTML = [0, duration / 4, duration / 2, (duration * 3) / 4, duration]
    .map((tick) => `<span>${formatDuration(tick)}</span>`)
    .join("");
  els.frameTimecode.textContent = formatTimecode(state.playback);
  els.playButton.innerHTML = state.playing ? "&#10074;&#10074;" : "&#9654;";
  document.querySelectorAll("[data-overlay]").forEach((button) => {
    button.classList.toggle("active", button.dataset.overlay === state.activeOverlay);
  });
  renderSelectedEvidenceRail(record);
}

function renderSelectedEvidenceRail(record) {
  const validationQueued = record.validationQueued || record.outputStateKey === "output.validationQueued";
  const outputQueued = Boolean(record.outputStateKey) && record.outputStateKey !== "output.validationQueued";
  const validationBadge = validationQueued ? t("rail.queued") : t("rail.required");
  const reportBadge = record.report ? t("rail.included") : t("rail.notIncluded");
  els.selectedEvidenceRail.innerHTML = `
    <div class="rail-summary">
      <span class="rail-kicker">${escapeHtml(t("rail.selectedEvidence"))}</span>
      <strong>${escapeHtml(sourceLabel(record.source))}</strong>
      <span class="status-pill status-${record.status}">${escapeHtml(statusLabel(record.status))}</span>
      <code>${escapeHtml(record.id)} · ${escapeHtml(valueLabel(record.readMode || t("meta.notRecorded")))}</code>
      <span>${escapeHtml(t("rail.readOnly"))} · ${escapeHtml(valueLabel(record.validation))}</span>
    </div>
    <div class="rail-actions">
      ${railActionHtml("queue-validation", "rail.validation", "rail.validationDetail", validationBadge, !validationQueued)}
      ${railActionHtml("export-mp4", "rail.export", "rail.exportDetail", outputQueued ? t("rail.queued") : t("rail.preview"), outputQueued)}
      ${railActionHtml("report-set", "rail.report", "rail.reportDetail", reportBadge, record.report)}
      ${railActionHtml("package-case", "rail.package", "rail.packageDetail", state.packagePreviewQueued ? t("rail.queued") : t("rail.preview"), state.packagePreviewQueued)}
      ${railActionHtml("preview-open", "rail.previewOpen", "rail.previewDetail", t("rail.preview"), true)}
      ${railActionHtml("preview-window", "rail.windowMode", "rail.windowDetail", t("rail.preview"), true)}
    </div>
  `;
}

function railActionHtml(action, titleKey, detailKey, badge, emphasized) {
  const stateClass = emphasized ? "is-queued" : "is-required";
  return `
    <button class="rail-action ${stateClass}" type="button" data-rail-action="${action}">
      <span class="rail-action-copy">
        <strong>${escapeHtml(t(titleKey))}</strong>
        <small>${escapeHtml(t(detailKey))}</small>
      </span>
      <em>${escapeHtml(badge)}</em>
    </button>`;
}

function renderInspector() {
  const record = selectedRecord();
  els.statusCard.innerHTML = `
    <span class="status-pill status-${record.status}">${statusLabel(record.status)}</span>
    <strong>${escapeHtml(valueLabel(record.validation))}</strong>
    <span class="state-note">${escapeHtml(valueLabel(record.note))}</span>
    <div class="decision-gate" aria-label="${escapeAttr(t("gate.title"))}">
      ${decisionGateItems(record).map((item) => `
        <div class="decision-gate-row gate-${item.state}">
          <span>${escapeHtml(t(item.labelKey))}</span>
          <strong>${escapeHtml(t(item.stateKey))}</strong>
        </div>
      `).join("")}
    </div>
  `;
  const fields = [
    [t("meta.id"), `<code>${escapeHtml(record.id)}</code>`],
    [t("meta.source"), escapeHtml(sourceLabel(record.source))],
    [t("meta.path"), `<code>${escapeHtml(record.path)}</code>`],
    [t("meta.vendor"), escapeHtml(record.vendor)],
    [t("meta.parser"), `<code>${escapeHtml(record.parser)}</code>`],
    [t("meta.hash"), `<code>${escapeHtml(valueLabel(record.fullHash || record.hash))}</code>`],
    [t("meta.hashState"), escapeHtml(valueLabel(record.hashStatus))],
    [t("meta.acquired"), escapeHtml(record.acquiredAt || t("meta.notRecorded"))],
    [t("meta.readMode"), escapeHtml(valueLabel(record.readMode || t("meta.notRecorded")))],
    [t("meta.offset"), escapeHtml(valueLabel(record.offset || t("meta.notRecorded")))],
    [t("meta.codec"), escapeHtml(record.codec || t("meta.notRecorded"))],
    [t("meta.output"), escapeHtml(formatOutputState(record))],
    [t("meta.channel"), escapeHtml(valueLabel(record.channel))],
    [t("meta.duration"), record.type === "photo" ? t("meta.still") : formatDuration(record.duration)],
  ];
  els.metaList.innerHTML = fields.map(([key, value]) => `<dt>${key}</dt><dd>${value}</dd>`).join("");
}

function decisionGateItems(record) {
  const validationComplete = record.status !== "candidate" && record.status !== "needs_verification";
  const playbackConfirmed = String(record.validation || "").includes("confirmed") || record.reviewed;
  const exportQueued = Boolean(record.outputStateKey) && record.outputStateKey !== "output.validationQueued";
  return [
    { labelKey: "gate.indexed", state: "complete", stateKey: "gate.complete" },
    { labelKey: "gate.container", state: validationComplete ? "complete" : "pending", stateKey: validationComplete ? "gate.complete" : "gate.pending" },
    { labelKey: "gate.playback", state: playbackConfirmed ? "complete" : "pending", stateKey: playbackConfirmed ? "gate.complete" : "gate.pending" },
    { labelKey: "gate.report", state: record.report ? "complete" : "pending", stateKey: record.report ? "gate.complete" : "gate.pending" },
    { labelKey: "gate.export", state: exportQueued ? "queued" : "pending", stateKey: exportQueued ? "gate.queued" : "gate.pending" },
  ];
}

function renderAll() {
  applyLocalization();
  renderWorkbenchFlow();
  renderFilters();
  renderFiles();
  renderViewer();
  renderInspector();
  renderPreview();
}

function rowStatusLabel(status) {
  return t(`status.row.${status}`);
}

function selectRecord(id) {
  state.selectedId = id;
  state.playback = 0;
  stopTimer();
  renderAll();
}

function ensureSelectionVisible() {
  const visible = filteredRecords();
  if (!visible.some((record) => record.id === state.selectedId) && visible[0]) {
    state.selectedId = visible[0].id;
    state.playback = 0;
  }
}

function requestInventoryRender() {
  if (state.renderQueued) return;
  state.renderQueued = true;
  window.requestAnimationFrame(() => {
    state.renderQueued = false;
    renderFiles();
  });
}

function updateRecord(mutator) {
  const record = selectedRecord();
  mutator(record);
  state.dataVersion += 1;
  ensureSelectionVisible();
  renderAll();
}

function markRecordReviewed() {
  updateRecord((record) => {
    record.reviewed = true;
    if (record.status === "needs_verification") record.status = "reviewed";
  });
}

function markRecordImportant() {
  updateRecord((record) => {
    record.status = "important";
  });
}

function toggleRecordReportSet() {
  updateRecord((record) => {
    record.report = !record.report;
  });
}

function queueMp4Export() {
  updateRecord((record) => {
    record.outputStateKey = "output.mp4Queued";
    record.outputStateTime = formatTimecode(state.playback);
  });
}

function queueAviExport() {
  updateRecord((record) => {
    record.outputStateKey = "output.aviQueued";
    record.outputStateTime = formatTimecode(state.playback);
  });
}

function queueFrameCapture() {
  updateRecord((record) => {
    record.outputStateKey = "output.frameQueued";
    record.outputStateTime = formatTimecode(state.playback);
  });
}

function queueValidation() {
  updateRecord((record) => {
    if (record.status === "candidate" || record.status === "needs_verification") {
      record.validationQueued = true;
      if (!record.outputStateKey) {
        record.outputStateKey = "output.validationQueued";
        record.outputStateTime = null;
      }
    }
  });
}

function queuePackagePreview() {
  state.packagePreviewQueued = true;
  renderAll();
}

function openPreview(windowMode = false) {
  state.previewOpen = true;
  state.previewWindowMode = windowMode;
  renderPreview();
  window.requestAnimationFrame(() => els.previewCloseButton.focus());
}

function closePreview() {
  state.previewOpen = false;
  renderPreview();
}

function renderPreview() {
  const record = selectedRecord();
  els.preview.hidden = !state.previewOpen;
  els.preview.classList.toggle("is-window-mode", state.previewWindowMode);
  if (!state.previewOpen) return;
  els.previewTitle.textContent = state.previewWindowMode ? t("preview.windowTitle") : t("preview.title");
  els.previewModeButton.textContent = state.previewWindowMode ? t("preview.panelMode") : t("preview.windowMode");
  drawScene(els.previewCanvas, record, state.playback, 1, false);
  const rows = [
    [t("preview.status"), `${statusLabel(record.status)} · ${valueLabel(record.validation)}`],
    [t("preview.source"), `${sourceLabel(record.source)} · ${middleTruncate(record.path, 72)}`],
    [t("meta.codec"), record.codec || t("meta.notRecorded")],
    [t("meta.duration"), record.type === "photo" ? t("meta.still") : formatDuration(record.duration)],
    [t("preview.hash"), valueLabel(record.hashStatus)],
    [t("preview.output"), formatOutputState(record)],
  ];
  els.previewDetails.innerHTML = `
    <span class="status-pill status-${record.status}">${escapeHtml(statusLabel(record.status))}</span>
    <strong>${escapeHtml(record.name)}</strong>
    <p>${escapeHtml(valueLabel(record.note))}</p>
    <dl>${rows.map(([key, value]) => `<dt>${escapeHtml(key)}</dt><dd>${escapeHtml(value)}</dd>`).join("")}</dl>
  `;
}

function formatOutputState(record) {
  if (!record.outputStateKey) return t("output.noneQueued");
  const label = t(record.outputStateKey);
  return record.outputStateTime ? `${label} @ ${record.outputStateTime}` : label;
}

function togglePlay() {
  const record = selectedRecord();
  if (record.type === "photo") return;
  state.playing = !state.playing;
  if (state.playing) {
    startTimer();
  } else {
    stopTimer(false);
  }
  renderViewer();
}

function startTimer() {
  stopTimer(false);
  timer = window.setInterval(() => {
    const record = selectedRecord();
    const duration = Math.max(record.duration || 1, 1);
    state.playback += 0.25 * state.speed;
    if (state.playback >= duration) state.playback = 0;
    renderViewer();
  }, 250);
}

function stopTimer(resetPlaying = true) {
  if (timer) window.clearInterval(timer);
  timer = null;
  if (resetPlaying) state.playing = false;
}

function step(seconds) {
  const record = selectedRecord();
  const duration = Math.max(record.duration || 1, 1);
  state.playback = Math.min(duration, Math.max(0, state.playback + seconds));
  renderViewer();
}

function moveSelection(delta) {
  const visible = filteredRecords();
  const index = visible.findIndex((record) => record.id === state.selectedId);
  const next = visible[Math.min(visible.length - 1, Math.max(0, index + delta))];
  if (next) selectRecord(next.id);
}

function drawScene(canvas, record, time, zoom, thumb) {
  const ctx = canvas.getContext("2d");
  const width = canvas.width;
  const height = canvas.height;
  ctx.clearRect(0, 0, width, height);
  const paired = !thumb && state.syncView ? pairedRecord(record) : null;
  if (paired) {
    drawSyncedScene(ctx, width, height, record, paired, time);
    drawCameraOverlay(ctx, width, height, record, time);
    return;
  }
  ctx.save();
  if (!thumb && zoom > 1) {
    ctx.translate(width / 2, height / 2);
    ctx.scale(zoom, zoom);
    ctx.translate(-width / 2, -height / 2);
  }
  drawSceneContent(ctx, width, height, record, time);
  ctx.restore();
  if (!thumb) drawCameraOverlay(ctx, width, height, record, time);
  if (!thumb && state.activeOverlay === "levels") drawLevelOverlay(ctx, width, height);
  if (!thumb && state.activeOverlay === "compare") drawCompareOverlay(ctx, width, height);
}

function drawSceneContent(ctx, width, height, record, time) {
  if (record.scene === "road") drawRoad(ctx, width, height, time);
  else if (record.scene === "rear") drawRear(ctx, width, height, time);
  else if (record.scene === "parking") drawParking(ctx, width, height, time);
  else if (record.scene === "still") drawStill(ctx, width, height);
  else if (record.scene === "plate") drawPlate(ctx, width, height);
  else drawDamaged(ctx, width, height, time);
}

function drawSyncedScene(ctx, width, height, primary, secondary, time) {
  ctx.fillStyle = "#101815";
  ctx.fillRect(0, 0, width, height);
  const pad = width * 0.025;
  const gap = width * 0.014;
  const paneWidth = (width - pad * 2 - gap) / 2;
  const paneHeight = paneWidth * 9 / 16;
  const top = (height - paneHeight) / 2;
  drawSyncedPane(ctx, pad, top, paneWidth, paneHeight, primary, time, t("syncPane.a"));
  drawSyncedPane(ctx, pad + paneWidth + gap, top, paneWidth, paneHeight, secondary, time, t("syncPane.b"));
}

function drawSyncedPane(ctx, x, y, width, height, record, time, label) {
  ctx.save();
  ctx.beginPath();
  ctx.rect(x, y, width, height);
  ctx.clip();
  ctx.translate(x, y);
  drawSceneContent(ctx, width, height, record, time);
  ctx.restore();
  ctx.strokeStyle = "rgba(238, 247, 244, 0.42)";
  ctx.lineWidth = 2;
  ctx.strokeRect(x, y, width, height);
  ctx.fillStyle = "rgba(14, 22, 19, 0.72)";
  ctx.fillRect(x + 12, y + 12, width * 0.42, 32);
  ctx.fillStyle = "#eaf3ef";
  ctx.font = `20px ${getComputedStyle(document.body).fontFamily}`;
  ctx.fillText(`${label} ${valueLabel(record.channel)}`, x + 22, y + 35);
}

function pairedRecord(record) {
  if (record.type !== "video") return null;
  if (!record.timestamp || record.timestamp === "unknown") return null;
  return records.find((item) => (
    item.id !== record.id
    && item.type === "video"
    && item.timestamp === record.timestamp
    && item.vendor === record.vendor
  )) || null;
}

function drawRoad(ctx, width, height, time) {
  const sky = ctx.createLinearGradient(0, 0, 0, height * 0.55);
  sky.addColorStop(0, "#9db8c8");
  sky.addColorStop(1, "#d4ddd8");
  ctx.fillStyle = sky;
  ctx.fillRect(0, 0, width, height);
  ctx.fillStyle = "#657468";
  ctx.fillRect(0, height * 0.54, width, height * 0.46);
  ctx.fillStyle = "#2f3635";
  ctx.beginPath();
  ctx.moveTo(width * 0.1, height);
  ctx.lineTo(width * 0.45, height * 0.54);
  ctx.lineTo(width * 0.56, height * 0.54);
  ctx.lineTo(width * 0.91, height);
  ctx.closePath();
  ctx.fill();
  ctx.strokeStyle = "#f0e8a8";
  ctx.lineWidth = width * 0.008;
  for (let i = 0; i < 8; i += 1) {
    const y = height * (0.58 + i * 0.07 + (time % 1) * 0.02);
    ctx.beginPath();
    ctx.moveTo(width * 0.5, y);
    ctx.lineTo(width * 0.5, y + height * 0.035);
    ctx.stroke();
  }
  drawCar(ctx, width * 0.63, height * 0.63, width * 0.12, "#932f2d");
  drawCar(ctx, width * 0.34, height * 0.67, width * 0.16, "#d7d9d4");
}

function drawRear(ctx, width, height, time) {
  ctx.fillStyle = "#d8ded9";
  ctx.fillRect(0, 0, width, height * 0.48);
  ctx.fillStyle = "#465452";
  ctx.fillRect(0, height * 0.48, width, height * 0.52);
  for (let i = 0; i < 9; i += 1) {
    ctx.fillStyle = i % 2 ? "#bfc7c2" : "#aeb9b3";
    ctx.fillRect(i * width * 0.13 - (time % 2) * 20, height * 0.49, width * 0.08, height * 0.51);
  }
  drawCar(ctx, width * 0.44, height * 0.6, width * 0.18, "#314f69");
}

function drawParking(ctx, width, height, time) {
  ctx.fillStyle = "#c9d1cc";
  ctx.fillRect(0, 0, width, height);
  ctx.fillStyle = "#6e7a72";
  ctx.fillRect(0, height * 0.4, width, height * 0.6);
  ctx.strokeStyle = "#e6e1be";
  ctx.lineWidth = width * 0.005;
  for (let i = 0; i < 7; i += 1) {
    ctx.beginPath();
    ctx.moveTo(width * (0.1 + i * 0.14), height * 0.43);
    ctx.lineTo(width * (0.03 + i * 0.14), height);
    ctx.stroke();
  }
  drawCar(ctx, width * 0.2, height * 0.54, width * 0.14, "#e6e6df");
  drawCar(ctx, width * (0.62 + Math.sin(time) * 0.03), height * 0.58, width * 0.15, "#7b2f35");
  drawPerson(ctx, width * 0.73, height * 0.52, width * 0.04);
}

function drawStill(ctx, width, height) {
  drawRoad(ctx, width, height, 27.4);
  ctx.strokeStyle = "#d9b136";
  ctx.lineWidth = width * 0.012;
  ctx.strokeRect(width * 0.58, height * 0.52, width * 0.19, height * 0.17);
}

function drawPlate(ctx, width, height) {
  drawParking(ctx, width, height, 48);
  ctx.fillStyle = "rgba(238, 242, 239, 0.88)";
  ctx.fillRect(width * 0.41, height * 0.5, width * 0.28, height * 0.1);
  ctx.fillStyle = "#1f2724";
  ctx.font = `${Math.floor(width * 0.04)}px ${getComputedStyle(document.body).fontFamily}`;
  ctx.fillText("42A 7193", width * 0.435, height * 0.565);
  ctx.strokeStyle = "#b14d42";
  ctx.lineWidth = width * 0.01;
  ctx.strokeRect(width * 0.39, height * 0.47, width * 0.32, height * 0.16);
}

function drawDamaged(ctx, width, height, time) {
  ctx.fillStyle = "#1b211f";
  ctx.fillRect(0, 0, width, height);
  drawRoad(ctx, width, height, time);
  ctx.fillStyle = "rgba(27, 33, 31, 0.62)";
  for (let i = 0; i < 18; i += 1) {
    ctx.fillRect((i * 91) % width, (i * 53) % height, width * 0.08, height * 0.03);
  }
  ctx.strokeStyle = "#9f6c36";
  ctx.lineWidth = width * 0.006;
  ctx.beginPath();
  ctx.moveTo(width * 0.22, height * 0.23);
  ctx.lineTo(width * 0.41, height * 0.37);
  ctx.lineTo(width * 0.36, height * 0.62);
  ctx.lineTo(width * 0.66, height * 0.72);
  ctx.stroke();
}

function drawCar(ctx, x, y, size, color) {
  ctx.fillStyle = color;
  ctx.fillRect(x, y, size, size * 0.36);
  ctx.fillStyle = shade(color, -28);
  ctx.fillRect(x + size * 0.18, y - size * 0.18, size * 0.54, size * 0.22);
  ctx.fillStyle = "#111";
  ctx.beginPath();
  ctx.arc(x + size * 0.2, y + size * 0.36, size * 0.08, 0, Math.PI * 2);
  ctx.arc(x + size * 0.8, y + size * 0.36, size * 0.08, 0, Math.PI * 2);
  ctx.fill();
}

function drawPerson(ctx, x, y, size) {
  ctx.fillStyle = "#26312d";
  ctx.beginPath();
  ctx.arc(x, y, size * 0.28, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillRect(x - size * 0.18, y + size * 0.25, size * 0.36, size * 0.82);
}

function drawCameraOverlay(ctx, width, height, record, time) {
  const timestampLabel = record.timestamp === "unknown" ? valueLabel("unknown") : record.timestamp;
  ctx.fillStyle = "rgba(0, 0, 0, 0.56)";
  ctx.fillRect(width * 0.025, height * 0.035, width * 0.31, height * 0.08);
  ctx.fillStyle = "#eaf3ef";
  ctx.font = `${Math.floor(width * 0.022)}px ${getComputedStyle(document.body).fontFamily}`;
  ctx.fillText(timestampLabel, width * 0.04, height * 0.073);
  ctx.font = `${Math.floor(width * 0.017)}px ${getComputedStyle(document.body).fontFamily}`;
  ctx.fillText(`${valueLabel(record.channel)}  ${formatTimecode(time)}`, width * 0.04, height * 0.103);
}

function drawLevelOverlay(ctx, width, height) {
  const left = width * 0.72;
  const top = height * 0.08;
  const barWidth = width * 0.18;
  ctx.fillStyle = "rgba(14, 22, 19, 0.68)";
  ctx.fillRect(left, top, barWidth, height * 0.16);
  for (let i = 0; i < 18; i += 1) {
    const value = Math.sin(i * 0.7) * 0.35 + 0.55;
    ctx.fillStyle = i % 3 === 0 ? "#d9b136" : "#8ed4cb";
    ctx.fillRect(left + 12 + i * (barWidth - 24) / 18, top + height * (0.13 - value * 0.1), 5, height * value * 0.1);
  }
}

function drawCompareOverlay(ctx, width, height) {
  ctx.fillStyle = "rgba(255, 255, 255, 0.16)";
  ctx.fillRect(width * 0.5, 0, width * 0.5, height);
  ctx.strokeStyle = "#f7efe4";
  ctx.lineWidth = 3;
  ctx.beginPath();
  ctx.moveTo(width * 0.5, 0);
  ctx.lineTo(width * 0.5, height);
  ctx.stroke();
  ctx.fillStyle = "rgba(14, 22, 19, 0.72)";
  ctx.fillRect(width * 0.51, height * 0.06, width * 0.19, height * 0.05);
  ctx.fillStyle = "#eaf3ef";
  ctx.font = `${Math.floor(width * 0.018)}px ${getComputedStyle(document.body).fontFamily}`;
  ctx.fillText(t("canvas.derivedView"), width * 0.525, height * 0.094);
}

function frameBadge(record) {
  if (record.sourceKind === "derived") return t("badge.derived");
  if (record.type === "candidate") return t("badge.candidate");
  return t("badge.original");
}

function statusLabel(value) {
  return t(`status.${value}`);
}

function typeLabel(value) {
  return t(`type.${value}`);
}

function shortTimestamp(value) {
  if (!value || value === "unknown") return valueLabel("unknown");
  const parts = value.split(" ");
  if (parts.length < 2) return value;
  return `${parts[0].slice(5)} ${parts[1].slice(0, 5)}`;
}

function formatDuration(seconds) {
  const value = Math.max(0, Math.floor(seconds));
  const mm = String(Math.floor(value / 60)).padStart(2, "0");
  const ss = String(value % 60).padStart(2, "0");
  return `${mm}:${ss}`;
}

function formatTimecode(seconds) {
  const ms = Math.floor((seconds % 1) * 1000);
  const total = Math.floor(seconds);
  const hh = String(Math.floor(total / 3600)).padStart(2, "0");
  const mm = String(Math.floor((total % 3600) / 60)).padStart(2, "0");
  const ss = String(total % 60).padStart(2, "0");
  return `${hh}:${mm}:${ss}.${String(ms).padStart(3, "0")}`;
}

function shade(hex, amount) {
  const number = Number.parseInt(hex.slice(1), 16);
  const r = Math.max(0, Math.min(255, (number >> 16) + amount));
  const g = Math.max(0, Math.min(255, ((number >> 8) & 0xff) + amount));
  const b = Math.max(0, Math.min(255, (number & 0xff) + amount));
  return `rgb(${r},${g},${b})`;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function escapeAttr(value) {
  return escapeHtml(value).replaceAll("'", "&#39;");
}

function middleTruncate(value, maxLength) {
  const text = String(value);
  if (text.length <= maxLength) return text;
  const keep = Math.max(8, Math.floor((maxLength - 3) / 2));
  return `${text.slice(0, keep)}...${text.slice(-keep)}`;
}

function activateOnKeyboard(callback) {
  return (event) => {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    callback(event);
  };
}

els.searchInput.addEventListener("input", () => {
  els.fileRows.scrollTop = 0;
  ensureSelectionVisible();
  renderAll();
});
els.fileRows.addEventListener("scroll", requestInventoryRender);
els.playButton.addEventListener("click", togglePlay);
els.prevButton.addEventListener("click", () => moveSelection(-1));
els.nextButton.addEventListener("click", () => moveSelection(1));
els.stepBackButton.addEventListener("click", () => step(-1 / 30));
els.stepForwardButton.addEventListener("click", () => step(1 / 30));
els.speedSelect.addEventListener("change", () => {
  state.speed = Number(els.speedSelect.value);
});
els.zoomInput.addEventListener("input", () => {
  state.zoom = Number(els.zoomInput.value);
  renderViewer();
});
els.timelineRail.addEventListener("click", (event) => {
  const record = selectedRecord();
  const bounds = els.timelineRail.getBoundingClientRect();
  const ratio = Math.min(1, Math.max(0, (event.clientX - bounds.left) / bounds.width));
  state.playback = ratio * Math.max(record.duration || 1, 1);
  renderViewer();
});
document.querySelectorAll("[data-overlay]").forEach((button) => {
  button.addEventListener("click", () => {
    state.activeOverlay = button.dataset.overlay;
    renderViewer();
  });
});
els.selectedEvidenceRail.addEventListener("click", (event) => {
  if (!(event.target instanceof Element)) return;
  const button = event.target.closest("[data-rail-action]");
  if (!button) return;
  if (button.dataset.railAction === "queue-validation") queueValidation();
  if (button.dataset.railAction === "export-mp4") queueMp4Export();
  if (button.dataset.railAction === "report-set") toggleRecordReportSet();
  if (button.dataset.railAction === "package-case") queuePackagePreview();
  if (button.dataset.railAction === "preview-open") openPreview(false);
  if (button.dataset.railAction === "preview-window") openPreview(true);
});
els.previewModeButton.addEventListener("click", () => {
  state.previewWindowMode = !state.previewWindowMode;
  renderPreview();
});
els.previewCloseButton.addEventListener("click", closePreview);
els.preview.addEventListener("click", (event) => {
  if (!(event.target instanceof Element)) return;
  const command = event.target.closest("[data-preview-command]")?.dataset.previewCommand;
  if (command === "report") toggleRecordReportSet();
  if (command === "export") queueMp4Export();
  if (command === "verify") queueValidation();
});
document.getElementById("markReviewedButton").addEventListener("click", markRecordReviewed);
document.getElementById("markImportantButton").addEventListener("click", markRecordImportant);
document.getElementById("addReportButton").addEventListener("click", toggleRecordReportSet);
document.getElementById("exportMp4Button").addEventListener("click", queueMp4Export);
document.getElementById("exportAviButton").addEventListener("click", queueAviExport);
document.getElementById("captureFrameButton").addEventListener("click", queueFrameCapture);
document.getElementById("verifyButton").addEventListener("click", queueValidation);
document.getElementById("syncViewButton").addEventListener("click", (event) => {
  state.syncView = !state.syncView;
  event.currentTarget.classList.toggle("active", state.syncView);
  renderAll();
});
els.languageButton.addEventListener("click", () => {
  state.locale = state.locale === "ko" ? "en" : "ko";
  localStorage.setItem("frametrace.locale", state.locale);
  renderAll();
});
document.getElementById("packageButton").addEventListener("click", queuePackagePreview);
document.addEventListener("keydown", (event) => {
  if (event.target && ["INPUT", "SELECT"].includes(event.target.tagName)) return;
  if (event.key === " ") {
    event.preventDefault();
    togglePlay();
  } else if (event.key === "ArrowRight") {
    step(1 / 30);
  } else if (event.key === "ArrowLeft") {
    step(-1 / 30);
  } else if (event.key.toLowerCase() === "n") {
    moveSelection(1);
  } else if (event.key.toLowerCase() === "p") {
    moveSelection(-1);
  } else if (event.key === "Escape" && state.previewOpen) {
    closePreview();
  }
});

renderAll();
