# Code Review: frametrace-agbrowse-ui-20260626

## Verdict

- codeReview: FAIL
- codeQualityStatus: BLOCK
- recommendation: REQUEST_CHANGES
- reportPath: `.omo/evidence/frametrace-agbrowse-ui-20260626-code-review.md`
- blockers:
  - `gui/evidence-viewer/app.js` and `gui/evidence-viewer/styles.css` remain oversized production files under the required remove-ai-slops/programming perspective.
  - Hidden-but-rendered source/queue/activity UI remains in production markup and render code.

## Scope

Reviewed only the requested GUI files plus the requested notepad artifact:

- `gui/evidence-viewer/index.html`
- `gui/evidence-viewer/styles.css`
- `gui/evidence-viewer/workflow.css`
- `gui/evidence-viewer/translations.js`
- `gui/evidence-viewer/app.js`
- `.omo/ulw-loop/frametrace-agbrowse-ui-20260626/notepad.md`

The broader repository worktree is not GUI-only; `git status --short` shows many unrelated Rust/docs/tests changes and untracked `.omo` artifacts. This review does not approve those unrelated changes. The reviewed source scope itself is GUI-only: the requested five GUI files are the only source files inspected for this gate.

## Skill Perspective Check

Required skill-perspective check ran before judging maintainability and test relevance:

- `omo:remove-ai-slops`: loaded and applied to scoped production GUI files and evidence. Result: the specific `workflow-band` dead UI blocker is resolved, but dead/hidden UI remains through hidden source/queue/activity sections.
- `omo:programming`: loaded, including the code-smells reference for the 250 pure-LOC ceiling. Result: the diff still violates the oversized-file perspective for `app.js` and `styles.css`.
- `code-review`: loaded. Independent subagent review was attempted, but the agent pool returned `agent thread limit reached`; no independent subagent approval was available.

## Evidence Inspected

- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/static-assertions.json`: `ok: true`; includes checks for translations split, workflow CSS split, and dead `workflow-band` removal.
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/node-check-app.txt`: empty silent-success artifact; reran `node --check gui/evidence-viewer/app.js`, exit 0.
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/node-check-translations.txt`: empty silent-success artifact; reran `node --check gui/evidence-viewer/translations.js`, exit 0.
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/git-diff-check-gui.txt`: empty silent-success artifact; reran `git diff --check -- ...`, exit 0.
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/cargo-test-lib.txt`: `cargo test --lib` passed, 160 passed / 0 failed.
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/desktop-1280.png` and `desktop-1280-rail-actions.json`: viewer-first flow and rail actions are visible and functional.
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/mobile-375.png`: true 375px mobile surface is usable without obvious horizontal overflow.
- `.omo/ulw-loop/frametrace-agbrowse-ui-20260626/notepad.md`: exists and records GUI-only scope and agbrowse QA intent.

Evidence caveat: `quality-gate.json` still records `gateReview.status: pending` and stale watch text about `workflow-band`; `goals.json` still shows the goal `in_progress`. The code review below is based on direct inspection of current files and artifacts, not those stale status fields.

## Findings

### CRITICAL (0)

None.

### HIGH (2)

1. `gui/evidence-viewer/app.js`, `gui/evidence-viewer/styles.css`

   Issue: Oversized production files remain unresolved under the loaded programming/remove-ai-slops criteria. Direct pure-LOC measurement found:
   - `app.js`: 1376 pure LOC
   - `styles.css`: 1068 pure LOC
   - `translations.js`: 487 pure LOC, acceptable only as a pure data-table split
   - `workflow.css`: 210 pure LOC
   - `index.html`: 190 pure LOC

   Risk: The final fixes reduced size by extracting translations/workflow CSS, but `app.js` and base `styles.css` are still far beyond the 250 pure-LOC ceiling. They continue to combine multiple responsibilities and remain difficult to review safely.

   Fix: Split `app.js` by responsibility (data/model, filtering, rendering, event actions, localization helpers) and split `styles.css` by visible surface/responsibility until non-data files are below the project ceiling or carry a deliberate documented exception.

2. `gui/evidence-viewer/index.html:34`, `gui/evidence-viewer/index.html:187`, `gui/evidence-viewer/styles.css:204`, `gui/evidence-viewer/styles.css:995`, `gui/evidence-viewer/app.js:528`, `gui/evidence-viewer/app.js:571`, `gui/evidence-viewer/app.js:858`, `gui/evidence-viewer/app.js:881`

   Issue: Hidden-but-rendered UI remains. The source/queue pane and activity section are present in markup, globally hidden with `display: none`, and still populated on every `renderAll()` through `renderSources()`, `renderQueue()`, and `renderInspector()`.

   Risk: This is dead/ambiguous production UI. It creates maintenance burden and false confidence because source and queue controls have active render/event code that the user cannot use. The specific `workflow-band` blocker is resolved, but the broader dead-UI blocker is not fully resolved.

   Fix: Either restore these surfaces as visible/reachable UI, or remove the hidden markup, CSS, translations, element lookups, render paths, and event wiring.

### MEDIUM (2)

1. `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/quality-gate.json`

   Issue: The quality-gate artifact still says `gateReview.status: pending` and includes stale watch text about `workflow-band`, even though direct inspection finds no `workflow-band` in current GUI files.

   Risk: The artifact set is internally inconsistent. Reviewers must re-run direct checks instead of trusting the stored gate summary.

   Fix: Refresh gate/status artifacts after the final fixes, or remove stale gate fields from the evidence bundle.

2. `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/git-diff-check-gui.txt`

   Issue: The stored diff-check artifact is silent success, but `workflow.css` and `translations.js` are untracked GUI files, so ordinary `git diff --check` does not prove whitespace/diff hygiene for those files.

   Risk: Evidence is not misleading about tracked changes, but it is incomplete for newly split untracked files.

   Fix: Add an explicit check over untracked GUI files, or stage/include them before generating diff hygiene evidence.

### LOW (1)

1. `gui/evidence-viewer/translations.js:1`

   Issue: A single header comment describes the translation table. This is harmless, but because `translations.js` is a pure data table and already oversized by line count, keep it as the only non-data line.

   Risk: Low. No functional risk observed.

   Fix: No required fix unless further non-data logic grows in this file.

## Requested Answers

- Oversized blockers resolved: NO. `translations.js` and `workflow.css` were split, but `app.js` and `styles.css` remain oversized.
- Dead-UI blockers resolved: PARTIAL. `workflow-band` is removed, but hidden rendered source/queue/activity UI remains.
- Scope remains GUI-only: The reviewed source scope is GUI-only. The full dirty worktree is not GUI-only and was not approved.
- Evidence relevance: Browser screenshots/action JSON are relevant and support the user-visible flow. Static/node/cargo evidence is useful but not sufficient to clear maintainability blockers.

## Recommendation

REQUEST_CHANGES. No CRITICAL findings, but HIGH maintainability/dead-UI blockers remain.
