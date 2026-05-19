use crate::util::json_for_script;

pub fn render_review_html(manifest_json: &str, index_json: &str) -> String {
    let manifest = json_for_script(manifest_json);
    let index = json_for_script(index_json);
    format!(
        r#"<!doctype html>
<html lang="ko">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>FrameTrace Review</title>
  <style>
    :root {{
      color-scheme: light;
      font-family: "Segoe UI", Arial, sans-serif;
      background: #f6f7f9;
      color: #1f2933;
    }}
    body {{
      margin: 0;
      background: #f6f7f9;
    }}
    header {{
      background: #ffffff;
      border-bottom: 1px solid #d9dee7;
      padding: 18px 24px;
      position: sticky;
      top: 0;
      z-index: 10;
    }}
    h1 {{
      font-size: 20px;
      margin: 0 0 6px;
      letter-spacing: 0;
    }}
    .subtle {{
      color: #667085;
      font-size: 13px;
    }}
    main {{
      padding: 20px 24px 32px;
    }}
    .metrics {{
      display: grid;
      gap: 12px;
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
      margin-bottom: 18px;
    }}
    .metric {{
      background: #ffffff;
      border: 1px solid #d9dee7;
      border-radius: 8px;
      padding: 14px;
    }}
    .metric strong {{
      display: block;
      font-size: 22px;
      margin-top: 4px;
    }}
    .toolbar {{
      display: flex;
      gap: 10px;
      align-items: center;
      margin: 16px 0;
      flex-wrap: wrap;
    }}
    input, select {{
      border: 1px solid #c7ceda;
      border-radius: 6px;
      padding: 9px 10px;
      background: #fff;
      font-size: 14px;
    }}
    input {{
      min-width: min(460px, 100%);
      flex: 1;
    }}
    table {{
      width: 100%;
      border-collapse: collapse;
      background: #fff;
      border: 1px solid #d9dee7;
      border-radius: 8px;
      overflow: hidden;
    }}
    th, td {{
      padding: 10px 12px;
      border-bottom: 1px solid #ecf0f4;
      text-align: left;
      vertical-align: top;
      font-size: 13px;
    }}
    th {{
      background: #f1f4f8;
      font-weight: 600;
      color: #344054;
      position: sticky;
      top: 76px;
    }}
    tr:hover td {{
      background: #fafcff;
    }}
    code {{
      font-family: Consolas, "SFMono-Regular", monospace;
      font-size: 12px;
      word-break: break-all;
    }}
    .badge {{
      border-radius: 999px;
      padding: 3px 8px;
      background: #e9eff6;
      color: #344054;
      white-space: nowrap;
      font-size: 12px;
    }}
    .actions a {{
      color: #155eef;
      text-decoration: none;
      margin-right: 8px;
    }}
    .empty {{
      background: #fff;
      border: 1px solid #d9dee7;
      border-radius: 8px;
      padding: 24px;
    }}
    .warnings {{
      background: #fff7ed;
      border: 1px solid #fed7aa;
      border-radius: 8px;
      color: #7c2d12;
      margin: 0 0 16px;
      padding: 12px 14px;
      font-size: 13px;
    }}
  </style>
</head>
<body>
  <header>
    <h1 id="title">FrameTrace Review</h1>
    <div class="subtle" id="subtitle"></div>
  </header>
  <main>
    <section class="metrics" aria-label="scan metrics">
      <div class="metric">Indexed videos<strong id="metric-count">0</strong></div>
      <div class="metric">Total bytes<strong id="metric-bytes">0</strong></div>
      <div class="metric">Likely sources<strong id="metric-sources">0</strong></div>
      <div class="metric">Scan warnings<strong id="metric-warnings">0</strong></div>
      <div class="metric">Hash mode<strong id="metric-hash">-</strong></div>
      <div class="metric">ffprobe<strong id="metric-probe">-</strong></div>
    </section>
    <section id="warnings"></section>
    <section class="toolbar" aria-label="filters">
      <input id="query" type="search" placeholder="Search path, codec, extension, source, parser, confidence">
      <select id="source">
        <option value="">All sources</option>
      </select>
      <select id="confidence">
        <option value="">All confidence</option>
      </select>
    </section>
    <section id="table-wrap"></section>
  </main>
  <script>
    const manifest = {manifest};
    const scan = {index};
    const videos = Array.isArray(scan.videos) ? scan.videos : [];
    const warnings = Array.isArray(scan.warnings) ? scan.warnings : [];

    const fmtBytes = value => {{
      if (!Number.isFinite(value)) return "-";
      const units = ["B", "KB", "MB", "GB", "TB"];
      let current = value;
      let unit = 0;
      while (current >= 1024 && unit < units.length - 1) {{
        current /= 1024;
        unit += 1;
      }}
      return `${{current.toFixed(unit === 0 ? 0 : 1)}} ${{units[unit]}}`;
    }};

    const fmtDuration = value => {{
      if (!Number.isFinite(value)) return "-";
      const seconds = Math.round(value);
      const h = Math.floor(seconds / 3600);
      const m = Math.floor((seconds % 3600) / 60);
      const s = seconds % 60;
      return h ? `${{h}}:${{String(m).padStart(2, "0")}}:${{String(s).padStart(2, "0")}}` : `${{m}}:${{String(s).padStart(2, "0")}}`;
    }};

    const escapeHtml = value => String(value ?? "")
      .replaceAll("&", "&amp;")
      .replaceAll("<", "&lt;")
      .replaceAll(">", "&gt;")
      .replaceAll('"', "&quot;")
      .replaceAll("'", "&#39;");

    const confidenceSelect = document.getElementById("confidence");
    [...new Set(videos.map(v => v.confidence).filter(Boolean))].sort().forEach(value => {{
      const option = document.createElement("option");
      option.value = value;
      option.textContent = value;
      confidenceSelect.appendChild(option);
    }});

    const sourceSelect = document.getElementById("source");
    const sourceNames = [...new Set(videos.map(v => v.source_profile?.vendor).filter(Boolean))].sort();
    sourceNames.forEach(value => {{
      const option = document.createElement("option");
      option.value = value;
      option.textContent = value;
      sourceSelect.appendChild(option);
    }});

    document.getElementById("title").textContent = manifest.title || "FrameTrace Review";
    document.getElementById("subtitle").textContent = `${{manifest.case_id || "case"}} · source: ${{scan.source_path || "-"}}`;
    document.getElementById("metric-count").textContent = scan.video_count ?? videos.length;
    document.getElementById("metric-bytes").textContent = fmtBytes(scan.total_bytes ?? 0);
    document.getElementById("metric-sources").textContent = sourceNames.length;
    document.getElementById("metric-warnings").textContent = warnings.length;
    document.getElementById("metric-hash").textContent = scan.options?.hash_files ? "SHA-256" : "Skipped";
    document.getElementById("metric-probe").textContent = scan.options?.use_ffprobe ? "Enabled" : "Skipped";
    document.getElementById("warnings").innerHTML = warnings.length
      ? `<div class="warnings"><strong>Scan warnings</strong><br>${{warnings.map(escapeHtml).join("<br>")}}</div>`
      : "";

    const render = () => {{
      const query = document.getElementById("query").value.trim().toLowerCase();
      const confidence = confidenceSelect.value;
      const source = sourceSelect.value;
      const filtered = videos.filter(video => {{
        if (confidence && video.confidence !== confidence) return false;
        if (source && video.source_profile?.vendor !== source) return false;
        if (!query) return true;
        return [
          video.relative_path,
          video.source_path,
          video.extension,
          video.video_codec,
          video.audio_codec,
          video.confidence,
          video.source_profile?.lane,
          video.source_profile?.vendor,
          video.source_profile?.parser,
          video.source_profile?.confidence,
          video.sha256
        ].some(value => String(value ?? "").toLowerCase().includes(query));
      }});

      const wrap = document.getElementById("table-wrap");
      if (!filtered.length) {{
        wrap.innerHTML = '<div class="empty">No matching videos.</div>';
        return;
      }}

      wrap.innerHTML = `<table>
        <thead>
          <tr>
            <th>ID</th>
            <th>Path</th>
            <th>Source</th>
            <th>Media</th>
            <th>Size</th>
            <th>Hash</th>
            <th>Review</th>
          </tr>
        </thead>
        <tbody>
          ${{filtered.map(video => `
            <tr>
              <td><span class="badge">${{escapeHtml(video.id)}}</span></td>
              <td><code>${{escapeHtml(video.relative_path || video.source_path)}}</code><br><span class="subtle">${{escapeHtml(video.confidence)}}</span></td>
              <td>${{escapeHtml(video.source_profile?.vendor || "-")}}<br><span class="subtle">${{escapeHtml(video.source_profile?.parser || "-")}} · ${{escapeHtml(video.source_profile?.confidence || "-")}}</span></td>
              <td>${{escapeHtml(video.video_codec || "-")}} / ${{escapeHtml(video.audio_codec || "-")}}<br><span class="subtle">${{escapeHtml(video.width || "-")}}x${{escapeHtml(video.height || "-")}} · ${{fmtDuration(video.duration_seconds)}}</span></td>
              <td>${{fmtBytes(video.size_bytes)}}</td>
              <td><code>${{escapeHtml(video.sha256 || video.hash_status || "-")}}</code></td>
              <td class="actions"><a href="${{escapeHtml(video.file_url)}}" target="_blank" rel="noreferrer">Open</a></td>
            </tr>
          `).join("")}}
        </tbody>
      </table>`;
    }};

    document.getElementById("query").addEventListener("input", render);
    sourceSelect.addEventListener("change", render);
    confidenceSelect.addEventListener("change", render);
    render();
  </script>
</body>
</html>
"#
    )
}
