use crate::util::json_for_script;

const VIEWER_TEMPLATE: &str = include_str!("../assets/evidence_viewer.html");
const VIEWER_CSS: &str = include_str!("../assets/evidence_viewer.css");
const VIEWER_JS: &str = include_str!("../assets/evidence_viewer.js");

pub fn render_review_html(manifest_json: &str, index_json: &str) -> String {
    let manifest = json_for_script(manifest_json);
    let index = json_for_script(index_json);
    format!(
        r#"<!doctype html>
<html lang="ko">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="icon" href="data:,">
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
    #table-wrap {{
      max-height: calc(100vh - 170px);
      overflow: auto;
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
      top: 0;
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
    filesystem_log_jsonl: &str,
    validation_log_jsonl: &str,
    fls_entries_jsonl: &str,
    thumbs_json: &str,
) -> String {
    // The layout/markup lives in assets/evidence_viewer.* and is embedded at
    // compile time, keeping the generated page a single serverless file.
    let data = format!(
        "window.__FRAMETRACE_DATA__ = {{manifest:{manifest},scan:{index},carveLog:{carve_lines},filesystemLog:{filesystem_lines},validationLog:{validation_lines},flsEntries:{fls_lines},thumbs:{thumbs_lines}}};",
        manifest = json_for_script(manifest_json),
        index = json_for_script(index_json),
        carve_lines = json_for_script(&jsonl_to_array(carve_log_jsonl)),
        filesystem_lines = json_for_script(&jsonl_to_array(filesystem_log_jsonl)),
        validation_lines = json_for_script(&jsonl_to_array(validation_log_jsonl)),
        fls_lines = json_for_script(&jsonl_to_array(fls_entries_jsonl)),
        thumbs_lines = thumbs_json,
    );
    VIEWER_TEMPLATE
        .replace("__CSS__", VIEWER_CSS)
        .replace("__DATA__", &data)
        .replace("__JS__", VIEWER_JS)
}

fn jsonl_to_array(jsonl: &str) -> String {
    let items: Vec<&str> = jsonl
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    format!("[{}]", items.join(","))
}

#[cfg(test)]
mod tests {
    use super::render_evidence_viewer_html;
    use std::path::PathBuf;
    use std::process::Command;

    #[test]
    fn evidence_viewer_includes_filesystem_recovery_records() {
        let manifest = r#"{"case_id":"FT-1","title":"Test"}"#;
        let index = r#"{"videos":[]}"#;
        let filesystem = r#"{"event":"recover-inode","partition_offset":2048,"inode":"1304","output_path":"/case/artifacts/recovered/filesystem/inode_1304.bin","size_bytes":10,"sha256":"abc","validation_status":"candidate-unvalidated"}"#;
        let html = render_evidence_viewer_html(manifest, index, "", filesystem, "", "", "{}");
        assert!(html.contains("recoveredFilesystemLog"));
        assert!(html.contains("tsk/icat"));
        assert!(html.contains("inode_1304.bin"));
    }

    fn extract_script_blocks(html: &str) -> Vec<String> {
        let mut blocks = Vec::new();
        let mut rest = html;
        while let Some(start) = rest.find("<script>") {
            let after = &rest[start + "<script>".len()..];
            let Some(end) = after.find("</script>") else {
                break;
            };
            blocks.push(after[..end].to_string());
            rest = &after[end..];
        }
        blocks
    }

    fn assert_script_blocks_parse_with_node(page_name: &str, html: &str) {
        let Ok(node_version) = Command::new("node").arg("--version").output() else {
            return; // node is optional locally; CI checks explicitly.
        };
        if !node_version.status.success() {
            return;
        }
        for (index, script) in extract_script_blocks(html).into_iter().enumerate() {
            let path: PathBuf = std::env::temp_dir().join(format!(
                "frametrace-script-check-{}-{}-{}.js",
                page_name,
                index,
                std::process::id()
            ));
            std::fs::write(&path, &script).expect("script temp file should write");
            let output = Command::new("node")
                .arg("--check")
                .arg(&path)
                .output()
                .expect("node should run after a successful --version");
            let _ = std::fs::remove_file(&path);
            assert!(
                output.status.success(),
                "{page_name} script block {index} has a syntax error:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn generated_page_scripts_are_syntactically_valid() {
        let manifest = r#"{"case_id":"FT-1","title":"테스트 케이스"}"#;
        let index = r#"{"videos":[{"id":"vid_000001","source_path":"C:\\case\\a.mp4","file_url":"file:///C:/case/a.mp4","relative_path":"a.mp4","size_bytes":1,"source_profile":{"vendor":"v","parser":"p"},"ffprobe_ok":true}]}"#;
        let carve = r#"{"id":"carve_000001","output_path":"\\\\?\\C:\\case\\carved\\a.mp4","signature":"mp4-ftyp","size_bytes":2,"sha256":"d","validation_status":"candidate-unvalidated"}"#;
        let filesystem = r#"{"event":"recover-inode","partition_offset":2048,"inode":"1304","output_path":"\\\\?\\C:\\case\\inode.bin","size_bytes":10,"sha256":"a","validation_status":"candidate-unvalidated"}"#;
        let validation = r#"{"selector":"vid_000001","target_path":"C:\\case\\a.mp4","validation_status":"ffprobe-video-stream-confirmed"}"#;

        assert_script_blocks_parse_with_node(
            "review",
            &crate::html_report::render_review_html(manifest, index),
        );
        assert_script_blocks_parse_with_node(
            "viewer",
            &render_evidence_viewer_html(manifest, index, carve, filesystem, validation, "", "{}"),
        );
        assert_script_blocks_parse_with_node(
            "report",
            &crate::report::render_case_report(&crate::report::ReportInputs {
                manifest_json: manifest,
                index_json: index,
                export_log_jsonl: "",
                proxy_log_jsonl: "",
                thumbnail_log_jsonl: "",
                carve_log_jsonl: carve,
                filesystem_log_jsonl: filesystem,
                validation_log_jsonl: validation,
                batch_log_jsonl: "",
                scan_runs_json: "[]",
                marks_json: "[]",
            }),
        );
    }
}
