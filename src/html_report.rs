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
    .table-status {{
      color: #475467;
      font-size: 13px;
      margin: 0 0 8px;
    }}
    .pager {{
      display: flex;
      gap: 8px;
      align-items: center;
      justify-content: flex-end;
      margin-top: 10px;
    }}
    button {{
      border: 1px solid #c7ceda;
      border-radius: 6px;
      background: #ffffff;
      color: #1f2933;
      padding: 7px 10px;
      font-size: 13px;
    }}
    button:disabled {{
      color: #98a2b3;
      background: #f2f4f7;
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
    <div class="table-status" id="result-status"></div>
    <section id="table-wrap"></section>
    <section class="pager" aria-label="pagination">
      <button id="prev-page" type="button">Previous</button>
      <span class="subtle" id="page-status"></span>
      <button id="next-page" type="button">Next</button>
    </section>
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
    const warningSeverity = warning => /failed|unreadable|skipped/i.test(warning) ? "주의" : "정보";
    document.getElementById("warnings").innerHTML = warnings.length
      ? `<div class="warnings"><strong>Scan warnings</strong><table><thead><tr><th>Severity</th><th>Message</th><th>Status</th></tr></thead><tbody>${{warnings.map(warning => `<tr><td>${{warningSeverity(warning)}}</td><td>${{escapeHtml(warning)}}</td><td>Review required</td></tr>`).join("")}}</tbody></table></div>`
      : "";

    let currentPage = 1;
    const pageSize = 100;

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

      const pageCount = Math.max(1, Math.ceil(filtered.length / pageSize));
      currentPage = Math.min(currentPage, pageCount);
      const start = (currentPage - 1) * pageSize;
      const pageRows = filtered.slice(start, start + pageSize);
      document.getElementById("result-status").textContent = `${{filtered.length}} matching videos · showing ${{filtered.length ? start + 1 : 0}}-${{Math.min(start + pageSize, filtered.length)}}`;
      document.getElementById("page-status").textContent = `Page ${{currentPage}} / ${{pageCount}}`;
      document.getElementById("prev-page").disabled = currentPage <= 1;
      document.getElementById("next-page").disabled = currentPage >= pageCount;

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
          ${{pageRows.map(video => `
            <tr>
              <td><span class="badge">${{escapeHtml(video.id)}}</span></td>
              <td><code>${{escapeHtml(video.relative_path || video.source_path)}}</code><br><span class="subtle">${{escapeHtml(video.confidence)}}</span></td>
              <td>${{escapeHtml(video.source_profile?.vendor || "-")}}<br><span class="subtle">${{escapeHtml(video.source_profile?.parser || "-")}} · ${{escapeHtml(video.source_profile?.confidence || "-")}}</span></td>
              <td>${{escapeHtml(video.video_codec || "-")}} / ${{escapeHtml(video.audio_codec || "-")}}<br><span class="subtle">${{escapeHtml(video.width || "-")}}x${{escapeHtml(video.height || "-")}} · ${{fmtDuration(video.duration_seconds)}}</span></td>
              <td>${{fmtBytes(video.size_bytes)}}</td>
              <td><code>${{escapeHtml(video.sha256 || video.hash_status || "-")}}</code></td>
              <td class="actions"><a href="${{escapeHtml(video.file_url)}}" target="_blank" rel="noreferrer">Source</a></td>
            </tr>
          `).join("")}}
        </tbody>
      </table>`;
    }};

    document.getElementById("query").addEventListener("input", () => {{ currentPage = 1; render(); }});
    sourceSelect.addEventListener("change", () => {{ currentPage = 1; render(); }});
    confidenceSelect.addEventListener("change", () => {{ currentPage = 1; render(); }});
    document.getElementById("prev-page").addEventListener("click", () => {{ currentPage -= 1; render(); }});
    document.getElementById("next-page").addEventListener("click", () => {{ currentPage += 1; render(); }});
    render();
  </script>
</body>
</html>
"#
    )
}

pub fn render_evidence_viewer_html(
    manifest_json: &str,
    index_json: &str,
    carve_log_jsonl: &str,
    validation_log_jsonl: &str,
) -> String {
    let manifest = json_for_script(manifest_json);
    let index = json_for_script(index_json);
    let carve_lines = json_for_script(&jsonl_to_array(carve_log_jsonl));
    let validation_lines = json_for_script(&jsonl_to_array(validation_log_jsonl));
    format!(
        r#"<!doctype html>
<html lang="ko">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>FrameTrace Evidence Viewer</title>
  <style>
    :root {{
      --bg: #eef1f0;
      --panel: #fbfcfb;
      --ink: #1f2724;
      --muted: #68736f;
      --line: #d8dedb;
      --accent: #0f7c71;
      --danger: #b14d42;
      --warn: #b4802a;
      --ok: #2f7a48;
      --candidate: #6c5d99;
      --mono: "SFMono-Regular", Consolas, "Liberation Mono", monospace;
      --sans: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }}
    * {{ box-sizing: border-box; }}
    body {{ margin: 0; min-width: 1180px; background: var(--bg); color: var(--ink); font-family: var(--sans); letter-spacing: 0; }}
    header {{ height: 62px; display: grid; grid-template-columns: 240px 1fr auto; align-items: center; gap: 16px; padding: 0 18px; background: #fff; border-bottom: 1px solid var(--line); }}
    h1 {{ margin: 0; font-size: 18px; }}
    .case-line {{ color: var(--muted); font-size: 13px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }}
    .shell {{ display: grid; grid-template-columns: 320px minmax(0, 1fr) 300px; gap: 10px; padding: 10px; min-height: calc(100vh - 62px); }}
    aside, section {{ min-height: 0; }}
    .panel {{ background: var(--panel); border: 1px solid var(--line); border-radius: 8px; overflow: hidden; }}
    .panel-title {{ height: 36px; display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 0 12px; border-bottom: 1px solid var(--line); font-size: 12px; font-weight: 800; text-transform: uppercase; color: #39433f; }}
    .metrics {{ display: grid; grid-template-columns: repeat(2, 1fr); gap: 8px; padding: 10px; }}
    .metric {{ border: 1px solid var(--line); border-radius: 7px; padding: 9px; background: #fff; color: var(--muted); font-size: 12px; }}
    .metric strong {{ display: block; margin-top: 2px; color: var(--ink); font-size: 18px; }}
    .filters {{ display: grid; gap: 8px; padding: 10px; }}
    input, select, button {{ font: inherit; }}
    input, select {{ height: 34px; border: 1px solid #bdc9c4; border-radius: 6px; padding: 0 10px; background: #fff; color: var(--ink); }}
    button {{ height: 32px; border: 1px solid #bdc9c4; border-radius: 6px; background: #fff; color: var(--ink); cursor: pointer; }}
    button:disabled {{ color: #98a2b3; background: #f2f4f7; }}
    .list {{ overflow: auto; max-height: calc(100vh - 278px); }}
    .row {{ display: grid; grid-template-columns: 74px 1fr 66px; gap: 8px; padding: 9px 10px; border-bottom: 1px solid #e5ebe8; cursor: pointer; }}
    .row.active {{ background: #edf8f5; box-shadow: inset 3px 0 0 var(--accent); }}
    .row strong, .row code {{ display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }}
    code {{ font-family: var(--mono); font-size: 11px; word-break: break-all; }}
    .muted {{ color: var(--muted); font-size: 12px; }}
    .badge {{ display: inline-flex; align-items: center; max-width: 100%; min-height: 22px; border-radius: 999px; padding: 2px 8px; background: #e9eff6; color: #344054; font-size: 11px; font-weight: 800; white-space: nowrap; }}
    .badge.candidate {{ background: #eeeafd; color: var(--candidate); }}
    .badge.failed {{ background: #fff1ef; color: var(--danger); }}
    .badge.ok {{ background: #edf8f1; color: var(--ok); }}
    .viewer {{ display: grid; grid-template-rows: minmax(420px, 1fr) 150px; gap: 10px; }}
    .media {{ display: grid; grid-template-rows: 38px minmax(0, 1fr); background: #101815; border-radius: 8px; overflow: hidden; border: 1px solid #0c1713; }}
    .media-title {{ display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 0 12px; color: #eaf3ef; background: #16231f; }}
    video {{ width: 100%; height: 100%; background: #0b0f0d; object-fit: contain; }}
    .fallback {{ display: grid; place-items: center; color: #c8d5d0; min-height: 360px; padding: 28px; text-align: center; }}
    .inspector {{ display: grid; grid-template-rows: auto auto minmax(0, 1fr); gap: 10px; }}
    dl {{ display: grid; grid-template-columns: 94px minmax(0, 1fr); gap: 7px 10px; padding: 10px; margin: 0; }}
    dt {{ color: var(--muted); font-size: 12px; }}
    dd {{ margin: 0; min-width: 0; font-size: 12px; }}
    .validation-list {{ padding: 10px; display: grid; gap: 8px; overflow: auto; max-height: 260px; }}
    .validation-item {{ border: 1px solid var(--line); border-radius: 7px; padding: 8px; background: #fff; }}
    .pager {{ display: flex; align-items: center; justify-content: flex-end; gap: 8px; padding: 9px 10px; border-top: 1px solid var(--line); }}
  </style>
</head>
<body>
  <header>
    <h1>FrameTrace Evidence Viewer</h1>
    <div class="case-line" id="caseLine"></div>
    <div class="muted">실제 케이스 데이터</div>
  </header>
  <main class="shell">
    <aside class="panel">
      <div class="panel-title">증거 목록 <span id="resultCount"></span></div>
      <div class="metrics">
        <div class="metric">영상<strong id="metricVideos">0</strong></div>
        <div class="metric">복구 후보<strong id="metricCarved">0</strong></div>
        <div class="metric">검증됨<strong id="metricVerified">0</strong></div>
        <div class="metric">검증 실패<strong id="metricFailed">0</strong></div>
      </div>
      <div class="filters">
        <input id="query" type="search" placeholder="경로, ID, 파서, 해시 검색">
        <select id="kind">
          <option value="">전체 유형</option>
          <option value="video">색인 영상</option>
          <option value="carved">carving 후보</option>
        </select>
        <select id="status">
          <option value="">전체 검증 상태</option>
          <option value="verified-playable">verified-playable</option>
          <option value="validation-failed">validation-failed</option>
          <option value="candidate-unvalidated">candidate-unvalidated</option>
          <option value="duplicate-candidate">duplicate-candidate</option>
        </select>
      </div>
      <div class="list" id="recordList"></div>
      <div class="pager">
        <button id="prevPage">이전</button>
        <span class="muted" id="pageStatus"></span>
        <button id="nextPage">다음</button>
      </div>
    </aside>
    <section class="viewer">
      <div class="media">
        <div class="media-title"><strong id="mediaTitle">-</strong><span id="mediaStatus" class="badge">-</span></div>
        <div id="mediaStage"></div>
      </div>
      <div class="panel">
        <div class="panel-title">선택 증거 메모</div>
        <dl id="summaryList"></dl>
      </div>
    </section>
    <aside class="inspector">
      <section class="panel">
        <div class="panel-title">포렌식 인스펙터</div>
        <dl id="metaList"></dl>
      </section>
      <section class="panel">
        <div class="panel-title">검증 로그</div>
        <div class="validation-list" id="validationList"></div>
      </section>
      <section class="panel">
        <div class="panel-title">운영 원칙</div>
        <dl>
          <dt>원본</dt><dd>원본 경로는 수정하지 않음</dd>
          <dt>복구물</dt><dd>검증 전까지 후보 상태 유지</dd>
          <dt>보고서</dt><dd>검증 로그와 SHA-256 기준으로 판단</dd>
        </dl>
      </section>
    </aside>
  </main>
<script>
const manifest = {manifest};
const scan = {index};
const carveLog = {carve_lines};
const validationLog = {validation_lines};
const videos = Array.isArray(scan.videos) ? scan.videos : [];
const validationsByPath = new Map(validationLog.map(item => [normalizePath(item.target_path), item]));
const records = [
  ...videos.map(video => {{
    const validation = validationsByPath.get(normalizePath(video.source_path));
    return {{
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
      validation
    }};
  }}),
  ...carveLog.map(item => {{
    const validation = validationsByPath.get(normalizePath(item.output_path));
    return {{
      id: item.id || item.output_path,
      kind: "carved",
      name: item.output_path ? item.output_path.split(/[\\\\/]/).pop() : item.id,
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
      validation
    }};
  }})
];
let activeId = records[0]?.id || null;
let currentPage = 1;
const pageSize = 100;

const els = {{
  caseLine: document.getElementById("caseLine"),
  resultCount: document.getElementById("resultCount"),
  metricVideos: document.getElementById("metricVideos"),
  metricCarved: document.getElementById("metricCarved"),
  metricVerified: document.getElementById("metricVerified"),
  metricFailed: document.getElementById("metricFailed"),
  query: document.getElementById("query"),
  kind: document.getElementById("kind"),
  status: document.getElementById("status"),
  recordList: document.getElementById("recordList"),
  prevPage: document.getElementById("prevPage"),
  nextPage: document.getElementById("nextPage"),
  pageStatus: document.getElementById("pageStatus"),
  mediaTitle: document.getElementById("mediaTitle"),
  mediaStatus: document.getElementById("mediaStatus"),
  mediaStage: document.getElementById("mediaStage"),
  summaryList: document.getElementById("summaryList"),
  metaList: document.getElementById("metaList"),
  validationList: document.getElementById("validationList")
}};

function normalizePath(value) {{ return String(value || "").replaceAll("\\\\", "/").toLowerCase(); }}
function fileUrl(path) {{
  if (!path) return "";
  if (String(path).startsWith("file:")) return path;
  const normalized = String(path).replaceAll("\\\\", "/");
  if (normalized.length > 2 && normalized[1] === ":" && normalized[2] === "/") return "file:///" + encodeURI(normalized);
  if (normalized.startsWith("/")) return "file://" + encodeURI(normalized);
  return encodeURI(normalized);
}}
function escapeHtml(value) {{ return String(value ?? "").replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;").replaceAll("'", "&#39;"); }}
function fmtBytes(value) {{
  if (!Number.isFinite(value)) return "-";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let current = value;
  let unit = 0;
  while (current >= 1024 && unit < units.length - 1) {{ current /= 1024; unit += 1; }}
  return `${{current.toFixed(unit === 0 ? 0 : 1)}} ${{units[unit]}}`;
}}
function fmtDuration(value) {{
  if (!Number.isFinite(value)) return "-";
  const seconds = Math.round(value);
  return `${{Math.floor(seconds / 60)}}:${{String(seconds % 60).padStart(2, "0")}}`;
}}
function statusClass(status) {{
  if (status === "verified-playable" || status === "ffprobe-confirmed") return "ok";
  if (status === "validation-failed") return "failed";
  return "candidate";
}}
function filteredRecords() {{
  const query = els.query.value.trim().toLowerCase();
  return records.filter(record => {{
    if (els.kind.value && record.kind !== els.kind.value) return false;
    if (els.status.value && record.status !== els.status.value) return false;
    if (!query) return true;
    return [record.id, record.name, record.path, record.parser, record.vendor, record.sha256, record.note].some(value => String(value ?? "").toLowerCase().includes(query));
  }});
}}
function selectedRecord() {{ return records.find(record => record.id === activeId) || records[0]; }}
function renderList() {{
  const filtered = filteredRecords();
  if (!filtered.some(record => record.id === activeId)) activeId = filtered[0]?.id || records[0]?.id || null;
  const pageCount = Math.max(1, Math.ceil(filtered.length / pageSize));
  currentPage = Math.min(currentPage, pageCount);
  const start = (currentPage - 1) * pageSize;
  const pageRows = filtered.slice(start, start + pageSize);
  els.resultCount.textContent = `${{filtered.length}}`;
  els.pageStatus.textContent = `${{currentPage}} / ${{pageCount}}`;
  els.prevPage.disabled = currentPage <= 1;
  els.nextPage.disabled = currentPage >= pageCount;
  els.recordList.innerHTML = pageRows.map(record => `<div class="row ${{record.id === activeId ? "active" : ""}}" data-id="${{escapeHtml(record.id)}}">
    <span class="badge ${{statusClass(record.status)}}">${{escapeHtml(record.status)}}</span>
    <span><strong>${{escapeHtml(record.name)}}</strong><code>${{escapeHtml(record.path)}}</code><span class="muted">${{escapeHtml(record.vendor)}} · ${{escapeHtml(record.parser)}}</span></span>
    <span class="muted">${{escapeHtml(record.kind)}}</span>
  </div>`).join("") || `<div class="fallback">일치하는 증거가 없습니다.</div>`;
  els.recordList.querySelectorAll(".row").forEach(row => row.addEventListener("click", () => {{ activeId = row.dataset.id; render(); }}));
}}
function renderDetails() {{
  const record = selectedRecord();
  if (!record) {{
    els.mediaStage.innerHTML = `<div class="fallback">색인된 증거가 없습니다.</div>`;
    return;
  }}
  els.mediaTitle.textContent = record.name || record.id;
  els.mediaStatus.textContent = record.status;
  els.mediaStatus.className = `badge ${{statusClass(record.status)}}`;
  els.mediaStage.innerHTML = record.fileUrl
    ? `<video controls preload="metadata" src="${{escapeHtml(record.fileUrl)}}"></video>`
    : `<div class="fallback">직접 재생 가능한 파일 URL이 없습니다.</div>`;
  els.summaryList.innerHTML = [
    ["ID", record.id],
    ["상태", record.status],
    ["메모", record.note],
    ["길이", fmtDuration(record.duration)]
  ].map(([k, v]) => `<dt>${{escapeHtml(k)}}</dt><dd>${{escapeHtml(v)}}</dd>`).join("");
  els.metaList.innerHTML = [
    ["경로", `<code>${{escapeHtml(record.path)}}</code>`],
    ["제조사", record.vendor],
    ["파서", `<code>${{escapeHtml(record.parser)}}</code>`],
    ["코덱", record.codec],
    ["크기", fmtBytes(record.size)],
    ["SHA-256", `<code>${{escapeHtml(record.sha256)}}</code>`],
    ["오프셋", record.offset ?? "-"]
  ].map(([k, v]) => `<dt>${{escapeHtml(k)}}</dt><dd>${{v}}</dd>`).join("");
  const related = validationLog.filter(item => normalizePath(item.target_path) === normalizePath(record.path) || item.selector === record.id);
  els.validationList.innerHTML = related.length ? related.map(item => `<div class="validation-item">
    <strong>${{escapeHtml(item.validation_status || "-")}}</strong>
    <div class="muted">${{escapeHtml(item.validation_note || item.ffprobe_error || "-")}}</div>
    <code>${{escapeHtml(item.target_sha256 || "-")}}</code>
  </div>`).join("") : `<div class="validation-item">검증 로그 없음</div>`;
}}
function renderMetrics() {{
  els.caseLine.textContent = `${{manifest.case_id || "case"}} · ${{manifest.title || "Untitled"}} · ${{scan.source_path || "-"}}`;
  els.metricVideos.textContent = videos.length;
  els.metricCarved.textContent = carveLog.length;
  els.metricVerified.textContent = records.filter(record => record.status === "verified-playable" || record.status === "ffprobe-confirmed").length;
  els.metricFailed.textContent = records.filter(record => record.status === "validation-failed").length;
}}
function render() {{ renderMetrics(); renderList(); renderDetails(); }}
els.query.addEventListener("input", () => {{ currentPage = 1; render(); }});
els.kind.addEventListener("change", () => {{ currentPage = 1; render(); }});
els.status.addEventListener("change", () => {{ currentPage = 1; render(); }});
els.prevPage.addEventListener("click", () => {{ currentPage -= 1; render(); }});
els.nextPage.addEventListener("click", () => {{ currentPage += 1; render(); }});
render();
</script>
</body>
</html>
"#
    )
}

fn jsonl_to_array(jsonl: &str) -> String {
    let items: Vec<&str> = jsonl
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    format!("[{}]", items.join(","))
}
