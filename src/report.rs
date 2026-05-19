use crate::util::json_for_script;

pub fn render_case_report(
    manifest_json: &str,
    index_json: &str,
    export_log_jsonl: &str,
    proxy_log_jsonl: &str,
    thumbnail_log_jsonl: &str,
    carve_log_jsonl: &str,
) -> String {
    let manifest = json_for_script(manifest_json);
    let index = json_for_script(index_json);
    let export_lines = json_for_script(&jsonl_to_array(export_log_jsonl));
    let proxy_lines = json_for_script(&jsonl_to_array(proxy_log_jsonl));
    let thumbnail_lines = json_for_script(&jsonl_to_array(thumbnail_log_jsonl));
    let carve_lines = json_for_script(&jsonl_to_array(carve_log_jsonl));
    format!(
        r#"<!doctype html>
<html lang="ko">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>FrameTrace Forensic Video Report</title>
  <style>
    body {{
      margin: 0;
      font-family: "Segoe UI", Arial, sans-serif;
      color: #202936;
      background: #ffffff;
      line-height: 1.45;
    }}
    main {{
      max-width: 1120px;
      margin: 0 auto;
      padding: 32px 28px 48px;
    }}
    h1 {{
      font-size: 28px;
      margin: 0 0 8px;
      letter-spacing: 0;
    }}
    h2 {{
      font-size: 18px;
      margin: 30px 0 10px;
      padding-bottom: 6px;
      border-bottom: 1px solid #d7dde6;
      letter-spacing: 0;
    }}
    .muted {{
      color: #657286;
      font-size: 13px;
    }}
    .summary {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
      gap: 10px;
      margin: 22px 0;
    }}
    .box {{
      border: 1px solid #d7dde6;
      border-radius: 8px;
      padding: 12px;
      background: #f8fafc;
    }}
    .box strong {{
      display: block;
      font-size: 20px;
      margin-top: 4px;
    }}
    table {{
      width: 100%;
      border-collapse: collapse;
      border: 1px solid #d7dde6;
      margin-top: 10px;
    }}
    th, td {{
      border-bottom: 1px solid #e8edf3;
      padding: 8px 10px;
      text-align: left;
      vertical-align: top;
      font-size: 12px;
    }}
    th {{
      background: #eef2f7;
      font-size: 12px;
    }}
    code {{
      font-family: Consolas, "SFMono-Regular", monospace;
      word-break: break-all;
      font-size: 11px;
    }}
    .note {{
      border-left: 4px solid #3b82f6;
      background: #f5f8ff;
      padding: 10px 12px;
      margin-top: 12px;
      font-size: 13px;
    }}
    @media print {{
      main {{ padding: 0; }}
      .box {{ break-inside: avoid; }}
      table {{ break-inside: auto; }}
      tr {{ break-inside: avoid; }}
    }}
  </style>
</head>
<body>
<main>
  <h1 id="title">FrameTrace Forensic Video Report</h1>
  <div class="muted" id="case-line"></div>
  <section class="summary">
    <div class="box">Indexed videos<strong id="count">0</strong></div>
    <div class="box">Total evidence bytes<strong id="bytes">0</strong></div>
    <div class="box">Confirmed by ffprobe<strong id="confirmed">0</strong></div>
    <div class="box">Likely sources<strong id="sources">0</strong></div>
    <div class="box">Scan warnings<strong id="warnings-count">0</strong></div>
    <div class="box">Exported clips<strong id="exports">0</strong></div>
    <div class="box">Review artifacts<strong id="derived">0</strong></div>
    <div class="box">Carved candidates<strong id="carved">0</strong></div>
  </section>

  <h2>Processing Summary</h2>
  <div id="processing"></div>
  <div class="note">This report describes derived review artifacts. Source media should remain preserved separately with acquisition logs and device-level hashes when available.</div>

  <h2>Source / Parser Assessment</h2>
  <div id="source-assessment"></div>

  <h2>Video Index</h2>
  <div id="videos"></div>

  <h2>Exported MP4/AVI Outputs</h2>
  <div id="clip-exports"></div>

  <h2>Review Artifacts</h2>
  <div id="derived-artifacts"></div>

  <h2>Recovered / Carved Candidates</h2>
  <div id="carved-artifacts"></div>
<script>
const manifest = {manifest};
const scan = {index};
const exportsLog = {export_lines};
const proxyLog = {proxy_lines};
const thumbnailLog = {thumbnail_lines};
const carveLog = {carve_lines};
const videos = Array.isArray(scan.videos) ? scan.videos : [];
const warnings = Array.isArray(scan.warnings) ? scan.warnings : [];
const derivedLog = [...proxyLog, ...thumbnailLog];

const escapeHtml = value => String(value ?? "")
  .replaceAll("&", "&amp;")
  .replaceAll("<", "&lt;")
  .replaceAll(">", "&gt;")
  .replaceAll('"', "&quot;")
  .replaceAll("'", "&#39;");

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

const sourceCounts = new Map();
videos.forEach(video => {{
  const profile = video.source_profile || {{}};
  const key = `${{profile.vendor || "Unknown"}}\t${{profile.parser || "unknown"}}\t${{profile.confidence || "-"}}\t${{profile.lane || "-"}}`;
  sourceCounts.set(key, (sourceCounts.get(key) || 0) + 1);
}});

document.getElementById("title").textContent = manifest.title || "FrameTrace Forensic Video Report";
document.getElementById("case-line").textContent = `${{manifest.case_id || "case"}} · generated from local case data`;
document.getElementById("count").textContent = scan.video_count ?? videos.length;
document.getElementById("bytes").textContent = fmtBytes(scan.total_bytes ?? 0);
document.getElementById("confirmed").textContent = videos.filter(video => video.ffprobe_ok).length;
document.getElementById("sources").textContent = sourceCounts.size;
document.getElementById("warnings-count").textContent = warnings.length;
document.getElementById("exports").textContent = exportsLog.length;
document.getElementById("derived").textContent = derivedLog.length;
document.getElementById("carved").textContent = carveLog.length;

document.getElementById("processing").innerHTML = `<table>
  <tbody>
    <tr><th>Source path</th><td><code>${{escapeHtml(scan.source_path || "-")}}</code></td></tr>
    <tr><th>Scanned at</th><td>${{escapeHtml(scan.scanned_unix || "-")}}</td></tr>
    <tr><th>Hash mode</th><td>${{scan.options?.hash_files ? "Per-file SHA-256 calculated" : "Per-file SHA-256 skipped"}}</td></tr>
    <tr><th>Metadata mode</th><td>${{scan.options?.use_ffprobe ? "ffprobe enabled" : "ffprobe skipped"}}</td></tr>
    <tr><th>Warnings</th><td>${{warnings.length ? warnings.map(escapeHtml).join("<br>") : "None"}}</td></tr>
  </tbody>
</table>`;

document.getElementById("source-assessment").innerHTML = sourceCounts.size ? `<table>
  <thead>
    <tr><th>Likely source</th><th>Parser lane</th><th>Confidence</th><th>Files</th><th>Handling note</th></tr>
  </thead>
  <tbody>
    ${{[...sourceCounts.entries()].map(([key, count]) => {{
      const [vendor, parser, confidence, lane] = key.split("\t");
      const sample = videos.find(video => (video.source_profile?.vendor || "Unknown") === vendor && (video.source_profile?.parser || "unknown") === parser);
      return `<tr>
        <td>${{escapeHtml(vendor)}}</td>
        <td>${{escapeHtml(lane)}}<br><code>${{escapeHtml(parser)}}</code></td>
        <td>${{escapeHtml(confidence)}}</td>
        <td>${{count}}</td>
        <td>${{escapeHtml(sample?.source_profile?.recommended_action || "-")}}</td>
      </tr>`;
    }}).join("")}}
  </tbody>
</table>` : "<p>No source assessment available.</p>";

document.getElementById("videos").innerHTML = videos.length ? `<table>
  <thead>
    <tr>
      <th>ID</th><th>Path</th><th>Source</th><th>Format</th><th>Duration</th><th>Size</th><th>Hash</th>
    </tr>
  </thead>
  <tbody>
    ${{videos.map(video => `<tr>
      <td>${{escapeHtml(video.id)}}</td>
      <td><code>${{escapeHtml(video.relative_path || video.source_path)}}</code></td>
      <td>${{escapeHtml(video.source_profile?.vendor || "-")}}<br><code>${{escapeHtml(video.source_profile?.parser || "-")}}</code></td>
      <td>${{escapeHtml(video.video_codec || video.extension || "-")}} ${{video.width && video.height ? `(${{video.width}}x${{video.height}})` : ""}}</td>
      <td>${{fmtDuration(video.duration_seconds)}}</td>
      <td>${{fmtBytes(video.size_bytes)}}</td>
      <td><code>${{escapeHtml(video.sha256 || video.hash_status || "-")}}</code></td>
    </tr>`).join("")}}
  </tbody>
</table>` : "<p>No indexed videos.</p>";

document.getElementById("clip-exports").innerHTML = exportsLog.length ? `<table>
  <thead>
    <tr><th>Format</th><th>Source</th><th>Output</th><th>Range</th></tr>
  </thead>
  <tbody>
    ${{exportsLog.map(item => `<tr>
      <td>${{escapeHtml(item.format || "-")}}</td>
      <td><code>${{escapeHtml(item.source_path || item.selector || "-")}}</code></td>
      <td><code>${{escapeHtml(item.output_path || "-")}}</code></td>
      <td>${{item.start_seconds ?? "-"}}, ${{item.duration_seconds ?? "-"}}</td>
    </tr>`).join("")}}
  </tbody>
</table>` : "<p>No exported clips yet.</p>";

document.getElementById("derived-artifacts").innerHTML = derivedLog.length ? `<table>
  <thead>
    <tr><th>Kind</th><th>Source</th><th>Output</th></tr>
  </thead>
  <tbody>
    ${{derivedLog.map(item => `<tr>
      <td>${{escapeHtml(item.kind || "-")}}</td>
      <td><code>${{escapeHtml(item.source_path || "-")}}</code></td>
      <td><code>${{escapeHtml(item.output_path || "-")}}</code></td>
    </tr>`).join("")}}
  </tbody>
</table>` : "<p>No proxy or thumbnail artifacts yet.</p>";

document.getElementById("carved-artifacts").innerHTML = carveLog.length ? `<table>
  <thead>
    <tr><th>ID</th><th>Signature</th><th>Offset</th><th>Size</th><th>Output</th><th>SHA-256</th></tr>
  </thead>
  <tbody>
    ${{carveLog.map(item => `<tr>
      <td>${{escapeHtml(item.id || "-")}}</td>
      <td>${{escapeHtml(item.signature || item.extension || "-")}}</td>
      <td>${{escapeHtml(item.offset ?? "-")}}</td>
      <td>${{fmtBytes(item.size_bytes)}}</td>
      <td><code>${{escapeHtml(item.output_path || "-")}}</code></td>
      <td><code>${{escapeHtml(item.sha256 || "-")}}</code></td>
    </tr>`).join("")}}
  </tbody>
</table>` : "<p>No carved candidates yet.</p>";
</script>
</main>
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
