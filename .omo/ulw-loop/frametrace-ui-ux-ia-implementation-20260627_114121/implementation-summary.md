# FrameTrace Evidence Viewer UI/UX/IA Implementation Summary

Date: 2026-06-27
Repo: `/Users/shinyoohag/Desktop/frametrace`
Branch: `codex/frametrace-forensic-hardening`

## Implemented

- Reworked the static evidence viewer first screen into a field evidence review workbench with a visible flow for Evidence source, Video candidates, Verification status, Export, and Report.
- Replaced the blank-looking 12-column candidate table at narrow desktop widths with visible review rows: thumbnail preview, primary file label/path, metadata, status chips, and a preview action.
- Kept conservative forensic state wording visible: `local-first`, `candidate-unvalidated`, `verification required`, `hash/check pending`, `export draft`, `report draft`, and `not court-ready`.
- Moved mobile/tablet IA to evidence-first ordering: viewer, candidate browser, workflow summary, then inspector.
- Compacted mobile selected-evidence rail so the video frame remains visible before workflow cards.
- Added cache-busting query strings to static viewer assets so local QA does not reuse stale JS/CSS.
- Removed the generated `review/evidence-viewer.html` fixed `min-width: 1180px` blocker and added responsive CSS; final proof is browser layout evidence rather than a brittle CSS-token unit test.

## Verification Evidence

- `after-1280.png`: desktop agbrowse screenshot.
- `after-768.png`: tablet-width agbrowse screenshot.
- `after-500-agbrowse.png`: agbrowse headed Chrome minimum-width mobile evidence; requested 375, actual Chrome viewport 500.
- `after-375.png`: exact 375px supplemental browser screenshot using bundled Playwright because headed agbrowse/Chrome would not shrink below 500px.
- `after-1280-layout.json`, `after-768-layout.json`, `after-500-agbrowse-layout.json`, `after-375-layout.json`: no horizontal overflow, visible candidate rows, visible state labels.
- `after-1280-console.txt`, `after-768-console.txt`, `after-500-agbrowse-console.txt`, `after-375-console.txt`: no console errors captured.
- `after-preview-action.png` and `after-preview-action.json`: first candidate preview action opens the local-first preview overlay and keeps `candidate-unvalidated`, `verification required`, and `hash/check pending` visible.
- `generated-500-agbrowse.png`, `generated-500-agbrowse-snapshot.txt`, `generated-500-agbrowse-layout.json`, and `generated-500-agbrowse-console.txt`: real `make-review` generated viewer opened in headed agbrowse/Chrome with no horizontal overflow.
- `generated-375.png`, `generated-375-layout.json`, and `generated-375-console.txt`: exact 375px supplemental browser evidence for the generated `review/evidence-viewer.html`.
- After the generated viewer row fix, `generated-500-agbrowse-layout.json` and `generated-375-layout.json` report `badgeOverflow: false` and `badgeWithinRow: true`.

## Commands Verified

- `node --check gui/evidence-viewer/app.js`
- `node --check gui/evidence-viewer/translations.js`
- `git diff --check`
- restricted overclaim scan over `gui/evidence-viewer`, `src/html_report.rs`, `README.md`, and `docs`
- `cargo test --manifest-path /Users/shinyoohag/Desktop/frametrace/Cargo.toml html_report`
- `cargo test --manifest-path /Users/shinyoohag/Desktop/frametrace/Cargo.toml`
- `cargo run --quiet -- init-case ...`
- `cargo run --quiet -- scan-folder ... --no-ffprobe --max-depth 1`
- `cargo run --quiet -- make-review ...`
- `cargo run --quiet -- make-review ...` after the generated viewer mobile row fix
- `agbrowse start --headed`
- `agbrowse navigate http://127.0.0.1:4173/index.html`
- `agbrowse navigate http://127.0.0.1:4175/evidence-viewer.html`
- `agbrowse snapshot`
- `agbrowse screenshot`
- `agbrowse console`
