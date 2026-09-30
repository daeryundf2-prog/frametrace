async function loadViewerData() {
  if (window.__FRAMETRACE_DATA__ && window.__FRAMETRACE_DATA__.manifest) {
    return window.__FRAMETRACE_DATA__;
  }
  const note = document.createElement("div");
  note.id = "ft-load-note";
  note.style.cssText = "position:fixed;inset:0;display:grid;place-items:center;background:rgba(14,21,19,.92);color:#8fa79e;font:14px/1.5 system-ui,sans-serif;z-index:9999";
  note.textContent = "증거 목록을 불러오는 중…";
  document.body.appendChild(note);
  const done = () => note.remove();
  try {
    const probe = await fetch("/api/records?offset=0&limit=500");
    if (probe.ok) {
      const first = await probe.json();
      if (first && first.ok) {
        const metaResp = await fetch("/api/records-meta");
        const meta = metaResp.ok ? await metaResp.json() : {};
        const videos = Array.isArray(first.videos) ? first.videos.slice() : [];
        const total = first.total || videos.length;
        while (videos.length < total) {
          note.textContent = `증거 목록을 불러오는 중… ${videos.length}/${total}`;
          const r = await (await fetch(`/api/records?offset=${videos.length}&limit=500`)).json();
          if (!r || !r.ok || !Array.isArray(r.videos) || !r.videos.length) break;
          videos.push(...r.videos);
        }
        const scan = (meta && meta.scan) || {};
        scan.videos = videos;
        done();
        return {
          manifest: meta.manifest || {},
          caseDir: meta.caseDir || "",
          scan,
          carveLog: meta.carveLog,
          filesystemLog: meta.filesystemLog,
          validationLog: meta.validationLog,
          anomalyLog: meta.anomalyLog,
          flsEntries: meta.flsEntries,
          thumbs: meta.thumbs,
          annotations: meta.annotations,
          deepfake: meta.deepfake,
          telemetry: meta.telemetry,
          proxies: meta.proxies,
        };
      }
    }
  } catch { /* API unavailable — fall through to the bundle */ }
  await new Promise((resolve) => {
    const s = document.createElement("script");
    s.src = "data-bundle.js";
    s.onload = resolve;
    s.onerror = resolve;
    document.head.appendChild(s);
  });
  done();
  return window.__FRAMETRACE_DATA__ || {};
}
