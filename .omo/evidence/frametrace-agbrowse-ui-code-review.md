# Code Review: frametrace-agbrowse-ui

## Verdict

- codeReview: PASS
- codeQualityStatus: WATCH
- recommendation: APPROVE
- blockers: []

## Scope

Reviewed only the scoped GUI diff and requested files:

- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/gui-diff.patch`
- `gui/evidence-viewer/index.html`
- `gui/evidence-viewer/styles.css`
- `gui/evidence-viewer/app.js`

The scoped patch changes only those three GUI files. The broader worktree is dirty with many unrelated Rust/docs/test changes; this review did not modify or revert them.

Task brief found at `.omo/ulw-loop/frametrace-agbrowse-ui-20260626/brief.md`. No matching `frametrace-agbrowse-ui` notepad file was present.

## Skill Perspective Check

Ran required skill-perspective check before judging maintainability/test relevance:

- `omo:remove-ai-slops`: loaded and applied to production GUI/test evidence. Result: no blocking slop, no deletion-only tests, no tautological requested-removal tests found in scoped review. One LOW dead/hidden UI concern is listed below.
- `omo:programming`: loaded and applied its maintainability/code-smell criteria. Result: no blocking production complexity for this scoped GUI gate, but existing oversized static files remain a WATCH item.

## Evidence Inspected

- Static assertions: `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/static-assertions.json` reported `ok: true`.
- Syntax: `node --check gui/evidence-viewer/app.js` exited cleanly. The stored `node-check-app.txt` artifact is empty, which is normal for silent-success `node --check`; I reran it directly.
- Diff hygiene: `git diff --check -- gui/evidence-viewer/index.html gui/evidence-viewer/styles.css gui/evidence-viewer/app.js` exited cleanly. The stored `git-diff-check-gui.txt` artifact is empty, consistent with silent success; I reran it directly.
- Rust regression guard: `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/cargo-test-lib.txt` shows `cargo test --lib` passed, 160 passed / 0 failed.
- Browser evidence inspected:
  - Desktop: `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/desktop-1280.png`, `desktop-1280-overflow.json`, `desktop-1280-rail-actions.json`
  - Tablet: `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/tablet-768.png`, `tablet-768-overflow.json`
  - Mobile: `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/mobile-375.png`, `mobile-375-scrolled.png`, `mobile-375-inspector.png`, `mobile-375-report.png`, matching overflow JSON artifacts
- Cleanup evidence: `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/cleanup-receipt.txt` reports agbrowse stopped and port 4181 free.

## Findings

### CRITICAL (0)

None.

### HIGH (0)

None.

### MEDIUM (1)

1. `gui/evidence-viewer/app.js`, `gui/evidence-viewer/styles.css`

   Issue: The scoped patch adds substantial logic/style into already oversized static files. Measured pure LOC: `app.js` 1862, `styles.css` 1363, `index.html` 231. The patch adds 381 lines to `app.js` and 665 lines to `styles.css`.

   Risk: This violates the loaded programming perspective's file-size guidance and makes future UI changes harder to review. I am not blocking this gate because the task is explicitly scoped to the existing static GUI surface and the browser evidence verifies the requested first-screen behavior, but this should not keep growing in one file.

   Recommendation: Next GUI change should split by responsibility, for example inventory/rendering/actions/localization in JS and viewer/browser/inspector/responsive sections in CSS.

### LOW (1)

1. `gui/evidence-viewer/index.html:45`, `gui/evidence-viewer/styles.css:1290`

   Issue: The new `.workflow-band` markup and styles are added, but the final CSS rule `.browser-pane > .workflow-band { display: none; }` hides that band across the supplied viewports. The visible workflow is carried by `.workbench-flow` and the selected-evidence rail instead.

   Risk: This is minor dead UI/maintenance noise and can confuse future editors about which first-screen workflow component is authoritative.

   Recommendation: Either remove the hidden workflow band/translations/styles or explicitly document/render it where it is intended to appear.

## Positive Checks

- First-screen visual direction matches the brief: video/recovered evidence is primary, with source evidence, candidate status, validation state, export preview, report/package flow, and conservative local-first/queued wording visible in desktop and mobile captures.
- No overclaim/hype terms or unredacted absolute local paths were found in the scoped GUI text sweep.
- Rail actions update the expected queued/report/package states in `desktop-1280-rail-actions.json`.
- Scope is limited to requested GUI files in the reviewed patch.
- Dirty unrelated files appear preserved; no evidence of revert or unrelated edit in the scoped GUI patch.

## Final

APPROVE with WATCH items. No CRITICAL or HIGH findings remain.
