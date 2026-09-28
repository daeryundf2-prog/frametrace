let mediaRenderedFor = null;

function drawTelemetry(report) {
  const canvas = document.getElementById("telemetryCanvas");
  const note = document.getElementById("telemetryNote");
  const ctx = canvas.getContext("2d");
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  const pts = (report.points || []).filter(p => Number.isFinite(p.lat) && Number.isFinite(p.lon));
  if (pts.length < 2) {
    note.textContent = report.note || t("telemetry.noPoints");
    return;
  }
  // Relative track sketch — no basemap: the viewer stays fully offline.
  const lats = pts.map(p => p.lat), lons = pts.map(p => p.lon);
  const [minLat, maxLat] = [Math.min(...lats), Math.max(...lats)];
  const [minLon, maxLon] = [Math.min(...lons), Math.max(...lons)];
  const pad = 8;
  const s = Math.min((canvas.width - 2 * pad) / Math.max(maxLon - minLon, 1e-9),
                     (canvas.height - 2 * pad) / Math.max(maxLat - minLat, 1e-9));
  const X = p => pad + (p.lon - minLon) * s;
  const Y = p => canvas.height - pad - (p.lat - minLat) * s;
  const stride = Math.max(1, Math.floor(pts.length / 4000));
  ctx.strokeStyle = "#0f7c71";
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  for (let i = 0; i < pts.length; i += stride) {
    i === 0 ? ctx.moveTo(X(pts[i]), Y(pts[i])) : ctx.lineTo(X(pts[i]), Y(pts[i]));
  }
  ctx.stroke();
  ctx.fillStyle = "#2ea043";
  ctx.beginPath(); ctx.arc(X(pts[0]), Y(pts[0]), 3.5, 0, 7); ctx.fill();
  ctx.fillStyle = "#c0392b";
  const last = pts[pts.length - 1];
  ctx.beginPath(); ctx.arc(X(last), Y(last), 3.5, 0, 7); ctx.fill();
  const speed = Number.isFinite(report.max_speed_kmh) ? ` · ${tf("telemetry.maxSpeed", { v: report.max_speed_kmh.toFixed(0) })}` : "";
  const camm = report.camm ? ` · ${t("telemetry.cammCaveat")}` : "";
  note.textContent = tf("telemetry.summary", { n: pts.length }) + speed + camm;
}

function renderDetails() {
  const record = selectedRecord();
  const teleTitle = document.getElementById("telemetryTitle");
  const telePane = document.getElementById("telemetryPane");
  if (!record) {
    els.mediaStage.innerHTML = `<div class="fallback">${t("empty.index")}</div>`;
    els.mediaTitle.textContent = "-";
    els.mediaStatus.textContent = "-";
    els.summaryList.innerHTML = "";
    els.metaList.innerHTML = "";
    els.validationList.innerHTML = "";
    els.detailBadges.innerHTML = "";
    if (teleTitle) teleTitle.hidden = true;
    if (telePane) telePane.hidden = true;
    mediaRenderedFor = null;
    return;
  }
  els.mediaTitle.textContent = record.originalName || record.name || record.id;
  els.mediaStatus.textContent = record.status;
  els.mediaStatus.className = `badge ${statusClass(record.status)}`;
  const dualMates = state.layout.dual ? matesOf(record) : [];
  const mediaKey = `${record.id}:${record.fileUrl}:${proxyPathFor(record)}:dual:${state.layout.dual ? dualMates.map(m => m.id).join(",") : "off"}`;
  if (mediaRenderedFor !== mediaKey) {
    const mediaSrc = mediaSrcFor(record);
    const why = needsProxy(record);
    const online = location.protocol === "http:" || location.protocol === "https:";
    if (mediaSrc && dualMates.length) {
      els.mediaStage.innerHTML = `<div class="dual-stage">${[record, ...dualMates].map(r =>
        `<div class="dual-pane"><video controls preload="metadata" src="${escapeHtml(mediaSrcFor(r))}"></video>` +
        `<span class="dual-label">${escapeHtml(`${channelLabel(channelCodeOf(r))} · ${r.originalName || r.name || r.id}`)}</span></div>`
      ).join("")}</div>`;
      wireDualSync(els.mediaStage.querySelectorAll("video"));
    } else if (mediaSrc) {
      els.mediaStage.innerHTML = `<video controls preload="metadata" src="${escapeHtml(mediaSrc)}"></video>`;
    } else if (why && online) {
      // Unplayable container + workstation present: build the proxy once per
      // record automatically instead of leaving a dead <video> element.
      const auto = proxyAutoState.get(record.id) || "";
      if (!auto) autoRequestProxy(record);
      const href = record.fileUrl || "";
      els.mediaStage.innerHTML = `<div class="fallback">` +
        (auto.startsWith("failed:")
          ? escapeHtml(tf("media.proxyAutoFail", { err: auto.slice(7), why }))
          : escapeHtml(tf("media.proxyAuto", { why }))) +
        (href ? ` <a href="${escapeHtml(href)}" target="_blank">${escapeHtml(t("media.openOriginal"))}</a>` : "") +
        `</div>`;
    } else if (why) {
      // Standalone bundle with no proxy yet — offer the original for
      // download/open rather than a player that can never decode it.
      const href = record.fileUrl || (record.proxyPath ? fileUrl(record.proxyPath) : "");
      els.mediaStage.innerHTML = `<div class="fallback">${escapeHtml(tf("media.unplayable", { why }))}` +
        (href ? ` <a href="${escapeHtml(href)}" target="_blank">${escapeHtml(t("media.openOriginal"))}</a>` : "") +
        `</div>`;
    } else {
      els.mediaStage.innerHTML = `<div class="fallback">${t("empty.play")}</div>`;
    }
    els.mediaStage.querySelectorAll("video").forEach(v => {
      v.playbackRate = state.layout.rate || 1;
      v.addEventListener("loadedmetadata", applyVideoScale);
    });
    mediaRenderedFor = mediaKey;
  }
  applyVideoScale();
  updateRangeLabel();
  const mark = markOf(record);
  const recordTags = tagListFor(record);
  if (teleTitle && telePane) {
    teleTitle.hidden = !mediaSrcFor(record);
    const tele = record.telemetry;
    if (tele && Array.isArray(tele.points) && tele.points.length) {
      telePane.hidden = false;
      drawTelemetry(tele);
    } else {
      telePane.hidden = true;
    }
  }
  document.getElementById("detailBadges").innerHTML = [
    `<span class="badge ${statusClass(record.status)}">${escapeHtml(statusLabel(record.status))}</span>`,
    (record.warnings || []).length ? `<span class="badge warn" title="${escapeHtml(record.warnings.join("\n"))}">${tf("warn.count", { n: record.warnings.length })}</span>` : "",
    record.hasAnomaly ? `<span class="badge anomaly">${escapeHtml(t("header.anomaly"))}</span>` : "",
    record.dfl && record.dfl.band ? `<span class="badge dfl dfl-${escapeHtml(record.dfl.band)}">${escapeHtml(tf("dfl.badge", { band: record.dfl.band_label || record.dfl.band }))}</span>` : "",
    mark ? `<span class="mark-chip ${escapeHtml(mark.status)}">${escapeHtml(markLabel(mark.status))}</span>` : "",
    ...recordTags.map(tag => `<span class="tag-chip">${escapeHtml(tag)}</span>`),
    record.indexStatus === "stale" ? '<span class="muted">stale</span>' : ""
  ].join(" ");
  els.summaryList.innerHTML = [
    [t("detail.recTime"), record.recTime ? fmtUnix(record.recTime) + (record.recSource === "name" ? "" : " " + t("detail.est")) : t("detail.unknown")],
    [t("detail.channel"), record.channel || "-"],
    [t("detail.mark"), mark ? markLabel(mark.status) : t("detail.unmarked")],
    [t("detail.len"), fmtDuration(record.duration)],
    [t("detail.size"), fmtBytes(record.size)],
    [t("detail.anomaly"), record.hasAnomaly ? record.anomalies.map(item => item.kind).join(", ") : "-"],
    [t("detail.dfl"), record.dfl ? (record.dfl.band
        ? tf("detail.dflScore", { label: record.dfl.band_label || record.dfl.band, score: record.dfl.score ?? "?" }) + (record.dfl.signal_titles?.length ? ": " + record.dfl.signal_titles.slice(0, 3).join(", ") : "")
        : (record.dfl.error ? tf("detail.dflFail", { error: record.dfl.error }) : t("detail.dflNone"))) : "-"]
  ].map(([k, v]) => `<dt>${escapeHtml(k)}</dt><dd>${escapeHtml(v)}</dd>`).join("");
  els.metaList.innerHTML = [
    [t("detail.original"), `<code>${escapeHtml(record.originalName || record.name)}</code>`],
    [t("detail.origPath"), record.originalPath ? `<code>${escapeHtml(record.originalPath)}</code>` : "-"],
    [t("detail.path"), `<code>${escapeHtml(record.path)}</code>`],
    [t("detail.codec"), escapeHtml(record.codec)],
    ["SHA-256", `<code>${escapeHtml(record.sha256)}</code>`],
    [t("detail.offset"), record.offset != null ? String(record.offset) : "-"]
  ].map(([k, v]) => `<dt>${escapeHtml(k)}</dt><dd>${v}</dd>`).join("");
  // tag editor for the selected evidence
  const tagEditorHtml = `<div class="tag-editor">
    ${tagPresets().map(preset => `<button type="button" class="tag-btn ${recordTags.includes(preset) ? "on" : ""}" data-preset-tag="${escapeHtml(preset)}">${escapeHtml(preset)}</button>`).join("")}
    <input type="text" class="tag-input" id="customTagInput" placeholder="${t("tag.input")}" style="width:80px;height:24px;font-size:11px;">
    <button type="button" class="mini" id="btnAddCustomTag">${t("tag.apply")}</button>
    <button type="button" class="mini" id="btnSaveCustomTag" title="${t("tag.presetSave.title")}">${t("tag.presetSave")}</button>
  </div>`;
  // replace existing tag editor if any
  const oldEditor = document.querySelector(".tag-editor-wrap");
  if (oldEditor) oldEditor.remove();
  const metaEl = els.metaList;
  metaEl.insertAdjacentHTML("afterend", `<div class="tag-editor-wrap">${tagEditorHtml}
    <textarea id="evidenceNote" class="note-input" rows="2" placeholder="${t("note.placeholder")}">${escapeHtml(state.notes[record.id] || "")}</textarea>
  </div>`);
  document.getElementById("evidenceNote").addEventListener("input", e => {
    const value = e.target.value;
    if (value.trim()) state.notes[record.id] = value;
    else state.notes[record.id] = "";
    touchAnnotation(record.id);
    storageSet(NOTES_KEY, state.notes);
  });
  document.querySelectorAll(".tag-btn").forEach(btn => {
    btn.addEventListener("click", () => {
      toggleTag(record.id, btn.dataset.presetTag);
      render();
    });
  });
  const customInput = document.getElementById("customTagInput");
  const customBtn = document.getElementById("btnAddCustomTag");
  if (customInput && customBtn) {
    const addCustom = () => {
      const tag = customInput.value.trim();
      if (!tag) return;
      addTag(record.id, tag);
      customInput.value = "";
      render();
      document.getElementById("customTagInput")?.focus();
    };
    customBtn.addEventListener("click", addCustom);
    customInput.addEventListener("keydown", e => { if (e.key === "Enter") addCustom(); });
    const savePresetBtn = document.getElementById("btnSaveCustomTag");
    if (savePresetBtn) {
      savePresetBtn.addEventListener("click", () => {
        const tag = customInput.value.trim();
        if (!tag) return;
        addTagPreset(tag);
        customInput.value = "";
        render();
        toast(tf("toast.presetAdded", { tag }));
      });
    }
  }
  const related = validationLog.filter(item => normalizePath(item.target_path) === normalizePath(record.path) || item.selector === record.id);
  const anomalyRelated = (record.anomalies || []).map(item => `<div class="validation-item">
    <strong>${escapeHtml(item.kind || "anomaly")}</strong>
    <div class="muted">${escapeHtml(item.detail || "")}</div>
    <code>candidate-finding</code>
  </div>`);
  const warningItems = (record.warnings || []).map(warning => `<div class="validation-item warn">
    <strong>${t("warn.title")}</strong>
    <div class="muted">${escapeHtml(warning)}</div>
    <code>recovery-warning</code>
  </div>`);
  els.validationList.innerHTML = [
    ...warningItems,
    ...related.map(item => `<div class="validation-item">
    <strong>${escapeHtml(item.validation_status || "-")}</strong>
    <div class="muted">${escapeHtml(item.validation_note || item.ffprobe_error || "-")}</div>
    <code>${escapeHtml(item.target_sha256 || "-")}</code>
  </div>`),
    ...anomalyRelated
  ].join("") || `<div class="validation-item">${t("validation.none")}</div>`;
}

function renderTree() {
  const typeCounts = new Map();
  const kindCounts = new Map();
  const dayCounts = new Map();
  records.forEach(record => {
    const typeKey = record.recType || "unclassified";
    typeCounts.set(typeKey, (typeCounts.get(typeKey) || 0) + 1);
    kindCounts.set(record.kind, (kindCounts.get(record.kind) || 0) + 1);
    if (record.recDay) dayCounts.set(record.recDay, (dayCounts.get(record.recDay) || 0) + 1);
  });
  const kindLabels = { video: t("filter.kind.video"), carved: t("filter.kind.carved"), filesystem: t("filter.kind.filesystem"), candidate: t("filter.kind.candidate") };
  const items = [];
  const addItems = (title, entries, activeKey, onPick) => {
    items.push({ header: title });
    items.push({ label: t("tree.all"), count: null, key: "", pick: () => onPick(""), active: activeKey === "" });
    [...entries.entries()].sort((a, b) => (a[0] < b[0] ? -1 : 1)).forEach(([key, count]) => {
      items.push({ label: key, count, key, pick: () => onPick(key), active: key === activeKey });
    });
  };
  addItems(t("tree.rectype"), new Map([...typeCounts.entries()].map(([k, c]) => [recTypeLabel(k), c])),
    state.recType ? recTypeLabel(state.recType) : "", label => {
      const match = [...typeCounts.entries()].find(([k]) => recTypeLabel(k) === label);
      state.recType = match ? match[0] : "";
      state.currentPage = 1;
      render();
    });
  addItems(t("tree.kind"), new Map([...kindCounts.entries()].map(([k, c]) => [kindLabels[k] || k, c])),
    kindLabels[state.kind] || "", label => {
      state.kind = Object.entries(kindLabels).find(([, kindLabel]) => kindLabel === label)?.[0] || "";
      els.kind.value = state.kind;
      state.currentPage = 1;
      render();
    });
  addItems(t("tree.date"), dayCounts, state.dateFrom && state.dateFrom === state.dateTo ? state.dateFrom : "", day => {
    state.dateFrom = day;
    state.dateTo = day;
    els.dateFrom.value = day;
    els.dateTo.value = day;
    state.currentPage = 1;
    render();
  });
  els.facetTree.innerHTML = items.map(item => item.header !== undefined
    ? `<div class="tree-sec">${escapeHtml(item.header)}</div>`
    : `<div class="tree-item${item.active ? " active" : ""}" data-pick="${escapeHtml(item.label)}" tabindex="0" role="button"><span>${escapeHtml(item.label)}</span>${item.count != null ? `<span class="muted">${item.count}</span>` : ""}</div>`
  ).join("");
  const pickers = items.filter(item => item.pick);
  const nodes = els.facetTree.querySelectorAll(".tree-item");
  let idx = 0;
  nodes.forEach(node => {
    const entry = pickers[idx];
    if (!entry) return;
    idx += 1;
    node.addEventListener("click", () => entry.pick(node.dataset.pick));
    node.addEventListener("keydown", e => {
      if (e.key === "Enter" || e.key === " ") { e.preventDefault(); entry.pick(node.dataset.pick); }
    });
  });
}

function renderMetrics() {
  els.caseLine.textContent = `${manifest.case_id || "case"} · ${manifest.title || "Untitled"} · ${scan.source_path || "-"}`;
  els.metricVideos.textContent = tf("metric.indexed", { n: videos.length });
  els.metricCarved.textContent = carveLog.length + recoveredFilesystemLog.length;
  els.metricVerified.textContent = records.filter(record => record.status === "ffprobe-video-stream-confirmed" || record.status === "ffprobe-confirmed").length;
  els.metricFailed.textContent = records.filter(record => record.status === "validation-failed").length;
  if (els.metricAnomaly) {
    els.metricAnomaly.textContent = String(records.filter(record => record.hasAnomaly).length);
  }
  const markCount = Object.keys(state.marks).length;
  els.selectionCount.textContent = tf("sel.count", { n: state.selectedIds.size, m: markCount });
  if (els.caseWarnings) {
    if (inspectionWarnings.length) {
      els.caseWarnings.hidden = false;
      els.caseWarnings.innerHTML = `<b>${tf("warn.inspect", { n: inspectionWarnings.length })}</b>${inspectionWarnings.map(escapeHtml).join(" · ")}`;
    } else {
      els.caseWarnings.hidden = true;
    }
  }
}

function renderChips() {
  // Status/mark chips stay inline; tag filters live in the 정렬·표시
  // panel's tag select so the chip row stays a single group.
  const parts = [`<span class="chip-label">${t("filter.label")}</span>`];
  for (const [value, label] of PRESET_CHIPS) {
    if (value.startsWith("tag:")) continue;
    parts.push(`<button type="button" class="chip ${state.chip === value ? "active" : ""}" data-chip="${escapeHtml(value)}">${escapeHtml(t(label))}</button>`);
  }
  els.presetChips.innerHTML = parts.join("");
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
  // Tag filter options follow the editable preset list plus any tags
  // already applied in this case that aren't presets (custom tags).
  const appliedTags = new Set();
  Object.values(state.tags).forEach(list => (list || []).forEach(tag => appliedTags.add(tag)));
  const filterTags = [...state.tagPresets, ...[...appliedTags].filter(tag => !state.tagPresets.includes(tag))];
  els.tagFilter.innerHTML = `<option value="">${t("filter.tag.all")}</option>`
    + filterTags.map(tag => `<option value="tag:${escapeHtml(tag)}">${escapeHtml(tf("filter.tag.named", { tag }))}</option>`).join("");
  els.tagFilter.value = state.chip.startsWith("tag:") ? state.chip : "";
}

function render() {
  applyChromeI18n();
  // Filter+sort once per render — grid and histogram share the result.
  const filtered = filteredRecords();
  renderMetrics();
  renderChips();
  renderHistogram(filtered);
  renderTree();
  renderGrid(filtered);
  renderDetails();
}
