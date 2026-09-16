# deepfake-lens integration

FrameTrace screens evidence for synthetic-media signals through the
[deepfake-lens](https://github.com/daeryundf2-prog/deepfake-lens) sidecar
(Python, local-first). It is invoked as a subprocess — the same boundary
as ffprobe / Sleuth Kit / libewf — via `src/deepfake.rs`.

## Contract

```
deepfake-lens forensic <file> --format json   →  JSON on stdout
```

```json
{
  "score": 0,                    // 0-100 review priority, NOT a verdict
  "band": "low",                 // low | medium | high | unknown
  "band_label": "낮음",
  "verdict": "…",
  "signals": [{"title": "…", "detail": "…", "weight": 20}],
  "limitations": ["…"],
  "provenance_records": [],
  "has_c2pa": false,
  "has_synthid": false,
  "has_watermark": false
}
```

**Framing rule:** a `high` score means "review this first", never
"confirmed synthetic". Surfaces must keep that wording — the tool has
measured false-positive/false-negative behaviour (see
`experiments/TEXT_DETECTION_EVAL.md` in the deepfake-lens repo).

## Resolution order

1. `FRAMETRACE_DEEPFAKE_LENS` env var — full path or bare tool name
2. `deepfake-lens` on PATH (`pip install deepfake-lens`)
3. Fallback: `python -m deepfake_lens.cli` — for source checkouts set
   `FRAMETRACE_DEEPFAKE_LENS_HOME=<repo root>` so PYTHONPATH resolves.

## CLI

```
frametrace deepfake-screen <file> [--json-out out.json]
```

## Wiring into the case pipeline

1. **Artifact file — implemented.** `scan-folder --deepfake` screens
   each indexed file and writes `artifacts/deepfake/<id>.json`
   (full forensic report, or `{"ok":false,"error":…}` on failure).
   Screening failures become scan warnings, never aborts; incremental
   rescans and checkpoint replay skip already-indexed files.
2. **`VideoRecord` field**: add `pub deepfake: Option<DeepfakeSummary>`
   serialized as `deepfake_*` flattened keys (same pattern as
   `probe_summary_flat`). Touches the published JSONL contract — bump
   deliberately.
3. **Viewer badge**: `evidence_viewer.js` card renderer already composes
   chips (`kind-badge`, `badge warn`, …). Add a `badge dfl` chip driven
   by `record.deepfake_band` (high→red, medium→amber) plus a detail
   section listing `signal_titles` and `limitations`.

Only non-video files (audio, images, documents, archives) currently sit
outside `VideoRecord`; screen them through the same lane when the index
grows a generic file record.

## Notes

- Timeout: 900 s (neural members are slow on CPU-only boxes).
- Optional weights (SyncNet, AIDE, AASIST, ECAPA) live under the
  deepfake-lens `models/` dir; absent weights degrade to heuristics.
- Uploaded evidence bytes never leave the workstation; the sidecar does
  no network I/O.
