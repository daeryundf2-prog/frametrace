/* FrameTrace Evidence Viewer — serverless single-file review UI.
 * Data arrives via window.__FRAMETRACE_DATA__ (inlined by frametrace make-review).
 * Reviewer state (layout, marks, selection) persists in localStorage per case. */

const DATA = window.__FRAMETRACE_DATA__;
const manifest = DATA.manifest || {};
const scan = DATA.scan || {};
const carveLog = Array.isArray(DATA.carveLog) ? DATA.carveLog : [];
const filesystemLog = Array.isArray(DATA.filesystemLog) ? DATA.filesystemLog : [];
const validationLog = Array.isArray(DATA.validationLog) ? DATA.validationLog : [];

const BS = String.fromCharCode(92);
const EXT_PREFIX = BS + BS + "?" + BS;
const EXT_UNC = EXT_PREFIX + "unc" + BS;
const LAYOUT_KEY = "ft.viewer." + (manifest.case_id || "case") + ".layout.v2";
const MARKS_KEY = "ft.viewer." + (manifest.case_id || "case") + ".marks";
const MARK_STATUSES = ["reviewed", "important", "needs_verification"];

const videos = Array.isArray(scan.videos) ? scan.videos : [];

const flsEntryByInode = new Map();
(DATA.flsEntries || []).forEach(entry => {
  if (entry.inode != null) flsEntryByInode.set(String(entry.inode), entry);
});

const TIME_PATTERNS = [
  { re: /(20\d{2})[_.\-]?(0[1-9]|1[0-2])[_.\-]?(0[1-9]|[12]\d|3[01])[ T_\-]+([01]\d|2[0-3])[:_.\-]?([0-5]\d)[:_.\-]?([0-5]\d)/, hasTime: true },
  { re: /(20\d{2})(0[1-9]|1[0-2])(0[1-9]|[12]\d|3[01])[ T_\-]+([01]\d|2[0-3])([0-5]\d)([0-5]\d)/, hasTime: true },
  { re: /(20\d{2})[_.\-]?(0[1-9]|1[0-2])[_.\-]?(0[1-9]|[12]\d|3[01])(?!\d)/, hasTime: false }
];

function parseTimeFromName(name) {
  const text = String(name || "");
  for (const pattern of TIME_PATTERNS) {
    const match = text.match(pattern.re);
    if (!match) continue;
    const [year, month, day] = [match[1], match[2], match[3]];
    const hh = match[4] ?? "00";
    const mm = match[5] ?? "00";
    const ss = match[6] ?? "00";
    const ts = new Date(+year, +month - 1, +day, +hh, +mm, +ss).getTime() / 1000;
    if (Number.isFinite(ts)) {
      return { ts, date: `${year}-${month}-${day}`, source: "name" };
    }
  }
  return null;
}

function recordingTimeFor(record) {
  for (const candidate of [record.originalPath, record.name, record.path]) {
    const parsed = parseTimeFromName(candidate);
    if (parsed) return parsed;
  }
  if (record.kind === "video" && record.modifiedUnix) {
    const day = new Date(record.modifiedUnix * 1000).toLocaleDateString("sv-SE");
    return { ts: record.modifiedUnix, date: day, source: "mtime" };
  }
  return null;
}

function channelFor(record) {
  const name = `${record.originalPath || ""} ${record.name || ""}`;
  let match = name.match(/[_\-. ]([FRIB])(?:[_.\- ]|[a-z0-9]*$)/i);
  if (match) {
    const code = match[1].toUpperCase();
    return { F: "전방(F)", R: "후방(R)", I: "내부(I)", B: "후방2(B)" }[code] || code;
  }
  if (/front/i.test(name)) return "전방(F)";
  if (/rear/i.test(name)) return "후방(R)";
  if (/interior|inside/i.test(name)) return "내부(I)";
  return null;
}

function prefixFor(record) {
  const name = String(record.originalPath || record.name || "");
  const match = name.match(/[A-Za-z가-힣_\-]+/);
  return match ? match[0].replace(/[_\-]+$/, "") || "기타" : "기타";
}

function originalPathFor(record) {
  if (record.inode && flsEntryByInode.has(String(record.inode))) {
    return flsEntryByInode.get(String(record.inode)).path || "";
  }
  // Recovered outputs are named inode_<num>.bin; map that back to the
  // original path recorded by inspect-image.
  const match = String(record.name || "").match(/inode_(\d+)/);
  if (match && flsEntryByInode.has(match[1])) {
    return flsEntryByInode.get(match[1]).path || "";
  }
  return "";
}

function fmtUnix(value) {
  if (!Number.isFinite(value)) return "-";
  return new Date(value * 1000).toLocaleString();
}

function normalizePath(value) {
  let text = String(value || "");
  if (text.slice(0, 4).toLowerCase() === EXT_PREFIX) text = text.slice(4);
  return text.split(BS).join("/").toLowerCase();
}

function fileUrl(path) {
  if (!path) return "";
  let value = String(path);
  if (value.startsWith("file:")) return value;
  if (value.slice(0, 8).toLowerCase() === EXT_UNC) value = BS + BS + value.slice(8);
  else if (value.slice(0, 4).toLowerCase() === EXT_PREFIX) value = value.slice(4);
  const normalized = value.split(BS).join("/");
  if (normalized.length > 2 && normalized[1] === ":" && normalized[2] === "/") return "file:///" + encodeURI(normalized);
  if (normalized.startsWith("//")) return "file:" + encodeURI(normalized);
  if (normalized.startsWith("/")) return "file://" + encodeURI(normalized);
  return encodeURI(normalized);
}

function escapeHtml(value) {
  return String(value ?? "").replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;").replaceAll("'", "&#39;");
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function highlightEscape(value, query) {
  const safe = escapeHtml(value);
  if (!query) return safe;
  try {
    return safe.replaceAll(new RegExp("(" + escapeRegExp(escapeHtml(query)) + ")", "gi"), "<mark>$1</mark>");
  } catch (error) {
    return safe;
  }
}

function fmtBytes(value) {
  if (!Number.isFinite(value)) return "-";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let current = value;
  let unit = 0;
  while (current >= 1024 && unit < units.length - 1) { current /= 1024; unit += 1; }
  return `${current.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}

function fmtDuration(value) {
  if (!Number.isFinite(value)) return "-";
  const seconds = Math.round(value);
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  return h ? `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}` : `${m}:${String(s).padStart(2, "0")}`;
}

function statusLabel(status) {
  if (status === "ffprobe-video-stream-confirmed") return "검증됨";
  if (status === "ffprobe-confirmed") return "ffprobe 확인";
  if (status === "validation-failed") return "검증 실패";
  if (status === "candidate-unvalidated") return "미검증 후보";
  if (status === "duplicate-candidate") return "중복 후보";
  return status;
}

function statusClass(status) {
  if (status === "ffprobe-video-stream-confirmed" || status === "ffprobe-confirmed") return "ok";
  if (status === "validation-failed") return "failed";
  return "candidate";
}

function toast(message) {
  const host = document.getElementById("toastHost");
  const item = document.createElement("div");
  item.className = "toast";
  item.textContent = message;
  host.appendChild(item);
  setTimeout(() => item.remove(), 3200);
}

function storageGet(key, fallback) {
  try {
    const raw = localStorage.getItem(key);
    return raw ? JSON.parse(raw) : fallback;
  } catch (error) {
    return fallback;
  }
}

function storageSet(key, value) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
    return true;
  } catch (error) {
    toast("브라우저 저장소에 저장할 수 없습니다: " + error.message);
    return false;
  }
}

const validationsByPath = new Map(validationLog.map(item => [normalizePath(item.target_path), item]));
const recoveredFilesystemLog = filesystemLog.filter(item => item.event === "recover-inode" && item.output_path);

const records = [
  ...videos.map(video => {
    const validation = validationsByPath.get(normalizePath(video.source_path));
    return {
      id: video.id,
      kind: "video",
      name: video.relative_path || video.id,
      path: video.source_path,
      fileUrl: video.file_url,
      parser: video.source_profile?.parser || "-",
      vendor: video.source_profile?.vendor || "-",
      status: validation?.validation_status || (video.ffprobe_ok ? "ffprobe-confirmed" : "candidate-unvalidated"),
      sha256: validation?.target_sha256 || video.sha256 || video.hash_status || "-",
      duration: validation?.duration_seconds ?? video.duration_seconds,
      codec: validation?.video_codec || video.video_codec || "-",
      size: video.size_bytes,
      note: validation?.validation_note || video.source_profile?.recommended_action || "-",
      indexStatus: video.index_status || "active",
      modifiedUnix: video.modified_unix,
      inode: video.inode,
      validation
    };
  }),
  ...carveLog.map(item => {
    const validation = validationsByPath.get(normalizePath(item.output_path));
    return {
      id: item.id || item.output_path,
      kind: "carved",
      name: item.output_path ? item.output_path.split(/[\\\/]/).pop() : item.id,
      path: item.output_path,
      fileUrl: fileUrl(item.output_path),
      parser: item.signature || "carve",
      vendor: "Recovered candidate",
      status: validation?.validation_status || item.validation_status || "candidate-unvalidated",
      sha256: validation?.target_sha256 || item.sha256 || "-",
      duration: validation?.duration_seconds,
      codec: validation?.video_codec || item.extension || "-",
      size: item.size_bytes,
      note: validation?.validation_note || item.validation_note || "-",
      offset: item.offset,
      indexStatus: "active",
      validation
    };
  }),
  ...recoveredFilesystemLog.map(item => {
    const original = item.inode && flsEntryByInode.get(String(item.inode));
    const validation = validationsByPath.get(normalizePath(item.output_path));
    return {
      id: `inode:${item.partition_offset ?? 0}:${item.inode || item.output_path}`,
      kind: "filesystem",
      name: item.output_path ? item.output_path.split(/[\\\/]/).pop() : item.inode,
      path: item.output_path,
      fileUrl: fileUrl(item.output_path),
      parser: "tsk/icat",
      vendor: "Filesystem recovery",
      status: validation?.validation_status || item.validation_status || "candidate-unvalidated",
      sha256: validation?.target_sha256 || item.sha256 || "-",
      duration: validation?.duration_seconds,
      codec: validation?.video_codec || "-",
      size: item.size_bytes,
      note: validation?.validation_note || "Recovered inode output; validate before final reporting.",
      offset: item.partition_offset,
      inode: item.inode,
      originalPath: original?.path || "",
      indexStatus: "active",
      validation
    };
  })
];

records.forEach(record => {
  record.originalPath = record.originalPath || originalPathFor(record);
  const rec = recordingTimeFor(record);
  record.recTime = rec ? rec.ts : null;
  record.recDay = rec ? rec.date : null;
  record.recSource = rec ? rec.source : null;
  record.channel = channelFor(record);
  record.prefix = prefixFor(record);
});

const state = {
  activeId: records[0]?.id || null,
  selectedIds: new Set(),
  lastCheckedKey: null,
  marks: storageGet(MARKS_KEY, {}),
  layout: Object.assign({ col1: 50, col1Unit: "%", col3: 300, inspectorOpen: false, videoMode: "fit", videoZoom: 100, theater: false }, storageGet(LAYOUT_KEY, {})),
  currentPage: 1,
  pageSize: 100,
  query: "",
  kind: "",
  status: "",
  chip: "",
  sortBy: "id",
  groupBy: "none",
  dateFrom: "",
  dateTo: "",
  collapsedGroups: new Set()
};

const els = {
  caseLine: document.getElementById("caseLine"),
  resultCount: document.getElementById("resultCount"),
  metricVideos: document.getElementById("metricVideos"),
  metricCarved: document.getElementById("metricCarved"),
  metricVerified: document.getElementById("metricVerified"),
  metricFailed: document.getElementById("metricFailed"),
  query: document.getElementById("query"),
  kind: document.getElementById("kind"),
  status: document.getElementById("status"),
  pageSize: document.getElementById("pageSize"),
  presetChips: document.getElementById("presetChips"),
  recordList: document.getElementById("recordList"),
  prevPage: document.getElementById("prevPage"),
  nextPage: document.getElementById("nextPage"),
  pageStatus: document.getElementById("pageStatus"),
  mediaTitle: document.getElementById("mediaTitle"),
  mediaStatus: document.getElementById("mediaStatus"),
  mediaStage: document.getElementById("mediaStage"),
  summaryList: document.getElementById("summaryList"),
  metaList: document.getElementById("metaList"),
  validationList: document.getElementById("validationList"),
  selectionBar: document.getElementById("selectionBar"),
  selectionCount: document.getElementById("selectionCount"),
  videoMode: document.getElementById("videoMode"),
  videoZoom: document.getElementById("videoZoom"),
  shell: document.getElementById("shell"),
  sortBy: document.getElementById("sortBy"),
  groupBy: document.getElementById("groupBy"),
  dateFrom: document.getElementById("dateFrom"),
  dateTo: document.getElementById("dateTo"),
  dayHistogram: document.getElementById("dayHistogram"),
  periodLabel: document.getElementById("periodLabel")
};

const PRESET_CHIPS = [
  ["", "전체"],
  ["status:validation-failed", "검증 실패"],
  ["status:candidate-unvalidated", "미검증 후보"],
  ["status:duplicate-candidate", "중복 후보"],
  ["mark:important", "중요 마크"],
  ["mark:reviewed", "판독 완료"],
  ["mark:none", "판독 대기"]
];

function filteredRecords() {
  const list = records.filter(record => {
    if (state.kind && record.kind !== state.kind) return false;
    if (state.status && record.status !== state.status) return false;
    if (state.dateFrom || state.dateTo) {
      if (!record.recDay) return false;
      if (state.dateFrom && record.recDay < state.dateFrom) return false;
      if (state.dateTo && record.recDay > state.dateTo) return false;
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
    case "day": return record.recDay || "시각 미상";
    case "kind": return { video: "원본 (논리 파일)", carved: "카빙 후보", filesystem: "파일시스템 복구" }[record.kind] || record.kind;
    case "status": return statusLabel(record.status);
    case "mark": return markOf(record) ? markLabel(markOf(record).status) : "마크 없음";
    case "prefix": return record.prefix;
    case "channel": return record.channel || "채널 미상";
    default: return "";
  }
}

function selectedRecord() { return records.find(record => record.id === state.activeId) || records[0]; }
function markOf(record) { return state.marks[record.id] || null; }

function saveLayout() {
  const { col1, col1Unit, col3, inspectorOpen, videoMode, videoZoom, theater } = state.layout;
  storageSet(LAYOUT_KEY, { col1, col1Unit, col3, inspectorOpen, videoMode, videoZoom, theater });
}

let layoutSaveTimer = null;
function saveLayoutSoon() {
  clearTimeout(layoutSaveTimer);
  layoutSaveTimer = setTimeout(saveLayout, 250);
}

function applyLayout() {
  const layout = state.layout;
  const col1 = layout.col1Unit === "%"
    ? `${Math.max(15, Math.min(80, layout.col1))}%`
    : `${Math.max(280, layout.col1)}px`;
  els.shell.style.setProperty("--col1", col1);
  els.shell.style.setProperty("--col3", Math.max(220, layout.col3) + "px");
  document.body.classList.toggle("theater", !!layout.theater);
  applyResponsive();
  applyVideoScale();
  els.videoMode.value = layout.videoMode;
  els.videoZoom.value = String(layout.videoZoom);
}

function applyResponsive() {
  const width = window.innerWidth;
  document.body.classList.toggle("narrow", width <= 1280 && width > 960);
  document.body.classList.toggle("stack", width <= 960);
  const narrow = document.body.classList.contains("narrow");
  document.getElementById("panelInspector").classList.toggle("open-drawer", narrow && !!state.layout.inspectorOpen);
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

function renderList() {
  const filtered = filteredRecords();
  if (!filtered.some(record => record.id === state.activeId)) {
    state.activeId = filtered[0]?.id || records[0]?.id || null;
  }
  const pageCount = Math.max(1, Math.ceil(filtered.length / state.pageSize));
  state.currentPage = Math.min(Math.max(1, state.currentPage), pageCount);
  const start = (state.currentPage - 1) * state.pageSize;
  const pageRows = filtered.slice(start, start + state.pageSize);

  els.resultCount.textContent = `${filtered.length}`;
  els.pageStatus.textContent = `${state.currentPage} / ${pageCount}`;
  els.prevPage.disabled = state.currentPage <= 1;
  els.nextPage.disabled = state.currentPage >= pageCount;

  const groupCounts = new Map();
  if (state.groupBy !== "none") {
    filtered.forEach(record => {
      const key = groupKeyFor(record);
      groupCounts.set(key, (groupCounts.get(key) || 0) + 1);
    });
  }

  const rowsHtml = [];
  let lastGroup = null;
  pageRows.forEach(record => {
    if (state.groupBy !== "none") {
      const key = groupKeyFor(record);
      if (key !== lastGroup) {
        lastGroup = key;
        const collapsed = state.collapsedGroups.has(key);
        rowsHtml.push(`<div class="group-header" data-group="${escapeHtml(key)}"><span>${escapeHtml(key)}</span><span class="muted">${groupCounts.get(key) || 0}건${collapsed ? " · 접힘" : ""}</span></div>`);
        if (collapsed) return;
      }
    }
    rowsHtml.push(renderRow(record));
  });
  els.recordList.innerHTML = rowsHtml.join("") || `<div class="fallback">일치하는 증거가 없습니다.</div>`;

  els.recordList.querySelectorAll(".group-header").forEach(header => {
    header.addEventListener("click", () => {
      const key = header.dataset.group;
      if (state.collapsedGroups.has(key)) state.collapsedGroups.delete(key);
      else state.collapsedGroups.add(key);
      renderList();
    });
  });
  els.recordList.querySelectorAll(".row").forEach(row => {
    row.addEventListener("click", event => {
      if (event.target.closest(".check-cell")) return;
      state.activeId = row.dataset.id;
      render();
    });
  });
  els.recordList.querySelectorAll("input[type='checkbox']").forEach(box => {
    // change (not click): label-wrapped boxes can deliver a forwarded click
    // on top of the direct one, which would toggle selection twice. change
    // carries no modifier state, so click captures shift first.
    let shiftRange = false;
    box.addEventListener("click", event => {
      event.stopPropagation();
      shiftRange = event.shiftKey && !!state.lastCheckedKey;
    });
    box.addEventListener("change", () => {
      const id = box.dataset.check;
      if (shiftRange) {
        selectRange(state.lastCheckedKey, id, box.checked);
      } else if (box.checked) {
        state.selectedIds.add(id);
      } else {
        state.selectedIds.delete(id);
      }
      state.lastCheckedKey = id;
      render();
    });
  });
}

function renderRow(record) {
  const mark = markOf(record);
  const markChip = mark ? `<span class="mark-chip ${escapeHtml(mark.status)}">${escapeHtml(markLabel(mark.status))}</span>` : "";
  const staleTag = record.indexStatus === "stale" ? ' <span class="muted">(stale)</span>' : "";
  const recWhen = record.recTime ? " · " + fmtUnix(record.recTime) : "";
  const original = record.originalPath ? `<code title="복구 전 원본 경로">${highlightEscape(record.originalPath, state.query)}</code>` : "";
  return `<div class="row ${record.id === state.activeId ? "active" : ""}" data-id="${escapeHtml(record.id)}">
      <label class="check-cell" title="선택"><input type="checkbox" aria-label="선택" ${state.selectedIds.has(record.id) ? "checked" : ""} data-check="${escapeHtml(record.id)}"></label>
      <span class="badge ${statusClass(record.status)}" title="${escapeHtml(record.status)}">${escapeHtml(statusLabel(record.status))}</span>
      <span class="cell-main"><strong>${highlightEscape(record.name, state.query)}</strong>${original}<code>${highlightEscape(record.path, state.query)}</code><span class="muted">${highlightEscape(record.vendor, state.query)} · ${highlightEscape(record.parser, state.query)}${escapeHtml(recWhen)}</span>${markChip}${staleTag}</span>
      <span class="muted kind-cell">${escapeHtml(record.kind)}</span>
    </div>`;
}

function renderHistogram() {
  const days = new Map();
  filteredRecords().forEach(record => {
    if (record.recDay) days.set(record.recDay, (days.get(record.recDay) || 0) + 1);
  });
  const top = [...days.entries()].sort((a, b) => (a[0] < b[0] ? -1 : 1)).slice(-16);
  const max = Math.max(1, ...top.map(entry => entry[1]));
  els.dayHistogram.innerHTML = top.map(([day, count]) => `
    <button type="button" class="${state.dateFrom === day && state.dateTo === day ? "selected" : ""}" data-day="${escapeHtml(day)}" title="${escapeHtml(day)}: ${count}건">
      <span class="bar" style="height:${Math.round((count * 40) / max)}px"></span>
      <span class="lbl">${escapeHtml(day.slice(5))}</span>
    </button>`).join("")
    || `<span class="muted">시각 정보가 있는 증거가 없습니다 — 파일명 패턴 또는 수정시각에서 추출합니다.</span>`;
  els.dayHistogram.querySelectorAll("button[data-day]").forEach(button => {
    button.addEventListener("click", () => {
      state.dateFrom = button.dataset.day;
      state.dateTo = button.dataset.day;
      els.dateFrom.value = state.dateFrom;
      els.dateTo.value = state.dateTo;
      state.currentPage = 1;
      render();
    });
  });
  const known = records.filter(record => record.recDay).map(record => record.recDay).sort();
  els.periodLabel.textContent = known.length
    ? "녹화 기간: " + known[0] + " ~ " + known[known.length - 1] + " · 시각 확인 " + known.length + "/" + records.length + "건 (파일명·수정시각 추출)"
    : "녹화 시각을 추출한 증거가 없습니다 (파일명 패턴 또는 수정시각 필요)";
}

function markLabel(status) {
  if (status === "reviewed") return "판독 완료";
  if (status === "important") return "중요";
  if (status === "needs_verification") return "검증 대기";
  return status;
}

function indexOfFiltered(id) {
  return filteredRecords().findIndex(record => record.id === id);
}

function selectRange(fromId, toId, checked) {
  const filtered = filteredRecords();
  const from = filtered.findIndex(record => record.id === fromId);
  const to = filtered.findIndex(record => record.id === toId);
  if (from < 0 || to < 0) return;
  const [low, high] = from <= to ? [from, to] : [to, from];
  for (let index = low; index <= high; index += 1) {
    if (checked) state.selectedIds.add(filtered[index].id);
    else state.selectedIds.delete(filtered[index].id);
  }
}

function selectAllFiltered() {
  filteredRecords().forEach(record => state.selectedIds.add(record.id));
  render();
  toast(`필터 결과 ${state.selectedIds.size}개를 선택했습니다.`);
}

function clearSelection() {
  state.selectedIds.clear();
  render();
}

function targetIds() {
  if (state.selectedIds.size) return [...state.selectedIds];
  return state.activeId ? [state.activeId] : [];
}

function applyMark(status) {
  const ids = targetIds();
  if (!ids.length) { toast("대상 증거를 먼저 선택하세요."); return; }
  const stamped = Math.floor(Date.now() / 1000);
  ids.forEach(id => {
    if (status === null) delete state.marks[id];
    else state.marks[id] = { status, marked_unix: stamped };
  });
  storageSet(MARKS_KEY, state.marks);
  render();
  toast(`${ids.length}개 증거에 '${status === null ? "마크 해제" : markLabel(status)}'를 적용했습니다.`);
}

let mediaRenderedFor = null;

function renderDetails() {
  const record = selectedRecord();
  if (!record) {
    els.mediaStage.innerHTML = `<div class="fallback">색인된 증거가 없습니다.</div>`;
    els.mediaTitle.textContent = "-";
    els.mediaStatus.textContent = "-";
    els.summaryList.innerHTML = "";
    els.metaList.innerHTML = "";
    els.validationList.innerHTML = "";
    mediaRenderedFor = null;
    return;
  }
  els.mediaTitle.textContent = record.name || record.id;
  els.mediaStatus.textContent = record.status;
  els.mediaStatus.className = `badge ${statusClass(record.status)}`;
  // Re-creating the <video> forces a metadata refetch on every render; only
  // swap it when the selected evidence actually changed.
  const mediaKey = `${record.id}:${record.fileUrl}`;
  if (mediaRenderedFor !== mediaKey) {
    els.mediaStage.innerHTML = record.fileUrl
      ? `<video controls preload="metadata" src="${escapeHtml(record.fileUrl)}"></video>`
      : `<div class="fallback">직접 재생 가능한 파일 URL이 없습니다.</div>`;
    els.mediaStage.querySelector("video")?.addEventListener("loadedmetadata", applyVideoScale);
    mediaRenderedFor = mediaKey;
  }
  applyVideoScale();
  const mark = markOf(record);
  els.summaryList.innerHTML = [
    ["ID", record.id],
    ["상태", record.status],
    ["판독", mark ? markLabel(mark.status) : "미판독"],
    ["메모", record.note],
    ["길이", fmtDuration(record.duration)]
  ].map(([k, v]) => `<dt>${escapeHtml(k)}</dt><dd>${escapeHtml(v)}</dd>`).join("");
  els.metaList.innerHTML = [
    ["경로", `<code>${escapeHtml(record.path)}</code>`],
    ["원본 경로", record.originalPath ? `<code>${escapeHtml(record.originalPath)}</code>` : "-"],
    ["촬영 시각", record.recTime ? `${escapeHtml(fmtUnix(record.recTime))} (${record.recSource === "name" ? "파일명" : "수정시각 추정"})` : "미상"],
    ["제조사", escapeHtml(record.vendor)],
    ["파서", `<code>${escapeHtml(record.parser)}</code>`],
    ["코덱", escapeHtml(record.codec)],
    ["크기", fmtBytes(record.size)],
    ["SHA-256", `<code>${escapeHtml(record.sha256)}</code>`],
    ["오프셋", record.offset ?? "-"]
  ].map(([k, v]) => `<dt>${escapeHtml(k)}</dt><dd>${v}</dd>`).join("");
  const related = validationLog.filter(item => normalizePath(item.target_path) === normalizePath(record.path) || item.selector === record.id);
  els.validationList.innerHTML = related.length ? related.map(item => `<div class="validation-item">
    <strong>${escapeHtml(item.validation_status || "-")}</strong>
    <div class="muted">${escapeHtml(item.validation_note || item.ffprobe_error || "-")}</div>
    <code>${escapeHtml(item.target_sha256 || "-")}</code>
  </div>`).join("") : `<div class="validation-item">검증 로그 없음</div>`;
}

function renderMetrics() {
  els.caseLine.textContent = `${manifest.case_id || "case"} · ${manifest.title || "Untitled"} · ${scan.source_path || "-"}`;
  els.metricVideos.textContent = videos.length;
  els.metricCarved.textContent = carveLog.length + recoveredFilesystemLog.length;
  els.metricVerified.textContent = records.filter(record => record.status === "ffprobe-video-stream-confirmed" || record.status === "ffprobe-confirmed").length;
  els.metricFailed.textContent = records.filter(record => record.status === "validation-failed").length;
  const markCount = Object.keys(state.marks).length;
  els.selectionCount.textContent = `${state.selectedIds.size}개 선택 · 마크 ${markCount}`;
}

function renderChips() {
  els.presetChips.innerHTML = PRESET_CHIPS.map(([value, label]) =>
    `<button type="button" class="chip ${state.chip === value ? "active" : ""}" data-chip="${escapeHtml(value)}">${escapeHtml(label)}</button>`
  ).join("");
  els.presetChips.querySelectorAll(".chip").forEach(chip => {
    chip.addEventListener("click", () => {
      const value = chip.dataset.chip;
      state.chip = value;
      if (value.startsWith("status:")) {
        state.status = value.slice(7);
        els.status.value = state.status;
      } else {
        state.status = "";
        els.status.value = "";
      }
      state.currentPage = 1;
      render();
    });
  });
}

function render() {
  renderMetrics();
  renderChips();
  renderHistogram();
  renderList();
  renderDetails();
}

function downloadJSON(filename, payload) {
  const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(link.href), 5000);
  toast(`${filename} 다운로드를 시작했습니다.`);
}

function selectedRecords() {
  return records.filter(record => state.selectedIds.has(record.id));
}

function copyText(text, label) {
  const done = () => toast(`${label}을(를) 클립보드에 복사했습니다.`);
  const fallback = () => {
    const area = document.createElement("textarea");
    area.value = text;
    document.body.appendChild(area);
    area.select();
    try {
      document.execCommand("copy");
      done();
    } catch (error) {
      toast("복사에 실패했습니다: " + error.message);
    }
    area.remove();
  };
  if (navigator.clipboard?.writeText) {
    navigator.clipboard.writeText(text).then(done, fallback);
  } else {
    fallback();
  }
}

function toggleTheater() {
  state.layout.theater = !state.layout.theater;
  saveLayout();
  applyLayout();
}

function toggleFullscreen() {
  const stage = document.getElementById("mediaStage");
  if (document.fullscreenElement) {
    document.exitFullscreen().catch(error => toast("전체화면 종료 실패: " + error.message));
    return;
  }
  if (stage.requestFullscreen) {
    stage.requestFullscreen().catch(error => toast("전체화면 진입 실패: " + error.message));
  } else {
    toast("이 브라우저는 전체화면을 지원하지 않습니다.");
  }
}

async function togglePip() {
  const video = els.mediaStage.querySelector("video");
  if (!video) { toast("재생 중인 영상이 없습니다."); return; }
  try {
    if (document.pictureInPictureElement) {
      await document.exitPictureInPicture();
    } else if (video.requestPictureInPicture) {
      await video.requestPictureInPicture();
    } else {
      toast("이 브라우저는 화면 속 화면을 지원하지 않습니다.");
    }
  } catch (error) {
    toast("화면 속 화면 실패: " + error.message);
  }
}

function moveActive(step) {
  const filtered = filteredRecords();
  if (!filtered.length) return;
  const index = filtered.findIndex(record => record.id === state.activeId);
  const next = Math.max(0, Math.min(filtered.length - 1, (index < 0 ? 0 : index + step)));
  state.activeId = filtered[next].id;
  const row = els.recordList.querySelector(`.row[data-id="${CSS.escape(state.activeId)}"]`);
  row?.scrollIntoView({ block: "nearest" });
  render();
}

function toggleActiveSelection() {
  if (!state.activeId) return;
  if (state.selectedIds.has(state.activeId)) state.selectedIds.delete(state.activeId);
  else state.selectedIds.add(state.activeId);
  render();
}

function toggleShortcuts(open) {
  document.getElementById("shortcutsModal").hidden = !open;
}

document.addEventListener("keydown", event => {
  const tag = document.activeElement?.tagName;
  if (tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA") {
    if (event.key === "Escape") document.activeElement.blur();
    return;
  }
  if (event.ctrlKey && (event.key === "a" || event.key === "A")) { event.preventDefault(); selectAllFiltered(); return; }
  if (event.ctrlKey && (event.key === "d" || event.key === "D")) { event.preventDefault(); clearSelection(); return; }
  switch (event.key) {
    case "j": moveActive(1); break;
    case "k": moveActive(-1); break;
    case " ": event.preventDefault(); toggleActiveSelection(); break;
    case "Enter": render(); break;
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
document.getElementById("btnShortcuts").addEventListener("click", () => toggleShortcuts(true));
document.getElementById("btnShortcutsClose").addEventListener("click", () => toggleShortcuts(false));
document.getElementById("btnInspectorClose").addEventListener("click", () => {
  state.layout.inspectorOpen = false;
  saveLayout();
  applyResponsive();
});
document.getElementById("btnSelectFiltered").addEventListener("click", selectAllFiltered);
document.getElementById("btnClearSelection").addEventListener("click", clearSelection);
document.getElementById("btnMarkReviewed").addEventListener("click", () => applyMark("reviewed"));
document.getElementById("btnMarkImportant").addEventListener("click", () => applyMark("important"));
document.getElementById("btnMarkVerify").addEventListener("click", () => applyMark("needs_verification"));
document.getElementById("btnMarkClear").addEventListener("click", () => applyMark(null));
document.getElementById("btnCopyIds").addEventListener("click", () => {
  const ids = targetIds();
  if (ids.length) copyText(ids.join("\n"), "증거 ID");
});
document.getElementById("btnCopyPaths").addEventListener("click", () => {
  const paths = selectedRecords().map(record => record.path);
  if (paths.length) copyText(paths.join("\n"), "증거 경로");
});
document.getElementById("btnDownloadSelection").addEventListener("click", () => {
  const selected = selectedRecords();
  if (!selected.length) { toast("먼저 증거를 선택하세요."); return; }
  downloadJSON(`frametrace-selection-${manifest.case_id || "case"}-${Date.now()}.json`, {
    schema_version: 1,
    case_id: manifest.case_id || null,
    created_unix: Math.floor(Date.now() / 1000),
    items: selected.map(record => ({
      selector: record.id,
      kind: record.kind,
      action: record.kind === "video" ? "export" : "validate",
      format: "mp4",
      notes: record.name
    }))
  });
});
document.getElementById("btnDownloadMarks").addEventListener("click", () => {
  const marks = Object.entries(state.marks).map(([id, mark]) => ({ id, status: mark.status, marked_unix: mark.marked_unix }));
  if (!marks.length) { toast("저장된 판독 마크가 없습니다."); return; }
  downloadJSON(`frametrace-marks-${manifest.case_id || "case"}.json`, {
    schema_version: 1,
    case_id: manifest.case_id || null,
    exported_unix: Math.floor(Date.now() / 1000),
    marks
  });
});

function setupSplitters() {
  const left = document.getElementById("splitterLeft");
  const right = document.getElementById("splitterRight");
  let drag = null;
  const begin = (event, side) => {
    drag = {
      side,
      startX: event.clientX,
      start1: state.layout.col1,
      start3: state.layout.col3,
      shellWidth: Math.max(1, els.shell.clientWidth)
    };
    event.target.classList.add("dragging");
    event.preventDefault();
  };
  const move = event => {
    if (!drag) return;
    if (drag.side === "left") {
      if (state.layout.col1Unit === "%") {
        state.layout.col1 = Math.max(15, Math.min(80,
          drag.start1 + ((event.clientX - drag.startX) / drag.shellWidth) * 100));
      } else {
        state.layout.col1 = Math.max(280, Math.min(760, drag.start1 + event.clientX - drag.startX));
      }
    } else {
      state.layout.col3 = Math.max(220, Math.min(560, drag.start3 - (event.clientX - drag.startX)));
    }
    applyLayout();
  };
  const end = () => {
    if (!drag) return;
    document.querySelectorAll(".splitter").forEach(item => item.classList.remove("dragging"));
    drag = null;
    saveLayout();
  };
  left.addEventListener("pointerdown", event => begin(event, "left"));
  right.addEventListener("pointerdown", event => begin(event, "right"));
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", end);
  left.addEventListener("dblclick", () => { state.layout.col1 = 50; state.layout.col1Unit = "%"; saveLayout(); applyLayout(); });
  right.addEventListener("dblclick", () => { state.layout.col3 = 300; saveLayout(); applyLayout(); });
}

window.addEventListener("resize", applyResponsive);

state.pageSize = Number(els.pageSize.value) || 100;
setupSplitters();
applyLayout();
render();
