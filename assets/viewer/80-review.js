function moveActive(step) {
  const filtered = filteredRecords();
  if (!filtered.length) return;
  const index = filtered.findIndex(record => record.id === state.activeId);
  const next = Math.max(0, Math.min(filtered.length - 1, (index < 0 ? 0 : index + step)));
  const nextId = filtered[next].id;
  const nextPage = Math.floor(next / state.pageSize) + 1;
  const pageChanged = nextPage !== state.currentPage;
  state.activeId = nextId;
  if (pageChanged) {
    state.currentPage = nextPage;
    render();
    const card = els.recordGrid.querySelector(`.card[data-id="${CSS.escape(state.activeId)}"]`);
    card?.scrollIntoView({ block: "nearest" });
    return;
  }
  // Same page: avoid rebuilding up to 1000 cards on every j/k.
  els.recordGrid.querySelectorAll(".card.active").forEach(el => { el.classList.remove("active"); el.tabIndex = -1; });
  const card = els.recordGrid.querySelector(`.card[data-id="${CSS.escape(state.activeId)}"]`);
  if (card) { card.classList.add("active"); card.tabIndex = 0; }
  card?.scrollIntoView({ block: "nearest" });
  renderDetails();
}

function toggleActiveSelection() {
  if (!state.activeId) return;
  if (state.selectedIds.has(state.activeId)) state.selectedIds.delete(state.activeId);
  else state.selectedIds.add(state.activeId);
  render();
}

let shortcutsLastFocus = null;
function toggleShortcuts(open) {
  const modal = document.getElementById("shortcutsModal");
  if (open && modal.hidden) shortcutsLastFocus = document.activeElement;
  modal.hidden = !open;
  if (open) document.getElementById("btnShortcutsClose")?.focus();
  else if (shortcutsLastFocus) { shortcutsLastFocus.focus(); shortcutsLastFocus = null; }
}
document.getElementById("shortcutsModal").addEventListener("keydown", e => {
  if (e.key !== "Tab") return;
  const focusables = [...e.currentTarget.querySelectorAll("button, input, select, a[href], [tabindex]")]
    .filter(el => !el.disabled);
  if (!focusables.length) return;
  const first = focusables[0];
  const last = focusables[focusables.length - 1];
  if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); }
  else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
});

// Single-key triage: mark the active record and advance so an examiner
// can clear a review queue without touching the mouse.
function markActive(status) {
  const before = filteredRecords();
  const index = before.findIndex(record => record.id === state.activeId);
  if (index < 0) return;
  if (status === null) { delete state.marks[state.activeId]; state.notes[state.activeId] = ""; }
  else state.marks[state.activeId] = { status, marked_unix: Math.floor(Date.now() / 1000) };
  touchAnnotation(state.activeId);
  storageSet(MARKS_KEY, state.marks);
  const after = filteredRecords();
  const next = before.slice(index + 1).find(record => after.some(item => item.id === record.id));
  state.activeId = next?.id || after[Math.min(index, after.length - 1)]?.id || null;
  state.currentPage = Math.max(1, Math.floor(after.findIndex(record => record.id === state.activeId) / state.pageSize) + 1);
  render();
}

document.addEventListener("keydown", event => {
  const tag = document.activeElement?.tagName;
  if (tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA") {
    if (event.key === "Escape") document.activeElement.blur();
    return;
  }
  if (tag === "BUTTON" && (event.key === " " || event.key === "Enter")) {
    return; // let the focused button receive its normal activation keys
  }
  if (event.ctrlKey && (event.key === "a" || event.key === "A")) { event.preventDefault(); selectAllFiltered(); return; }
  if (event.ctrlKey && (event.key === "d" || event.key === "D")) { event.preventDefault(); clearSelection(); return; }
  switch (event.key) {
    case "j": moveActive(1); break;
    case "k": moveActive(-1); break;
    case " ": event.preventDefault(); toggleActiveSelection(); break;
    case "Enter":
      els.mediaStage.querySelector("video")?.play().catch(() => {});
      break;
    case "1": markActive("reviewed"); break;
    case "2": markActive("important"); break;
    case "3": markActive("needs_verification"); break;
    case "0": markActive(null); break;
    case "ArrowLeft": skipVideo(-10); break;
    case "ArrowRight": skipVideo(10); break;
    case "[": rateStep(-1); break;
    case "]": rateStep(1); break;
    case "i": setRangePoint("in"); break;
    case "o": setRangePoint("out"); break;
    case "f": toggleFullscreen(); break;
    case "t": toggleTheater(); break;
    case "p": togglePip(); break;
    case "?": toggleShortcuts(true); break;
    case "Escape": toggleShortcuts(false); if (state.layout.theater) toggleTheater(); break;
    default: break;
  }
});

els.query.addEventListener("input", () => {
  clearTimeout(els.query._debounce);
  els.query._debounce = setTimeout(() => {
    state.query = els.query.value.trim().toLowerCase();
    state.currentPage = 1;
    render();
  }, 150);
});
els.kind.addEventListener("change", () => { state.kind = els.kind.value; state.currentPage = 1; render(); });
els.status.addEventListener("change", () => { state.status = els.status.value; state.currentPage = 1; render(); });
els.pageSize.addEventListener("change", () => {
  state.pageSize = Number(els.pageSize.value) || 100;
  state.currentPage = 1;
  render();
});
els.prevPage.addEventListener("click", () => { state.currentPage -= 1; render(); });
els.nextPage.addEventListener("click", () => { state.currentPage += 1; render(); });
els.sortBy.addEventListener("change", () => { state.sortBy = els.sortBy.value; state.currentPage = 1; render(); });
els.groupBy.addEventListener("change", () => { state.groupBy = els.groupBy.value; state.currentPage = 1; render(); });
els.tagFilter.addEventListener("change", () => {
  state.chip = els.tagFilter.value;
  state.status = "";
  els.status.value = "";
  state.currentPage = 1;
  render();
});
els.dateFrom.addEventListener("change", () => { state.dateFrom = els.dateFrom.value; state.currentPage = 1; render(); });
els.dateTo.addEventListener("change", () => { state.dateTo = els.dateTo.value; state.currentPage = 1; render(); });
document.getElementById("btnClearDates").addEventListener("click", () => {
  state.dateFrom = "";
  state.dateTo = "";
  els.dateFrom.value = "";
  els.dateTo.value = "";
  state.currentPage = 1;
  render();
});
els.videoMode.addEventListener("change", () => { state.layout.videoMode = els.videoMode.value; saveLayout(); applyVideoScale(); });
els.videoZoom.addEventListener("input", () => {
  state.layout.videoMode = els.videoMode.value === "fit" ? "fit" : els.videoMode.value;
  if (els.videoMode.value !== "fit") state.layout.videoMode = els.videoMode.value;
  state.layout.videoZoom = Number(els.videoZoom.value);
  if (els.videoMode.value === "fit") { els.videoMode.value = "fit"; }
  saveLayoutSoon();
  applyVideoScale();
});
document.getElementById("btnTheater").addEventListener("click", toggleTheater);
document.getElementById("btnFullscreen").addEventListener("click", toggleFullscreen);
document.getElementById("btnPip").addEventListener("click", togglePip);
document.getElementById("btnSkipBack").addEventListener("click", () => skipVideo(-10));
document.getElementById("btnSkipFwd").addEventListener("click", () => skipVideo(10));
document.getElementById("playRate").addEventListener("change", () => setRate(Number(els.playRate.value) || 1));
document.getElementById("btnPopPlayer").addEventListener("click", openPlayerWindow);
document.getElementById("btnDual").addEventListener("click", () => {
  state.layout.dual = !state.layout.dual;
  saveLayout();
  applyLayout();
  if (state.layout.dual) {
    const record = selectedRecord();
    if (!record || !matesOf(record).length) toast(t("toast.noPair"));
  }
  mediaRenderedFor = null;
  renderDetails();
});
document.getElementById("btnTelemetry").addEventListener("click", async (ev) => {
  const record = selectedRecord();
  if (!record) return;
  const btn = ev.currentTarget;
  btn.disabled = true;
  try {
    const res = await fetch("/api/telemetry", {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: JSON.stringify({ id: record.id, path: record.path || "" })
    });
    const data = await res.json();
    if (data.ok && data.report) {
      record.telemetry = data.report;
      if (DATA.telemetry) DATA.telemetry[String(record.id).replace(/[:\\\/]/g, "_")] = data.report;
      renderDetails();
      toast(tf("toast.telemetryDone", { n: data.report.point_count || 0 }));
    } else {
      toast(tf("toast.telemetryFail", { err: data.error || "" }));
    }
  } catch {
    toast(t("toast.telemetryOffline"));
  } finally {
    btn.disabled = false;
  }
});

// Grab the decoded frame straight off the <video> element — no ffmpeg round
// trip — and store it as a hashed case artifact (artifacts/captures/).
async function captureCurrentFrame(videoEl, recordId) {
  if (!videoEl || !videoEl.videoWidth) { toast(t("toast.noVideo")); return; }
  const canvas = document.createElement("canvas");
  canvas.width = videoEl.videoWidth;
  canvas.height = videoEl.videoHeight;
  canvas.getContext("2d").drawImage(videoEl, 0, 0);
  const dataUrl = canvas.toDataURL("image/jpeg", 0.92);
  const b64 = dataUrl.slice(dataUrl.indexOf(",") + 1);
  try {
    const res = await fetch("/api/capture-frame", {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: JSON.stringify({ id: recordId, image: b64, time: videoEl.currentTime.toFixed(2) })
    });
    const data = await res.json();
    toast(data.ok ? tf("toast.captureSaved", { path: data.path }) : tf("toast.captureFail", { err: data.error || "" }));
  } catch {
    toast(t("toast.captureOffline"));
  }
}
document.getElementById("btnCaptureFrame").addEventListener("click", () => {
  const record = selectedRecord();
  captureCurrentFrame(currentVideo(), record ? record.id : "unknown");
});

// --- 구간(인/아웃) 마킹 + 클립보내기: 기록별로 localStorage에 유지됨 ---
function rangeOf(record) { return record ? state.ranges[record.id] : null; }
function updateRangeLabel() {
  const el = document.getElementById("rangeLabel");
  const record = selectedRecord();
  const range = rangeOf(record);
  const point = value => Number.isFinite(value) ? `${value.toFixed(1)}s` : "—";
  el.textContent = range && (Number.isFinite(range.in) || Number.isFinite(range.out))
    ? tf("range.label", { in: point(range.in), out: point(range.out) })
    : "";
  const proxyBtn = document.getElementById("btnProxy");
  if (proxyBtn) proxyBtn.classList.toggle("on", !!(record && state.proxies[record.id]));
}
function setRangePoint(which) {
  const video = currentVideo();
  const record = selectedRecord();
  if (!video || !record) { toast(t("toast.pickEvidence")); return; }
  const t = video.currentTime;
  const range = state.ranges[record.id] || { in: null, out: null };
  if (which === "in") range.in = t; else range.out = t;
  if (Number.isFinite(range.in) && Number.isFinite(range.out) && range.out <= range.in) {
    toast(t("toast.outBeforeIn"));
    if (which === "in") range.in = null; else range.out = null;
  }
  if (range.in == null && range.out == null) delete state.ranges[record.id];
  else state.ranges[record.id] = range;
  storageSet(RANGES_KEY, state.ranges);
  updateRangeLabel();
}
document.getElementById("btnSetIn").addEventListener("click", () => setRangePoint("in"));
document.getElementById("btnSetOut").addEventListener("click", () => setRangePoint("out"));
// Shared proxy request used by the manual toggle and by the automatic
// fallback when the selected record's container can't decode in-browser.
// Returns the proxy path on success; throws with the server error text.
async function requestReviewProxy(record) {
  const res = await fetch("/api/proxy", {
    method: "POST",
    headers: { "Content-Type": "text/plain" },
    body: JSON.stringify({ id: record.id, path: record.path || "" })
  });
  const data = await res.json();
  if (!data.ok) throw new Error(data.error || "proxy failed");
  state.proxies[record.id] = data.path;
  storageSet(PROXIES_KEY, state.proxies);
  mediaRenderedFor = null;
  render();
  return data.path;
}

// id → "building" | "failed:<err>" — one automatic attempt per record per
// page session; a failure is shown honestly, never retried in a loop.
const proxyAutoState = new Map();
function autoRequestProxy(record) {
  proxyAutoState.set(record.id, "building");
  requestReviewProxy(record).catch(err => {
    proxyAutoState.set(record.id, "failed:" + (err?.message || ""));
    mediaRenderedFor = null;
    render();
  });
}

// Review proxy toggle: heavy originals (4K/HEVC) stutter on exam machines,
// so the viewer can lazily ask the server for a low-bitrate proxy.
document.getElementById("btnProxy").addEventListener("click", async () => {
  const record = selectedRecord();
  if (!record) { toast(t("toast.playPick")); return; }
  const btn = document.getElementById("btnProxy");
  if (state.proxies[record.id]) {
    delete state.proxies[record.id];
    storageSet(PROXIES_KEY, state.proxies);
    mediaRenderedFor = null;
    render();
    toast(t("toast.playOriginal"));
    return;
  }
  btn.disabled = true;
  toast(t("toast.proxyBuilding"));
  try {
    await requestReviewProxy(record);
    toast(t("toast.proxyPlaying"));
  } catch (err) {
    toast(tf("toast.proxyFail", { err: err?.message || "" }));
  } finally {
    btn.disabled = false;
  }
});
document.getElementById("btnExportClip").addEventListener("click", async () => {
  const record = selectedRecord();
  const range = rangeOf(record);
  if (!record || !range || !Number.isFinite(range.in) || !Number.isFinite(range.out) || range.out <= range.in) {
    toast(t("toast.noRange"));
    return;
  }
  try {
    const res = await fetch("/api/export-clip", {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: JSON.stringify({ id: record.id, path: record.path || "", start: range.in.toFixed(3), duration: (range.out - range.in).toFixed(3) })
    });
    const data = await res.json();
    toast(data.ok ? tf("toast.clipDone", { path: data.path }) : tf("toast.clipFail", { err: data.error || "" }));
  } catch {
    toast(t("toast.clipOffline"));
  }
});
document.getElementById("btnExportClipBurn").addEventListener("click", async () => {
  const record = selectedRecord();
  const range = rangeOf(record);
  if (!record || !range || !Number.isFinite(range.in) || !Number.isFinite(range.out) || range.out <= range.in) {
    toast(t("toast.noRange"));
    return;
  }
  const exhibit = window.prompt(t("clip.exhibitPrompt"), t("clip.exhibitDefault")) ?? "";
  try {
    const res = await fetch("/api/export-clip", {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: JSON.stringify({ id: record.id, path: record.path || "", start: range.in.toFixed(3), duration: (range.out - range.in).toFixed(3), burn_in: "true", exhibit })
    });
    const data = await res.json();
    toast(data.ok ? tf("toast.clipDone", { path: data.path }) : tf("toast.clipFail", { err: data.error || "" }));
  } catch {
    toast(t("toast.clipOffline"));
  }
});
document.getElementById("btnShortcuts").addEventListener("click", () => toggleShortcuts(true));
document.getElementById("btnShortcutsClose").addEventListener("click", () => toggleShortcuts(false));
document.getElementById("btnSelectFiltered").addEventListener("click", selectAllFiltered);
document.getElementById("btnClearSelection").addEventListener("click", clearSelection);
document.getElementById("btnMarkReviewed").addEventListener("click", () => applyMark("reviewed"));
document.getElementById("btnMarkImportant").addEventListener("click", () => applyMark("important"));
document.getElementById("btnMarkVerify").addEventListener("click", () => applyMark("needs_verification"));
document.getElementById("btnMarkClear").addEventListener("click", () => applyMark(null));
document.getElementById("btnCopyIds").addEventListener("click", () => {
  const ids = targetIds();
  if (ids.length) copyText(ids.join("\n"), t("copy.ids"));
});
document.getElementById("btnCopyPaths").addEventListener("click", () => {
  const ids = new Set(targetIds());
  const paths = records.filter(record => ids.has(record.id)).map(record => record.path);
  if (paths.length) copyText(paths.join("\n"), t("copy.paths"));
  else toast(t("toast.selectTarget"));
});
// Tag menu: preset buttons are rebuilt from the editable preset list.
// Each row applies the tag to the current selection; the trailing ×
// removes the preset (applied tags on records are untouched). The
// input registers a new preset — and, when a selection exists, applies
// it right away.
const tagMenuList = document.getElementById("tagMenuList");
function rebuildTagMenu() {
  tagMenuList.innerHTML = state.tagPresets.map(tag =>
    `<div class="tag-menu-row"><button type="button" data-keep data-tag="${escapeHtml(tag)}">${escapeHtml(tag)}</button><button type="button" class="tag-del" data-keep data-del-preset="${escapeHtml(tag)}" title="${t("tagmenu.delPreset")}">×</button></div>`
  ).join("")
    + `<button type="button" data-tag-clear>${t("tagmenu.clear")}</button><hr>`
    + `<div class="tag-add"><input type="text" id="tagPresetInput" placeholder="${t("tagmenu.new")}" maxlength="24">`
    + `<button type="button" data-keep data-tag-add>${t("tagmenu.add")}</button></div>`;
}
function addTagPresetFromMenu() {
  const input = document.getElementById("tagPresetInput");
  const name = (input?.value || "").trim();
  if (!name) return;
  const isNew = !state.tagPresets.includes(name);
  addTagPreset(name);
  rebuildTagMenu();
  document.getElementById("tagPresetInput")?.focus();
  if (targetIds().length) applyTag(name);
  else if (isNew) toast(tf("toast.presetAdded", { tag: name }));
}
tagMenuList.addEventListener("click", e => {
  const del = e.target.closest("[data-del-preset]");
  if (del) { removeTagPreset(del.dataset.delPreset); rebuildTagMenu(); return; }
  if (e.target.closest("[data-tag-add]")) { addTagPresetFromMenu(); return; }
  if (e.target.closest("[data-tag-clear]")) { clearTags(); return; }
  const apply = e.target.closest("[data-tag]");
  if (apply) applyTag(apply.dataset.tag);
});
tagMenuList.addEventListener("keydown", e => {
  if (e.key === "Enter" && e.target.id === "tagPresetInput") {
    e.preventDefault();
    addTagPresetFromMenu();
  }
});

// Selection-bar menus: toggle on the anchor button, close on outside
// click / Escape / a menu item without data-keep (tag items stay open so
// several tags can be applied to one selection).
const MENU_BUTTONS = { tagMenuList: "btnTagMenu", exportMenuList: "btnExportMenu", viewMenuList: "btnViewMenu" };
function setMenuExpanded(listId, open) {
  const btn = document.getElementById(MENU_BUTTONS[listId]);
  if (btn) btn.setAttribute("aria-expanded", open ? "true" : "false");
}
function closeAllMenus() {
  document.querySelectorAll(".menu-list").forEach(l => { l.hidden = true; setMenuExpanded(l.id, false); });
}
function toggleMenu(listId) {
  const list = document.getElementById(listId);
  const willOpen = list.hidden;
  closeAllMenus();
  list.hidden = !willOpen;
  setMenuExpanded(listId, !willOpen);
}
document.getElementById("btnTagMenu").addEventListener("click", e => {
  e.stopPropagation();
  rebuildTagMenu();
  toggleMenu("tagMenuList");
});
document.getElementById("btnExportMenu").addEventListener("click", e => {
  e.stopPropagation();
  toggleMenu("exportMenuList");
});
document.getElementById("btnViewMenu").addEventListener("click", e => {
  e.stopPropagation();
  toggleMenu("viewMenuList");
});
document.getElementById("btnMoreFilters").addEventListener("click", e => {
  const extra = document.getElementById("filtersExtra");
  extra.hidden = !extra.hidden;
  e.currentTarget.setAttribute("aria-expanded", extra.hidden ? "false" : "true");
});
const btnGroupKind = document.getElementById("btnGroupKind");
btnGroupKind.addEventListener("click", () => {
  state.groupBy = state.groupBy === "kind" ? "none" : "kind";
  els.groupBy.value = state.groupBy;
  state.currentPage = 1;
  render();
});
const syncGroupKindChip = () => btnGroupKind.classList.toggle("active", state.groupBy === "kind");
els.groupBy.addEventListener("change", syncGroupKindChip);
document.querySelectorAll(".menu-list").forEach(list => {
  list.addEventListener("click", e => {
    // Keep the document-level closer from seeing in-menu clicks; an
    // action item without data-keep closes its own menu explicitly.
    e.stopPropagation();
    if (e.target.closest("button") && !e.target.closest("button").hasAttribute("data-keep")) {
      list.hidden = true;
      setMenuExpanded(list.id, false);
    }
  });
});
document.addEventListener("click", closeAllMenus);
document.addEventListener("keydown", e => {
  if (e.key === "Escape") closeAllMenus();
});
document.getElementById("btnDownloadSelection").addEventListener("click", () => {
  const selected = selectedRecords();
  if (!selected.length) { toast(t("toast.selectFirst")); return; }
  downloadJSON(`frametrace-selection-${manifest.case_id || "case"}-${Date.now()}.json`, {
    schema_version: 1,
    case_id: manifest.case_id || null,
    created_unix: Math.floor(Date.now() / 1000),
    items: selected.map(record => ({
      selector: record.id,
      kind: record.kind,
      action: record.kind === "video" ? "export" : record.kind === "candidate" ? "recover" : "validate",
      format: "mp4",
      notes: record.name
    }))
  });
});
