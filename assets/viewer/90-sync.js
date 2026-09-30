function marksPayload() {
  // Marks and free-text notes travel together: a record with only a note
  // is exported with status "noted" so the case DB keeps the memo.
  const ids = new Set(Object.keys(state.marks));
  Object.keys(state.notes).forEach(id => { if ((state.notes[id] || "").trim()) ids.add(id); });
  const marks = [...ids].map(id => {
    const mark = state.marks[id];
    const note = (state.notes[id] || "").trim();
    return {
      id,
      status: mark ? mark.status : "noted",
      marked_unix: mark ? mark.marked_unix : Math.floor(Date.now() / 1000),
      note,
      examiner: state.examiners[id] ?? ((state.examiner || "").trim() || null)
    };
  });
  const tagEntries = Object.entries(state.tags).map(([id, tags]) => ({ id, tags }));
  return {
    schema_version: 2,
    protocol: "patch-v1",
    deleted_ids: state.deletedIds.filter(id => !ids.has(id)),
    case_id: manifest.case_id || null,
    examiner: (state.examiner || "").trim() || null,
    exported_unix: Math.floor(Date.now() / 1000),
    marks,
    tags: tagEntries
  };
}

document.getElementById("btnDownloadMarks").addEventListener("click", () => {
  const payload = marksPayload();
  if (!payload.marks.length && !payload.tags.length && !payload.deleted_ids.length) { toast(t("toast.noChanges")); return; }
  downloadJSON(`frametrace-marks-${manifest.case_id || "case"}.json`, payload);
});

// --- Review-change protection -------------------------------------------
// touchAnnotation() funnels every mark/note/tag mutation, and deletedIds
// already tracks records changed since the last server import. pendingReviewSync
// drives both the beforeunload guard and the debounced server auto-save, so
// examiner work can't silently die in localStorage.
let pendingReviewSync = state.deletedIds.length > 0;
let autosaveTimer = null;
const viewerIsServed = location.protocol === "http:" || location.protocol === "https:";

function markReviewDirty() {
  pendingReviewSync = true;
  if (!viewerIsServed) return; // file:// viewing — unload warning only
  clearTimeout(autosaveTimer);
  autosaveTimer = setTimeout(autoApplyMarks, 2500);
}

function clearReviewDirty() {
  pendingReviewSync = false;
  state.deletedIds = [];
  state.drafts = {};
  storageSet(ANNOTATIONS_KEY, state.drafts);
}

async function pushMarksToCase(payload) {
  const res = await fetch("/api/import-marks", {
    method: "POST",
    headers: { "Content-Type": "text/plain" },
    body: JSON.stringify({ marks_json: JSON.stringify(payload) })
  });
  return res.json();
}

async function autoApplyMarks() {
  if (!pendingReviewSync || !viewerIsServed) return;
  const payload = marksPayload();
  if (!payload.marks.length && !payload.tags.length && !payload.deleted_ids.length) {
    clearReviewDirty();
    return;
  }
  try {
    const data = await pushMarksToCase(payload);
    if (data.ok) clearReviewDirty();
  } catch { /* stay dirty — the next change retries */ }
}

window.addEventListener("beforeunload", (event) => {
  if (!pendingReviewSync && !state.deletedIds.length) return;
  event.preventDefault();
  event.returnValue = "";
});

// Server-served viewers can push marks straight into the case DB instead of
// the download → file-pick → import dance.
document.getElementById("btnApplyMarks").addEventListener("click", async () => {
  const payload = marksPayload();
  if (!payload.marks.length && !payload.tags.length && !payload.deleted_ids.length) { toast(t("toast.noChanges")); return; }
  try {
    const data = await pushMarksToCase(payload);
    if (data.ok) {
      clearReviewDirty();
      toast(t("toast.applyDone"));
    } else {
      toast(tf("toast.applyFail", { err: data.error || "?" }));
    }
  } catch (err) {
    toast(t("toast.applyOffline"));
  }
});

// --- 선별 결과 내보내기: 사람이 받는 형태 (CSV / 요약 리포트 / 자료 묶음) ---

document.getElementById("btnDownloadCsv").addEventListener("click", () => {
  const selected = selectedRecords();
  if (!selected.length) { toast(t("toast.selectFirst")); return; }
  const header = ["id", "kind", "name", "status", "mark", "examiner_note", "tags", "sha256", "size_bytes",
    "recorded_time", "channel", "rec_type", "inode", "original_path", "source_path",
    "warnings", "note", "anomalies"];
  const rows = selected.map(record => [
    record.id,
    KIND_LABELS[record.kind] || record.kind,
    record.name,
    record.status,
    state.marks[record.id]?.status || ((state.notes[record.id] || "").trim() ? "noted" : ""),
    (state.notes[record.id] || "").trim(),
    (state.tags[record.id] || []).join("; "),
    record.sha256,
    record.size ?? "",
    isoTime(record.recTime),
    record.channel || "",
    record.recType || "",
    record.inode ?? "",
    record.originalPath || "",
    record.path || "",
    (record.warnings || []).join("; "),
    record.note && record.note !== "-" ? record.note : "",
    (record.anomalies || []).map(item => item.kind).join("; ")
  ].map(csvCell).join(","));
  downloadText(
    `frametrace-selection-${manifest.case_id || "case"}.csv`,
    String.fromCharCode(0xFEFF) + header.join(",") + "\n" + rows.join("\n") + "\n",
    "text/csv;charset=utf-8"
  );
});

// A self-contained, printable handoff report: everything a requester
// needs to understand what was selected and why, without the viewer.
document.getElementById("btnSummary").addEventListener("click", () => {
  let items = selectedRecords();
  if (!items.length) {
    items = records.filter(record => state.marks[record.id] || (state.tags[record.id] || []).length || (state.notes[record.id] || "").trim());
  }
  if (!items.length) { toast(t("toast.noItems")); return; }
  const e = escapeHtml;
  const fmtSize = value => Number.isFinite(value) ? `${(value / 1048576).toFixed(1)} MB` : "-";
  const rowHtml = items.map(record => {
    const mark = state.marks[record.id]?.status;
    const memo = (state.notes[record.id] || "").trim();
    const warn = [...(record.warnings || []), ...(record.anomalies || []).map(item => item.kind)].join("; ");
    return `<tr><td>${e(record.id)}</td><td>${e(record.name)}</td><td>${e(KIND_LABELS[record.kind] || record.kind)}</td>` +
      `<td>${e(record.status)}</td><td>${e(mark ? markLabel(mark) : (memo ? t("mark.noted") : ""))}</td>` +
      `<td>${e((state.tags[record.id] || []).join(", "))}</td>` +
      `<td class="mono">${e(record.sha256 && record.sha256 !== "-" ? record.sha256 : "")}</td>` +
      `<td>${fmtSize(record.size)}</td><td>${e(isoTime(record.recTime))}</td>` +
      `<td>${e(warn)}${record.note && record.note !== "-" ? `<br>${e(record.note)}` : ""}${memo ? `<br><b>${t("report.memo")}</b> ${e(memo)}` : ""}</td>` +
      `<td class="mono">${e(record.originalPath || record.path || "")}</td></tr>`;
  }).join("\n");
  const counts = { verified: 0, candidate: 0, failed: 0 };
  items.forEach(record => {
    if (record.status === "validation-failed") counts.failed += 1;
    else if (record.status === "ffprobe-video-stream-confirmed") counts.verified += 1;
    else counts.candidate += 1;
  });
  const html = `<!doctype html><html lang="${state.locale}"><head><meta charset="utf-8">
<title>${t("report.title")} — ${e(manifest.case_id || "case")}</title>
<style>
body{font-family:ui-sans-serif,system-ui,"Segoe UI",sans-serif;margin:32px;color:#1f2724}
h1{font-size:20px} .meta{color:#68736f;font-size:13px;margin-bottom:4px}
.note{background:#fdf6e8;border:1px solid #e3cf9e;border-radius:6px;padding:10px 12px;font-size:13px;margin:14px 0}
table{border-collapse:collapse;width:100%;font-size:12px}
th,td{border:1px solid #d8dedb;padding:6px 8px;text-align:left;vertical-align:top}
th{background:#f2f5f4} .mono{font-family:Consolas,monospace;font-size:11px;word-break:break-all}
</style></head><body>
<h1>${t("report.title")}</h1>
<div class="meta">${tf("report.meta", { case: manifest.case_id || "-", when: new Date().toLocaleString(), n: items.length })}</div>
<div class="meta">${tf("report.counts", { v: counts.verified, c: counts.candidate, f: counts.failed })}</div>
<div class="note">${t("report.note")}</div>
<table><thead><tr><th>ID</th><th>${t("report.th.name")}</th><th>${t("report.th.kind")}</th><th>${t("report.th.status")}</th><th>${t("report.th.mark")}</th><th>${t("report.th.tags")}</th>
<th>SHA-256</th><th>${t("report.th.size")}</th><th>${t("report.th.time")}</th><th>${t("report.th.warn")}</th><th>${t("report.th.orig")}</th></tr></thead>
<tbody>
${rowHtml}
</tbody></table>
</body></html>`;
  downloadText(`frametrace-summary-${manifest.case_id || "case"}.html`, html, "text/html;charset=utf-8");
});

// 서버 측 자료 묶음 — 선택 파일을 exports/selection-*/ 에 해시 매니페스트와 함께 복사.
document.getElementById("btnExportSelected").addEventListener("click", async () => {
  const selected = selectedRecords();
  if (!selected.length) { toast(t("toast.selectFirst")); return; }
  const btn = document.getElementById("btnExportSelected");
  btn.disabled = true;
  try {
    const res = await fetch("/api/export-selected", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ items: selected.map(exportItem) })
    }).then(reply => reply.json());
    if (!res.ok) {
      toast(tf("toast.exportFail", { err: res.error || "" }));
      return;
    }
    toast(tf("toast.exportDone", { copied: res.copied, skipped: res.skipped, dir: res.export_dir }));
    fetch("/api/open-folder", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path: res.export_dir })
    }).catch(() => {});
  } catch (err) {
    toast(t("toast.offline"));
  } finally {
    btn.disabled = false;
  }
});

// --- 원본 파일 다운로드: /media?path&download=1이 Content-Disposition으로
// 저장을 유도. file:// 스탠드얼론 번들에서는 서버가 없으므로 원본/프록시의
// file:// 경로로 폴백한다 — 브라우저가 렌더할 수 없는 형식(AVI/.bin/DAV)은
// 네비게이션 대신 그대로 저장되고, 재생 가능한 파일은 새 탭에서 열린다. ---
function downloadHref(record) {
  if (location.protocol === "http:" || location.protocol === "https:") {
    if (!record.path) return "";
    return "/media?path=" + encodeURIComponent(record.path) + "&download=1";
  }
  if (record.fileUrl) return record.fileUrl;
  if (record.proxyPath) return fileUrl(record.proxyPath);
  return "";
}
function triggerDownload(record) {
  const href = downloadHref(record);
  if (!href) return false;
  const a = document.createElement("a");
  a.href = href;
  a.download = record.originalName || record.name || record.id;
  // Standalone file:// fallback navigates instead of downloading for
  // renderable types — keep the viewer alive by opening a new tab.
  if (location.protocol !== "http:" && location.protocol !== "https:") a.target = "_blank";
  document.body.appendChild(a);
  a.click();
  a.remove();
  return true;
}
// Folder-pick saves need a fetchable source: /media over http(s). file://
// pages cannot fetch file:// URLs (CORS), so standalone falls back to
// browser downloads below.
function canFolderSave() {
  return typeof window.showDirectoryPicker === "function" &&
    (location.protocol === "http:" || location.protocol === "https:");
}
function safeFileName(name) {
  return String(name || "evidence").replace(/[\\/:*?"<>|]/g, "_") || "evidence";
}
async function writeRecordTo(dir, record) {
  const href = downloadHref(record);
  if (!href) return false;
  const res = await fetch(href);
  if (!res.ok || !res.body) return false;
  const handle = await dir.getFileHandle(safeFileName(record.originalName || record.name || record.id), { create: true });
  const writable = await handle.createWritable();
  await res.body.pipeTo(writable);
  return true;
}
document.getElementById("btnDownloadFile").addEventListener("click", async () => {
  const record = selectedRecord();
  if (!record) { toast(t("toast.selectFirst")); return; }
  if (canFolderSave() && typeof window.showSaveFilePicker === "function") {
    try {
      const handle = await showSaveFilePicker({ suggestedName: safeFileName(record.originalName || record.name || record.id) });
      const href = downloadHref(record);
      if (!href) { toast(t("toast.fileOnly")); return; }
      const res = await fetch(href);
      if (!res.ok || !res.body) { toast(t("toast.fileOnly")); return; }
      const writable = await handle.createWritable();
      await res.body.pipeTo(writable);
      toast(tf("toast.fileSaved", { name: handle.name }));
    } catch (error) {
      if (error.name !== "AbortError") toast(tf("toast.folderFail", { err: error.message }));
    }
    return;
  }
  if (!triggerDownload(record)) {
    toast(t("toast.fileOnly"));
  }
});
document.getElementById("btnDownloadFiles").addEventListener("click", async () => {
  const selected = selectedRecords();
  if (!selected.length) { toast(t("toast.selectFirst")); return; }
  if (canFolderSave()) {
    try {
      const dir = await showDirectoryPicker({ mode: "readwrite" });
      let saved = 0, failed = 0;
      for (const record of selected) {
        try {
          if (await writeRecordTo(dir, record)) saved += 1;
          else failed += 1;
        } catch {
          failed += 1;
        }
      }
      toast(failed ? tf("toast.folderPartial", { n: saved, f: failed }) : tf("toast.folderSaved", { n: saved }));
    } catch (error) {
      if (error.name !== "AbortError") toast(tf("toast.folderFail", { err: error.message }));
    }
    return;
  }
  let started = 0;
  selected.forEach((record, index) => {
    if (!downloadHref(record)) return;
    started += 1;
    setTimeout(() => triggerDownload(record), index * 450);
  });
  if (!started) {
    toast(t("toast.fileOnly"));
  } else {
    toast(tf("toast.downloads", { n: started }));
  }
});
document.getElementById("btnTranscodeQueue").addEventListener("click", async (ev) => {
  const btn = ev.currentTarget;
  const ids = selectedRecords().map(r => r.id).join(",");
  btn.disabled = true;
  toast(t("toast.queueRunning"));
  try {
    const res = await fetch("/api/transcode-queue", {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: JSON.stringify({ ids })
    });
    const data = await res.json();
    if (data.ok && data.summary) {
      toast(tf("toast.queueDone", { t: data.summary.total ?? 0, p: data.summary.proxied ?? 0, c: data.summary.skipped_existing ?? 0, f: data.summary.failed ?? 0 }));
    } else {
      toast(tf("toast.queueFail", { err: data.error || "" }));
    }
  } catch {
    toast(t("toast.queueOffline"));
  } finally {
    btn.disabled = false;
  }
});

// --- 케이스 타임라인 패널: db/timeline.jsonl을 읽어 시간순 이벤트를 표시 ---
async function loadTimeline(regenerate) {
  const list = document.getElementById("timelineList");
  const meta = document.getElementById("timelineMeta");
  if (location.protocol === "file:") {
    list.innerHTML = `<div class="timeline-row"><span class="desc">${t("timeline.offline")}</span></div>`;
    return;
  }
  if (regenerate) {
    meta.textContent = t("timeline.building");
    try {
      const res = await fetch("/api/advanced", {
        method: "POST", headers: { "Content-Type": "text/plain" },
        body: JSON.stringify({ tool: "timeline" })
      });
      const data = await res.json();
      if (!data.ok) { meta.textContent = tf("timeline.fail", { err: data.error || "" }); return; }
    } catch { meta.textContent = t("timeline.failOffline"); return; }
  }
  try {
    const res = await fetch("/case/db/timeline.jsonl");
    if (!res.ok) {
      meta.textContent = t("timeline.none");
      list.innerHTML = "";
      return;
    }
    const text = await res.text();
    const events = text.split("\n").filter(l => l.trim()).map(l => { try { return JSON.parse(l); } catch { return null; } }).filter(Boolean);
    const byPath = new Map(records.map(r => [r.path, r.id]));
    meta.textContent = tf("timeline.count", { n: events.length });
    const cap = 400;
    list.innerHTML = events.slice(0, cap).map(ev => {
      const recId = byPath.get(ev.path);
      return `<div class="timeline-row" data-id="${escapeHtml(recId || "")}">
        <span class="ts">${escapeHtml(isoTime(ev.ts_unix))}</span>
        <span class="src">${escapeHtml(ev.source || "")}</span>
        <span class="desc" title="${escapeHtml(ev.path || "")} ${escapeHtml(ev.detail || "")}">${escapeHtml(ev.kind || "")} — ${escapeHtml((ev.path || "").split(/[\\/]/).pop() || ev.path || "")}</span>
      </div>`;
    }).join("") + (events.length > cap ? `<div class="timeline-row"><span class="desc muted">${tf("timeline.ellipsis", { n: events.length - cap })}</span></div>` : "");
    list.querySelectorAll(".timeline-row[data-id]").forEach(row => {
      row.addEventListener("click", () => {
        if (!row.dataset.id) return;
        state.selectedIds.clear();
        state.selectedIds.add(row.dataset.id);
        state.activeId = row.dataset.id;
        render();
      });
    });
  } catch {
    meta.textContent = t("timeline.unreadable");
  }
}
document.getElementById("btnTimeline").addEventListener("click", () => {
  const panel = document.getElementById("timelinePanel");
  panel.hidden = !panel.hidden;
  if (!panel.hidden) loadTimeline(false);
});
document.getElementById("btnTimelineClose").addEventListener("click", () => {
  document.getElementById("timelinePanel").hidden = true;
});
document.getElementById("btnTimelineGen").addEventListener("click", () => loadTimeline(true));

// The viewer also runs standalone via file:// — server-backed features need
// the workstation server, and marks/tags live in a different localStorage
// origin than the served viewer, so make those limits visible up front.
if (location.protocol === "file:") {
  const btn = document.getElementById("btnExportSelected");
  btn.disabled = true;
  btn.title = t("offline.openServer");
  const apply = document.getElementById("btnApplyMarks");
  apply.disabled = true;
  apply.title = t("offline.applyTitle");
  const cap = document.getElementById("btnCaptureFrame");
  cap.disabled = true;
  cap.title = t("offline.serverOnly");
  const clip = document.getElementById("btnExportClip");
  clip.disabled = true;
  clip.title = t("offline.serverOnly");
  toast(t("toast.standalone"));
}

state.pageSize = Number(els.pageSize.value) || 100;
