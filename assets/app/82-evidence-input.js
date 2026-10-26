// Evidence input conveniences on the start screen:
//  * pasted paths lose wrapping quotes / file:// prefixes (Explorer's
//    "경로로 복사" yields "C:\x\disk.E01" with quotes);
//  * under WSL a Windows path (C:\...) is rewritten to /mnt/c/...;
//  * drag & drop onto the evidence card. Browsers never reveal a dropped
//    file's full path, so the dropped name (+ size) is looked up in recent
//    and common folders; if not found the picker opens filtered to it;
//  * one-click chips for recently used evidence.

function normalizeTypedPath(raw, wsl) {
  let v = String(raw || "").trim();
  if (/^(["']).*\1$/.test(v)) v = v.slice(1, -1).trim();
  if (/^file:\/\//i.test(v)) {
    // Merge review L3: `file://server/share/x.E01` names a network share.
    // Keep it a `//server/share/...` path — never `server/share/...`
    // (relative), which would slip past the network-path refusals below
    // and on the server. Only an empty host or `localhost` is local.
    const host = (v.match(/^file:\/\/([^/\\]*)/i) || [])[1] || "";
    const local = host === "" || /^localhost$/i.test(host);
    try { v = decodeURIComponent(v.replace(/^file:\/\/(localhost(?=[/\\]|$))?/i, local ? "" : "//")); } catch { v = local ? v : "//" + v.replace(/^file:\/\//i, ""); }
    if (local && /^\/[A-Za-z]:/.test(v)) v = v.slice(1).replace(/\//g, "\\");
  }
  const m = wsl && v.match(/^([A-Za-z]):(?:[\\/](.*))?$/);
  if (m) v = `/mnt/${m[1].toLowerCase()}/${(m[2] || "").replace(/\\/g, "/")}`.replace(/\/+$/, "") || "/";
  return v;
}

function setPathField(id, value, note) {
  const input = $(id);
  input.value = value;
  input.dispatchEvent(new Event("input", { bubbles: true }));
  if (note) announce(note);
}

function onPathPaste(e) {
  const text = (e.clipboardData && e.clipboardData.getData("text")) || "";
  const fixed = normalizeTypedPath(text, PICKER_PLUS.wsl);
  if (!text || fixed === text.trim()) return; // default paste is fine
  e.preventDefault();
  setPathField(e.target.id, fixed, PICKER_PLUS.wsl && fixed.startsWith("/mnt/") ? "Windows 경로를 WSL 경로로 바꿨습니다" : "");
}
function onPathChange(e) {
  const fixed = normalizeTypedPath(e.target.value, PICKER_PLUS.wsl);
  if (fixed !== e.target.value.trim()) setPathField(e.target.id, fixed);
}
for (const id of ["sourcePath", "caseDir"]) {
  $(id).addEventListener("paste", onPathPaste);
  $(id).addEventListener("change", onPathChange);
}

// ---- drag & drop ----
const evCard = $("blkEvidence");
let evDragDepth = 0;
evCard.addEventListener("dragenter", e => { e.preventDefault(); evDragDepth++; evCard.classList.add("dragging"); });
evCard.addEventListener("dragover", e => { e.preventDefault(); if (e.dataTransfer) e.dataTransfer.dropEffect = "copy"; });
evCard.addEventListener("dragleave", () => { if (--evDragDepth <= 0) { evDragDepth = 0; evCard.classList.remove("dragging"); } });
evCard.addEventListener("drop", e => {
  e.preventDefault();
  evDragDepth = 0;
  evCard.classList.remove("dragging");
  handleEvidenceDrop(e.dataTransfer);
});

async function handleEvidenceDrop(dt) {
  if (!dt) return;
  const uri = (dt.getData("text/uri-list") || "").split(/\r?\n/).find(l => l && !l.startsWith("#"));
  const text = uri || dt.getData("text/plain") || "";
  if (text && /^(file:|[A-Za-z]:[\\/]|\/|["'])/.test(text.trim())) {
    const dropped = normalizeTypedPath(text, PICKER_PLUS.wsl);
    // PR #6 merge audit: a drop may come from another page (a file:// link
    // to a share); a network/device path is never taken from a drop —
    // the examiner types it (the server's typed-UNC policy).
    if (/^[\\/]{2}/.test(dropped)) {
      toast("끌어 놓은 네트워크 경로는 받지 않습니다 — 경로를 직접 입력하십시오.", "warn");
      return;
    }
    setPathField("sourcePath", dropped, "끌어 놓은 경로를 넣었습니다");
    return;
  }
  const item = dt.items && dt.items[0];
  const entry = item && typeof item.webkitGetAsEntry === "function" ? item.webkitGetAsEntry() : null;
  const file = dt.files && dt.files[0];
  if (!file) return;
  const isDir = !!(entry && entry.isDirectory);
  $("sourceStatus").className = "pathstatus";
  $("sourceStatus").textContent = `‘${file.name}’ 위치를 찾는 중…`;
  const found = await findDroppedPath(file.name, isDir ? null : file.size);
  if (found) {
    setPathField("sourcePath", found, "끌어 놓은 항목의 경로를 찾았습니다");
    toast(`‘${file.name}’ 경로를 찾았습니다 — 맞는지 확인하십시오: ${found}`, "ok");
    return;
  }
  $("sourceStatus").textContent = "";
  toast(`브라우저 보안상 끌어 놓은 파일의 전체 경로는 알 수 없습니다. ‘${file.name}’이(가) 있는 폴더를 열어 주십시오.`, "warn");
  openPicker("sourcePath", "source");
  $("pickerFilter").value = file.name.toLowerCase();
}

// Looks for `name` (and, for files, the exact byte size) in folders the
// examiner is likely to use. Returns a path only for a single clear match.
async function findDroppedPath(name, size) {
  await loadPlaces();
  const prefs = loadPrefs();
  const dirs = new Set();
  for (const p of prefs.recentSources || []) dirs.add(parentPath(p));
  for (const p of Object.values(prefs.pickerLast || {})) dirs.add(/\.(e|ex|l|s)\d+$/i.test(p) ? parentPath(p) : p);
  for (const pl of PICKER_PLUS.places || []) if (pl.kind !== "drive") dirs.add(pl.path);
  const want = name.toLowerCase();
  const hits = new Set();
  const lists = await Promise.all([...dirs].filter(Boolean).slice(0, 16).map(d =>
    jgetSoft("/api/browse?path=" + encodeURIComponent(d) + "&files=e01")));
  for (const res of lists) {
    for (const ent of (res.ok && res.entries) || []) {
      if (String(ent.name).toLowerCase() !== want) continue;
      if (size != null && (ent.dir || (ent.size != null && ent.size !== size))) continue;
      if (size == null && !ent.dir) continue;
      hits.add(ent.path);
    }
  }
  return hits.size === 1 ? [...hits][0] : "";
}

// ---- recent evidence chips ----
function renderEvRecent() {
  const box = $("evRecent"), list = $("evRecentList");
  if (!box || !list) return;
  const items = (loadPrefs().recentSources || []).slice(0, 4);
  box.hidden = !items.length;
  list.innerHTML = items.map(p => {
    const base = p.split(/[\\/]/).filter(Boolean).pop() || p;
    const img = /\.(e|ex|l|s)\d+$/i.test(p);
    return `<li><button type="button" class="ev-chip" data-path="${esc(p)}" title="${esc(p)}">` +
      `<svg class="ic" aria-hidden="true"><use href="#${img ? "i-file-image" : "i-folder"}"/></svg>${esc(base)}</button></li>`;
  }).join("");
}
$("evRecentList").addEventListener("click", e => {
  const b = e.target.closest(".ev-chip");
  if (b) { setPathField("sourcePath", b.dataset.path); $("sourcePath").focus(); }
});
renderEvRecent();
loadPlaces();
