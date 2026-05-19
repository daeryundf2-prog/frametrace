# FrameTrace OpenDesign Review Checklist

Use this checklist before accepting any OpenDesign-generated FrameTrace UI artifact.

## Forensic Correctness

- Original evidence is never modified or implied to be editable.
- Source path, parser lane, validation state, and hash state remain visible for the selected item.
- Carved or recovered media is labeled as a candidate until engine validation exists.
- Derived outputs show source linkage and audit requirements.
- E01, raw image, mounted folder, SD card, and exported media sources can be distinguished.
- Evidence values are preserved verbatim across Korean/English locale changes.

## Large Case Usability

- The inventory design can handle 1,000+ files without loading all thumbnails at once.
- Filtering, search, queue states, and counts are visible before opening a file.
- Row heights, toolbar heights, and viewer dimensions stay stable during state changes.
- Review queues separate unreviewed, flagged, report-selected, exported, and risk items.
- Multi-channel review has a clear synchronized front/rear or channel-split path.

## Viewer Quality

- Video/photo viewing remains the center of the screen.
- Timecode and current source context are visible while viewing.
- Playback, frame-step, zoom, capture, range selection, and export controls are discoverable.
- Inspector data does not obscure the media.
- Text fits inside buttons and badges in Korean and English.

## Production Handoff

- Any command that changes case state maps to a Rust engine command and audit event.
- Mock-only states are explicitly marked in docs or prototype code.
- Report language is stored separately from original evidence metadata.
- Exported MP4/AVI and captured frames are treated as derived artifacts.
