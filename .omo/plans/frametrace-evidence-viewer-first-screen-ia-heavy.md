# FrameTrace Evidence Viewer First-Screen IA HEAVY Plan

## TL;DR
> Summary:      Implement the Evidence Viewer first screen as a Windows-local forensic review workstation: sources, video candidates, validation state, export, and report flow visible at a glance without release hype or server assumptions.
> Deliverables:
> - Failing-first browser proof before UI changes.
> - First-screen IA changes limited by default to `gui/evidence-viewer/index.html`, `gui/evidence-viewer/styles.css`, and `gui/evidence-viewer/app.js`.
> - Conditional Rust/generated-review parity only if the CLI contract has drifted.
> - Browser QA evidence at 375, 768, and 1280 px plus visual/reviewer gate and cleanup receipts.
> Effort:       Medium
> Risk:         Medium - the worktree is already dirty, the GUI files are already modified, and the current CSS is desktop-min-width first.

## Scope
### Must have
- Preserve FrameTrace as a Windows local workstation, not a server product, per `README.md:3` and `README.md:34`.
- Keep the primary screen as the Evidence Viewer, not a marketing dashboard, per `docs/EVIDENCE_VIEWER_GUI.md:5-19`.
- Show evidence sources, review queues, video/candidate inventory, validation state, export actions, and report/package flow in the first viewport.
- Keep candidate/recovered media explicitly `candidate-unvalidated` until engine validation exists, per `docs/EVIDENCE_VIEWER_GUI.md:168-175` and `opendesign/design-systems/frametrace-forensic-workstation/DESIGN.md:80-92`.
- Maintain local/serverless prototype behavior: `gui/evidence-viewer/index.html` opens directly from disk and `make-review` emits `review/evidence-viewer.html`, per `docs/EVIDENCE_VIEWER_GUI.md:191-207`.
- Use the OpenDesign FrameTrace design system as the primary style authority: `opendesign/design-systems/frametrace-forensic-workstation/DESIGN.md:3-69` and `opendesign/design-systems/frametrace-forensic-workstation/review_checklist.md:5-35`.
- Treat the root `DESIGN.md` found in this worktree as supplemental only; the user request named the OpenDesign design-system folder as authority. Verify this before edits because the request said root `DESIGN.md` was absent, but exploration found `DESIGN.md:1`.
- Preserve evidence values verbatim across KO/EN locale changes: paths, hashes, parser IDs, and source labels are evidence data, not translated copy.
- Record all task evidence under `.omo/evidence/task-<N>-frametrace-evidence-viewer-first-screen-ia-heavy.*`.

### Must NOT have (guardrails, anti-slop, scope boundaries)
- Do not build a landing page, hero page, release hype, decorative cards, gradient blobs, or atmospheric filler; see `opendesign/design-systems/frametrace-forensic-workstation/DESIGN.md:103-111`.
- Do not imply durable forensic mutation from prototype-only buttons. Any review/export/report/validation UI action must read as queued/preview/non-mutating unless backed by an engine command and audit event.
- Do not promote candidates to verified based on UI selection, playback mock, color, or report inclusion.
- Do not translate hashes, paths, source IDs, parser IDs, command names, or raw evidence metadata.
- Do not revert dirty-worktree changes. Read the existing modified files and patch around them.
- Do not change Rust unless Task 8 proves the CLI/generated review contract is stale.
- Do not add a persistent dev server or checked-in package manager setup unless the executor has a concrete QA-only reason. Browser QA should prefer `file://` plus temporary Playwright execution.

## Verification strategy
> Zero human intervention - all verification is agent-executed.
- Test decision: TDD + tests-after. First run failing browser assertions/screenshots at 375/768/1280 before production UI edits; after implementation rerun the same browser assertions, `node --check`, and feasible cargo tests.
- QA policy: every task has agent-executed scenarios.
- Evidence: `.omo/evidence/task-<N>-frametrace-evidence-viewer-first-screen-ia-heavy.<ext>`
- Browser tool: Playwright real Chrome via `npm exec --yes --package=playwright@1.61.0 -- node ...`. Use `channel: "chrome"` first. If Chrome is unavailable, download/use agent-browser from `https://github.com/vercel-labs/agent-browser` and capture the same viewport screenshots plus DOM assertions.
- Official QA references: Playwright locators/screenshots/viewports from `https://playwright.dev/docs/locators`, `https://playwright.dev/docs/screenshots`, `https://playwright.dev/docs/emulation`, and browser channels from `https://playwright.dev/docs/browsers`.
- Windows IA references: use Microsoft Windows app/design guidance as guardrails only, not forensic doctrine: `https://learn.microsoft.com/en-us/windows/apps/design/`, `https://learn.microsoft.com/en-us/windows/apps/design/basics/navigation-basics`, `https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/command-bar`, `https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/infobar`.

## Execution strategy
### Parallel execution waves
> Target 5-8 tasks per wave. <3 per wave (except final) = under-splitting.
> Extract shared dependencies as Wave-1 tasks to maximize parallelism.

Wave 1 (no dependencies):
- Task 1: Failing-first first-screen baseline and evidence ledger
- Task 2: HTML first-screen IA skeleton and landmark cleanup
- Task 3: Evidence state, candidate validation, and localization vocabulary
- Task 4: Responsive workstation CSS at 375/768/1280
- Task 8: Conditional CLI/generated-review parity audit

Wave 2 (after Wave 1):
- Task 5: depends [3, 4] - evidence source and queue rendering
- Task 6: depends [3, 4] - video/candidate inventory and validation state rendering
- Task 7: depends [2, 3] - export/report/validation flow affordances
Note: Tasks 5-7 all touch `gui/evidence-viewer/app.js`; in one worktree they must serialize in numeric order. They can only parallelize in separate branches with explicit integration.

Wave 3 (after Wave 2):
- Task 9: depends [1, 2, 3, 4, 5, 6, 7, 8] - browser QA, visual/reviewer gate, cleanup receipts

Critical path: Task 1 -> Task 4 -> Task 6 -> Task 9

### Dependency matrix
| Task | Depends on | Blocks | Can parallelize with |
|------|------------|--------|----------------------|
| 1    | none       | 9      | 2, 3, 4, 8           |
| 2    | none       | 7, 9   | 1, 3, 4, 8           |
| 3    | none       | 5, 6, 7, 9 | 1, 2, 4, 8       |
| 4    | none       | 5, 6, 9 | 1, 2, 3, 8          |
| 5    | 3, 4       | 9      | none in one worktree |
| 6    | 3, 4       | 9      | none in one worktree |
| 7    | 2, 3       | 9      | none in one worktree |
| 8    | none       | 9      | 1, 2, 3, 4           |
| 9    | 1-8        | final  | final verifiers only |

## Todos
> Implementation + Test = ONE task. Never separate.
> Every task MUST have: References + Acceptance Criteria + QA Scenarios + Commit.

- [ ] 1. Failing-first first-screen baseline and evidence ledger

  What to do: Before source edits, capture the current first-screen IA failure at 375, 768, and 1280 px. Assertions must check horizontal overflow, first-viewport visibility/accessibility of evidence sources, video candidates, validation status, export controls, report flow, and candidate-unvalidated language. Save screenshots, DOM metrics, command output, `git status --short`, and a cleanup receipt.
  Must NOT do: Do not edit source files, normalize existing dirty files, or start a persistent server.

  Parallelization: Can parallel: YES | Wave 1 | Blocks: [9] | Blocked by: []

  References (executor has NO interview context - be exhaustive):
  - Pattern:  `gui/evidence-viewer/index.html:11` - current app shell starts here.
  - Pattern:  `gui/evidence-viewer/index.html:32` - current four-pane workspace structure.
  - Pattern:  `gui/evidence-viewer/styles.css:26` - current `body` min-width causes narrow viewport risk.
  - Pattern:  `gui/evidence-viewer/app.js:1702` - current static prototype boots with `renderAll()`.
  - External: `https://playwright.dev/docs/screenshots` - screenshot evidence.
  - External: `https://playwright.dev/docs/emulation` - viewport coverage.

  Acceptance criteria (agent-executable only):
  - [ ] `git status --short > .omo/evidence/task-01-frametrace-evidence-viewer-first-screen-ia-heavy-status-before.txt` records the dirty baseline.
  - [ ] `node --check gui/evidence-viewer/app.js | tee .omo/evidence/task-01-frametrace-evidence-viewer-first-screen-ia-heavy-node-check.txt` completes and its output is retained.
  - [ ] The browser assertion run writes `.omo/evidence/task-01-frametrace-evidence-viewer-first-screen-ia-heavy-baseline.json` containing per-viewport `scrollWidth`, `innerWidth`, visible text findings, and at least one failing first-screen IA assertion.
  - [ ] Screenshots exist for 375, 768, and 1280 px under `.omo/evidence/`.

  QA scenarios (MANDATORY - task incomplete without these):
  > Name the exact tool AND its exact invocation - not "verify it works". Browser use: use Chrome to drive the page; if Chrome is not available, download and use agent-browser (https://github.com/vercel-labs/agent-browser). Computer use: OS-level GUI automation for a non-browser desktop app.
  ```
  Scenario: failing-first browser IA proof
    Tool:     bash + playwright(real Chrome)
    Steps:    npm exec --yes --package=playwright@1.61.0 -- node <<'NODE'
              const { chromium } = require("playwright");
              const fs = require("fs");
              const path = require("path");
              const root = process.cwd();
              fs.mkdirSync(".omo/evidence", { recursive: true });
              (async () => {
                const browser = await chromium.launch({ channel: "chrome", headless: true });
                const page = await browser.newPage();
                const fileUrl = "file://" + path.resolve(root, "gui/evidence-viewer/index.html");
                const viewports = [{w:375,h:812},{w:768,h:1024},{w:1280,h:800}];
                const out = [];
                for (const vp of viewports) {
                  await page.setViewportSize({ width: vp.w, height: vp.h });
                  await page.goto(fileUrl);
                  await page.waitForSelector("#fileRows .data-row");
                  const metric = await page.evaluate(() => ({
                    innerWidth,
                    scrollWidth: document.documentElement.scrollWidth,
                    sources: !!document.querySelector("#sourceList")?.innerText.trim(),
                    candidates: document.body.innerText.includes("candidate-unvalidated") || document.body.innerText.includes("미검증 후보"),
                    validation: document.body.innerText.includes("Validation") || document.body.innerText.includes("검증"),
                    exportFlow: document.body.innerText.includes("MP4") && document.body.innerText.includes("AVI"),
                    reportFlow: document.body.innerText.includes("Report") || document.body.innerText.includes("보고서"),
                  }));
                  out.push({ viewport: vp, ...metric, pass: metric.scrollWidth <= metric.innerWidth && metric.sources && metric.candidates && metric.validation && metric.exportFlow && metric.reportFlow });
                  await page.screenshot({ path: `.omo/evidence/task-01-frametrace-evidence-viewer-first-screen-ia-heavy-${vp.w}.png`, fullPage: true });
                }
                await browser.close();
                fs.writeFileSync(".omo/evidence/task-01-frametrace-evidence-viewer-first-screen-ia-heavy-baseline.json", JSON.stringify(out, null, 2));
                if (out.every(x => x.pass)) throw new Error("Expected failing-first proof, but every viewport passed");
              })().catch((error) => { console.error(error); process.exit(1); });
              NODE
    Expected: Command exits 0 because at least one viewport currently fails the desired first-screen IA assertions; JSON and three PNGs exist.
    Evidence: .omo/evidence/task-01-frametrace-evidence-viewer-first-screen-ia-heavy-baseline.json

  Scenario: cleanup receipt
    Tool:     bash
    Steps:    { git status --short; printf '\nNo persistent server started; browser process closed by script.\n'; } > .omo/evidence/task-01-frametrace-evidence-viewer-first-screen-ia-heavy-cleanup-receipt.md
    Expected: Receipt exists and does not claim source cleanup or revert dirty user work.
    Evidence: .omo/evidence/task-01-frametrace-evidence-viewer-first-screen-ia-heavy-cleanup-receipt.md
  ```

  Commit: NO | Message: `test(gui): capture failing first-screen IA baseline` | Files: [.omo/evidence/*]

- [ ] 2. HTML first-screen IA skeleton and landmark cleanup

  What to do: Reshape `gui/evidence-viewer/index.html` so the first viewport reads as a local Windows forensic review tool. Keep the four-pane workstation, but make the global case strip and pane headings explicitly expose evidence sources, candidate queue, validation state, export queue, and report/package flow. Use semantic landmarks and aria labels that survive KO/EN localization.
  Must NOT do: Do not create a landing page, hero, marketing copy, or extra navigation hierarchy.

  Parallelization: Can parallel: YES | Wave 1 | Blocks: [7, 9] | Blocked by: []

  References:
  - Pattern:  `gui/evidence-viewer/index.html:12` - topbar brand/case/action surface.
  - Pattern:  `gui/evidence-viewer/index.html:33` - evidence source pane.
  - Pattern:  `gui/evidence-viewer/index.html:44` - evidence browser pane.
  - Pattern:  `gui/evidence-viewer/index.html:158` - forensic inspector pane.
  - Product:  `docs/EVIDENCE_VIEWER_GUI.md:9` - required left/center/right screen model.
  - Product:  `docs/EVIDENCE_VIEWER_GUI.md:14` - immediate questions the screen must answer.
  - Design:   `opendesign/design-systems/frametrace-forensic-workstation/DESIGN.md:47` - four-pane workstation layout.
  - External: `https://learn.microsoft.com/en-us/windows/apps/design/basics/navigation-basics` - keep hierarchy shallow.

  Acceptance criteria:
  - [ ] Browser DOM assertions at 1280 px find visible first-screen strings for evidence sources, video candidates, validation, export, and report/package flow.
  - [ ] `document.querySelectorAll("main, aside, section, header").length` is unchanged or reduced unless each new landmark has a specific IA purpose.
  - [ ] `node --check gui/evidence-viewer/app.js` still passes.

  QA scenarios:
  ```
  Scenario: first-screen landmark visibility
    Tool:     bash + playwright(real Chrome)
    Steps:    npm exec --yes --package=playwright@1.61.0 -- node -e 'const { chromium } = require("playwright"); const path=require("path"); (async()=>{const b=await chromium.launch({channel:"chrome",headless:true}); const p=await b.newPage({viewport:{width:1280,height:800}}); await p.goto("file://"+path.resolve("gui/evidence-viewer/index.html")); await p.waitForSelector("#fileRows .data-row"); const text=await p.locator("body").innerText(); for (const s of ["FrameTrace","검증","MP4","AVI"]) if(!text.includes(s)) throw new Error("missing "+s); await p.screenshot({path:".omo/evidence/task-02-frametrace-evidence-viewer-first-screen-ia-heavy-1280.png", fullPage:true}); await b.close();})();'
    Expected: Command exits 0 and screenshot shows the workstation first screen, not a landing page.
    Evidence: .omo/evidence/task-02-frametrace-evidence-viewer-first-screen-ia-heavy-1280.png

  Scenario: no marketing chrome
    Tool:     bash
    Steps:    ! rg -n "launch|release|hero|cloud|server product|AI-powered|next-generation" gui/evidence-viewer/index.html gui/evidence-viewer/app.js
    Expected: Command exits 0; no release-hype or hero wording appears.
    Evidence: .omo/evidence/task-02-frametrace-evidence-viewer-first-screen-ia-heavy-no-hype.txt
  ```

  Commit: YES | Message: `feat(gui): clarify evidence viewer first-screen IA` | Files: [gui/evidence-viewer/index.html]

- [ ] 3. Evidence state, candidate validation, and localization vocabulary

  What to do: Update `gui/evidence-viewer/app.js` mock state and KO/EN vocabulary so first-screen data explicitly includes source kind, candidate status, validation requirement, export/report queue state, and non-mutating prototype wording. Candidate records must remain `candidate-unvalidated`; report/export buttons must queue or preview, not imply completion.
  Must NOT do: Do not translate evidence values, hashes, parser IDs, paths, or source IDs.

  Parallelization: Can parallel: YES | Wave 1 | Blocks: [5, 6, 7, 9] | Blocked by: []

  References:
  - Pattern:  `gui/evidence-viewer/app.js:1` - seed evidence records.
  - Pattern:  `gui/evidence-viewer/app.js:261` - KO/EN translation table.
  - Pattern:  `gui/evidence-viewer/app.js:645` - filter definitions.
  - Pattern:  `gui/evidence-viewer/app.js:1027` - bulk preview is already non-mutating.
  - Product:  `docs/EVIDENCE_VIEWER_GUI.md:95` - production row contract fields.
  - Product:  `docs/EVIDENCE_VIEWER_GUI.md:168` - viewer rules and derived artifact boundaries.
  - Design:   `opendesign/design-systems/frametrace-forensic-workstation/DESIGN.md:80` - voice and candidate wording.

  Acceptance criteria:
  - [ ] `node --check gui/evidence-viewer/app.js` passes.
  - [ ] At least one first-screen candidate row exposes `candidate-unvalidated` or KO equivalent in DOM text before opening a separate report.
  - [ ] Switching locale preserves raw paths, hashes, parser IDs, and candidate validation values.

  QA scenarios:
  ```
  Scenario: candidate state stays explicit
    Tool:     bash + playwright(real Chrome)
    Steps:    npm exec --yes --package=playwright@1.61.0 -- node -e 'const { chromium } = require("playwright"); const path=require("path"); (async()=>{const b=await chromium.launch({channel:"chrome",headless:true}); const p=await b.newPage({viewport:{width:1280,height:800}}); await p.goto("file://"+path.resolve("gui/evidence-viewer/index.html")); await p.waitForSelector("#fileRows .data-row"); await p.getByRole("tab", {name:/복구|Carved/}).click(); const body=await p.locator("body").innerText(); if(!/candidate-unvalidated|미검증 후보|후보/.test(body)) throw new Error("candidate state not explicit"); await p.screenshot({path:".omo/evidence/task-03-frametrace-evidence-viewer-first-screen-ia-heavy-candidate.png", fullPage:true}); await b.close();})();'
    Expected: Candidate state is visible with text, not color alone.
    Evidence: .omo/evidence/task-03-frametrace-evidence-viewer-first-screen-ia-heavy-candidate.png

  Scenario: locale preserves evidence values
    Tool:     bash + playwright(real Chrome)
    Steps:    npm exec --yes --package=playwright@1.61.0 -- node -e 'const { chromium } = require("playwright"); const path=require("path"); (async()=>{const b=await chromium.launch({channel:"chrome",headless:true}); const p=await b.newPage({viewport:{width:1280,height:800}}); await p.goto("file://"+path.resolve("gui/evidence-viewer/index.html")); await p.waitForSelector("#metaList"); const before=await p.locator("#metaList").innerText(); await p.locator("#languageButton").click(); const after=await p.locator("#metaList").innerText(); for (const v of ["vid_000001","blackvue_channel_suffix"]) if(!before.includes(v)||!after.includes(v)) throw new Error("evidence value changed or disappeared: "+v); await b.close();})();'
    Expected: Evidence IDs/parser IDs remain verbatim across locale switch.
    Evidence: .omo/evidence/task-03-frametrace-evidence-viewer-first-screen-ia-heavy-locale.txt
  ```

  Commit: YES | Message: `feat(gui): expose candidate validation vocabulary` | Files: [gui/evidence-viewer/app.js]

- [ ] 4. Responsive workstation CSS at 375/768/1280

  What to do: Update `gui/evidence-viewer/styles.css` so 1280 behaves like a dense desktop workstation, 768 behaves like a tablet/narrow workstation with no horizontal overflow, and 375 behaves as a stacked local review triage surface with key evidence state reachable in the first screen. Keep row heights stable and do not use viewport-scaled type.
  Must NOT do: Do not remove the desktop workstation layout to satisfy mobile; responsive behavior must adapt, not replace, the forensic surface.

  Parallelization: Can parallel: YES | Wave 1 | Blocks: [5, 6, 9] | Blocked by: []

  References:
  - Pattern:  `gui/evidence-viewer/styles.css:1` - current tokens.
  - Pattern:  `gui/evidence-viewer/styles.css:26` - current global body constraints.
  - Pattern:  `gui/evidence-viewer/styles.css:144` - current workspace grid.
  - Pattern:  `gui/evidence-viewer/styles.css:371` - inventory row grid.
  - Pattern:  `gui/evidence-viewer/styles.css:946` - current narrow desktop breakpoint.
  - Product:  `docs/EVIDENCE_VIEWER_GUI.md:87` - density targets.
  - Design:   `opendesign/design-systems/frametrace-forensic-workstation/DESIGN.md:25` - no viewport-scaled type.
  - Design:   `opendesign/design-systems/frametrace-forensic-workstation/DESIGN.md:36` - dense spacing and stable row height.

  Acceptance criteria:
  - [ ] Browser QA at 375, 768, and 1280 reports `document.documentElement.scrollWidth <= innerWidth + 1`.
  - [ ] At 1280, source, inventory, viewer, and inspector are visible without hiding validation/export/report context.
  - [ ] At 768 and 375, sources, candidates, validation, export, and report flow are visible or reachable through first-screen stacked sections without horizontal scrolling.
  - [ ] CSS scan finds no `vw` font-size scaling: `! rg -n "font-size:\\s*[^;]*vw" gui/evidence-viewer/styles.css`.

  QA scenarios:
  ```
  Scenario: responsive no-horizontal-overflow
    Tool:     bash + playwright(real Chrome)
    Steps:    npm exec --yes --package=playwright@1.61.0 -- node -e 'const { chromium } = require("playwright"); const fs=require("fs"), path=require("path"); (async()=>{fs.mkdirSync(".omo/evidence",{recursive:true}); const b=await chromium.launch({channel:"chrome",headless:true}); const p=await b.newPage(); const out=[]; for (const vp of [{w:375,h:812},{w:768,h:1024},{w:1280,h:800}]) { await p.setViewportSize({width:vp.w,height:vp.h}); await p.goto("file://"+path.resolve("gui/evidence-viewer/index.html")); await p.waitForSelector("#fileRows .data-row"); const m=await p.evaluate(()=>({innerWidth,scrollWidth:document.documentElement.scrollWidth, text:document.body.innerText.slice(0,5000)})); await p.screenshot({path:`.omo/evidence/task-04-frametrace-evidence-viewer-first-screen-ia-heavy-${vp.w}.png`,fullPage:true}); if (m.scrollWidth > m.innerWidth + 1) throw new Error(`overflow ${vp.w}: ${m.scrollWidth} > ${m.innerWidth}`); out.push({vp,...m,text:undefined}); } fs.writeFileSync(".omo/evidence/task-04-frametrace-evidence-viewer-first-screen-ia-heavy-responsive.json", JSON.stringify(out,null,2)); await b.close();})();'
    Expected: Command exits 0 and writes screenshots for all three viewport widths.
    Evidence: .omo/evidence/task-04-frametrace-evidence-viewer-first-screen-ia-heavy-responsive.json

  Scenario: typography guardrail
    Tool:     bash
    Steps:    ! rg -n "font-size:\\s*[^;]*vw" gui/evidence-viewer/styles.css
    Expected: Command exits 0; no viewport-scaled type exists.
    Evidence: .omo/evidence/task-04-frametrace-evidence-viewer-first-screen-ia-heavy-css-scan.txt
  ```

  Commit: YES | Message: `feat(gui): make evidence viewer responsive` | Files: [gui/evidence-viewer/styles.css]

- [ ] 5. Evidence source and queue rendering

  What to do: Strengthen the rendered source/queue panes so the first screen distinguishes mounted SD card, NVR export, E01/raw image, recovered filesystem, derived outputs, unreviewed, important, report-selected, exported, and validation-risk items. Counts must be based on the current mock records and filters, not hardcoded copy.
  Must NOT do: Do not add fake source types that cannot be traced to existing mock records or documented product flows.

  Parallelization: Can parallel: NO in one worktree | Wave 2 | Blocks: [9] | Blocked by: [3, 4]

  References:
  - Pattern:  `gui/evidence-viewer/app.js:869` - current source rendering.
  - Pattern:  `gui/evidence-viewer/app.js:905` - current queue rendering.
  - Product:  `README.md:7` - case folder and source indexing promises.
  - Product:  `README.md:15` - E01 evidence container support.
  - Product:  `docs/EVIDENCE_VIEWER_GUI.md:27` - source tree count requirements.
  - Checklist:`opendesign/design-systems/frametrace-forensic-workstation/review_checklist.md:5` - source/path/parser/validation/hash visibility.

  Acceptance criteria:
  - [ ] Source pane includes at least four distinct source categories from mock records and each has a count.
  - [ ] Queue pane includes validation-risk/candidate and report/export flow counts.
  - [ ] Source and queue keyboard activation still works with Enter and Space.

  QA scenarios:
  ```
  Scenario: source and queue counts visible
    Tool:     bash + playwright(real Chrome)
    Steps:    npm exec --yes --package=playwright@1.61.0 -- node -e 'const { chromium } = require("playwright"); const path=require("path"); (async()=>{const b=await chromium.launch({channel:"chrome",headless:true}); const p=await b.newPage({viewport:{width:1280,height:800}}); await p.goto("file://"+path.resolve("gui/evidence-viewer/index.html")); await p.waitForSelector("#sourceList"); const src=await p.locator("#sourceList").innerText(); const q=await p.locator("#queueList").innerText(); if((src.match(/\\d+/g)||[]).length < 4) throw new Error("too few source counts"); if(!/검증|Verify|후보|Candidate|보고서|Report/.test(q)) throw new Error("queue lacks validation/report flow"); await p.screenshot({path:".omo/evidence/task-05-frametrace-evidence-viewer-first-screen-ia-heavy-sources.png", fullPage:true}); await b.close();})();'
    Expected: Source and queue counts are visible in the first screen.
    Evidence: .omo/evidence/task-05-frametrace-evidence-viewer-first-screen-ia-heavy-sources.png

  Scenario: keyboard queue activation
    Tool:     bash + playwright(real Chrome)
    Steps:    npm exec --yes --package=playwright@1.61.0 -- node -e 'const { chromium } = require("playwright"); const path=require("path"); (async()=>{const b=await chromium.launch({channel:"chrome",headless:true}); const p=await b.newPage({viewport:{width:1280,height:800}}); await p.goto("file://"+path.resolve("gui/evidence-viewer/index.html")); await p.waitForSelector(".queue-item"); await p.locator(".queue-item").last().focus(); await p.keyboard.press("Enter"); const pressed=await p.locator(".queue-item").last().getAttribute("aria-pressed"); if(pressed!=="true") throw new Error("queue keyboard activation failed"); await b.close();})();'
    Expected: Last queue item becomes active via keyboard.
    Evidence: .omo/evidence/task-05-frametrace-evidence-viewer-first-screen-ia-heavy-keyboard.txt
  ```

  Commit: YES | Message: `feat(gui): surface evidence source queues` | Files: [gui/evidence-viewer/app.js]

- [ ] 6. Video/candidate inventory and validation state rendering

  What to do: Make the inventory first screen expose the required forensic columns or equivalents: status, review state, file ID, name/path, source, type/parser lane, validation state, timestamp, size, hash state, and report flag. At narrow widths, preserve this information through row hierarchy/detail/inspector without hiding validation or source provenance.
  Must NOT do: Do not render all 10,000 rows into the DOM. Do not bulk-render thumbnails.

  Parallelization: Can parallel: NO in one worktree | Wave 2 | Blocks: [9] | Blocked by: [3, 4]

  References:
  - Pattern:  `gui/evidence-viewer/app.js:948` - current virtualized rendering.
  - Pattern:  `gui/evidence-viewer/app.js:997` - current row HTML.
  - Pattern:  `gui/evidence-viewer/styles.css:371` - current file row grid.
  - Product:  `docs/EVIDENCE_VIEWER_GUI.md:33` - required grid columns.
  - Product:  `docs/EVIDENCE_VIEWER_GUI.md:74` - large-case interaction rules.
  - Trace:    `docs/gui-large-inventory-traceability.md:19` - visible row/count acceptance.

  Acceptance criteria:
  - [ ] DOM row count stays bounded: `.data-row` count is less than 80 for 10,000 mock records at 1280 after scrolling.
  - [ ] At 1280, first-screen row or inspector text contains source, parser, validation, hash, and report state.
  - [ ] Candidate filter shows candidate rows without changing them to verified.

  QA scenarios:
  ```
  Scenario: bounded DOM and forensic columns
    Tool:     bash + playwright(real Chrome)
    Steps:    npm exec --yes --package=playwright@1.61.0 -- node -e 'const { chromium } = require("playwright"); const path=require("path"); (async()=>{const b=await chromium.launch({channel:"chrome",headless:true}); const p=await b.newPage({viewport:{width:1280,height:800}}); await p.goto("file://"+path.resolve("gui/evidence-viewer/index.html")); await p.waitForSelector("#fileRows .data-row"); await p.locator("#fileRows").evaluate(el=>{el.scrollTop=5000; el.dispatchEvent(new Event("scroll"));}); await p.waitForTimeout(100); const count=await p.locator("#fileRows .data-row").count(); const text=await p.locator("body").innerText(); if(count>=80) throw new Error("unbounded DOM rows: "+count); for (const s of ["vid_","blackvue","검증","해시"]) if(!text.toLowerCase().includes(s.toLowerCase())) throw new Error("missing forensic column text "+s); await p.screenshot({path:".omo/evidence/task-06-frametrace-evidence-viewer-first-screen-ia-heavy-inventory.png", fullPage:true}); await b.close();})();'
    Expected: DOM row count remains bounded and forensic state text is visible.
    Evidence: .omo/evidence/task-06-frametrace-evidence-viewer-first-screen-ia-heavy-inventory.png

  Scenario: candidate not verified by filter
    Tool:     bash + playwright(real Chrome)
    Steps:    npm exec --yes --package=playwright@1.61.0 -- node -e 'const { chromium } = require("playwright"); const path=require("path"); (async()=>{const b=await chromium.launch({channel:"chrome",headless:true}); const p=await b.newPage({viewport:{width:1280,height:800}}); await p.goto("file://"+path.resolve("gui/evidence-viewer/index.html")); await p.waitForSelector("#filterTabs"); await p.getByRole("tab", {name:/복구|Carved/}).click(); const body=await p.locator("body").innerText(); if(/verified playable/.test(body) && /candidate-unvalidated/.test(body) === false) throw new Error("candidate state softened"); await b.close();})();'
    Expected: Candidate view keeps candidate/unvalidated wording.
    Evidence: .omo/evidence/task-06-frametrace-evidence-viewer-first-screen-ia-heavy-candidate-filter.txt
  ```

  Commit: YES | Message: `feat(gui): show validation state in inventory` | Files: [gui/evidence-viewer/app.js, gui/evidence-viewer/styles.css]

- [ ] 7. Export/report/validation flow affordances

  What to do: Make export, report, validation, frame capture, and package actions read as forensic workflow steps. The first screen must show that MP4/AVI/frame outputs are derived artifacts, report inclusion is separate from validation, validation is required for candidates, and package/report flow is not release hype.
  Must NOT do: Do not add durable mutation semantics unless backed by existing Rust engine command/audit flow.

  Parallelization: Can parallel: NO in one worktree | Wave 2 | Blocks: [9] | Blocked by: [2, 3]

  References:
  - Pattern:  `gui/evidence-viewer/index.html:174` - current output queue buttons.
  - Pattern:  `gui/evidence-viewer/app.js:1027` - bulk preview.
  - Pattern:  `gui/evidence-viewer/app.js:1652` - current export handlers.
  - Product:  `README.md:72` - `make-review` flow.
  - Product:  `README.md:75` - `export-video` flow.
  - Product:  `README.md:77` - `make-report` flow.
  - Product:  `docs/WINDOWS_USAGE.md:129` - export client deliverables.
  - Product:  `docs/WINDOWS_USAGE.md:143` - derived artifact logs include hashes and command arguments.
  - External: `https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/command-bar` - command action grouping guardrail.

  Acceptance criteria:
  - [ ] First screen includes export/report/validation wording without implying validation completion.
  - [ ] Clicking MP4, AVI, frame, verify, report, and package actions produces visible queued/preview state or session activity.
  - [ ] Candidate verify action leaves candidate status unchanged unless an engine validation command is introduced by Task 8.

  QA scenarios:
  ```
  Scenario: queued derived-output flow
    Tool:     bash + playwright(real Chrome)
    Steps:    npm exec --yes --package=playwright@1.61.0 -- node -e 'const { chromium } = require("playwright"); const path=require("path"); (async()=>{const b=await chromium.launch({channel:"chrome",headless:true}); const p=await b.newPage({viewport:{width:1280,height:800}}); await p.goto("file://"+path.resolve("gui/evidence-viewer/index.html")); await p.waitForSelector("#exportMp4Button"); for (const id of ["#exportMp4Button","#exportAviButton","#captureFrameButton","#verifyButton","#addReportButton","#packageButton"]) await p.locator(id).click(); const body=await p.locator("body").innerText(); if(!/대기|queued|보고서|Report|MP4|AVI/.test(body)) throw new Error("queued flow not visible"); await p.screenshot({path:".omo/evidence/task-07-frametrace-evidence-viewer-first-screen-ia-heavy-flow.png", fullPage:true}); await b.close();})();'
    Expected: Derived-output/report/validation actions produce visible queued or preview state.
    Evidence: .omo/evidence/task-07-frametrace-evidence-viewer-first-screen-ia-heavy-flow.png

  Scenario: no completion wording
    Tool:     bash
    Steps:    ! rg -n "export complete|validated by UI|report finalized|검증 완료.*UI|내보내기 완료" gui/evidence-viewer/index.html gui/evidence-viewer/app.js
    Expected: Command exits 0; prototype controls do not claim durable completion.
    Evidence: .omo/evidence/task-07-frametrace-evidence-viewer-first-screen-ia-heavy-no-completion.txt
  ```

  Commit: YES | Message: `feat(gui): clarify export report validation flow` | Files: [gui/evidence-viewer/index.html, gui/evidence-viewer/app.js]

- [ ] 8. Conditional CLI/generated-review parity audit

  What to do: Audit whether the first-screen IA needs any Rust/generated review parity. Default outcome should be no Rust change. Only edit Rust if a generated `review/evidence-viewer.html` or command contract contradicts the GUI IA, and keep changes minimal. If Rust changes are needed, likely files are `src/cli/commands.rs`, `src/cli/mod.rs`, `src/cli/inventory_cmd.rs`, `src/cli/handlers.rs`, `tests/cli_inventory.rs`, and `tests/cli_review.rs`.
  Must NOT do: Do not change `src/cli/commands.rs` merely to support prototype copy. Do not widen CLI behavior without tests.

  Parallelization: Can parallel: YES for audit; NO for any Rust edit | Wave 1 | Blocks: [9] | Blocked by: []

  References:
  - API/Type: `src/cli/commands.rs:11` - command enum surface.
  - API/Type: `src/cli/commands.rs:251` - inventory command.
  - API/Type: `src/cli/commands.rs:271` - bulk preview command.
  - API/Type: `src/cli/commands.rs:282` - inventory export manifest command.
  - Pattern:  `src/cli/mod.rs:267` - inventory commands routed to handler.
  - Pattern:  `src/cli/inventory_cmd.rs:99` - inventory list/search/facet/detail execution.
  - Test:     `tests/cli_inventory.rs:52` - bounded SQLite JSON and export-manifest coverage.
  - Test:     `tests/cli_review.rs:34` - generated review HTML bounds.
  - Product:  `docs/EVIDENCE_VIEWER_GUI.md:133` - current CLI JSON contract.

  Acceptance criteria:
  - [ ] Evidence records either `rust:not-applicable` with exact reason, or lists the Rust files changed and matching tests.
  - [ ] If no Rust changed: `cargo test --locked --test cli_review --test cli_inventory -- --nocapture` passes or its failure is captured as pre-existing with stdout/stderr.
  - [ ] If `src/cli/commands.rs` changes: run the full command-surface range from this task's QA scenario and capture output.

  QA scenarios:
  ```
  Scenario: no-Rust parity audit
    Tool:     bash
    Steps:    { rg -n "candidate-unvalidated|inventory-bulk-preview|inventory-export-manifest|review/evidence-viewer.html" src tests docs README.md; printf '\nrust:not-applicable unless IA contract drift is found\n'; } > .omo/evidence/task-08-frametrace-evidence-viewer-first-screen-ia-heavy-rust-audit.txt
    Expected: Audit file exists and supports either no Rust changes or a specific Rust edit decision.
    Evidence: .omo/evidence/task-08-frametrace-evidence-viewer-first-screen-ia-heavy-rust-audit.txt

  Scenario: cargo feasible range
    Tool:     bash
    Steps:    cargo test --locked --test cli_review --test cli_inventory -- --nocapture 2>&1 | tee .omo/evidence/task-08-frametrace-evidence-viewer-first-screen-ia-heavy-cargo-review-inventory.txt
    Expected: Command exits 0 if Rust is unaffected; if it fails because of pre-existing dirty worktree issues, output is captured and the executor must not claim Rust parity.
    Evidence: .omo/evidence/task-08-frametrace-evidence-viewer-first-screen-ia-heavy-cargo-review-inventory.txt
  ```

  Commit: YES if Rust changed, otherwise NO | Message: `test(cli): preserve evidence viewer contract` | Files: [src/cli/commands.rs, src/cli/mod.rs, src/cli/inventory_cmd.rs, src/cli/handlers.rs, tests/cli_inventory.rs, tests/cli_review.rs]

- [ ] 9. Browser QA, visual/reviewer gate, and cleanup receipts

  What to do: Rerun the first-screen browser assertions at 375, 768, and 1280; run syntax and feasible cargo checks; run visual QA/reviewer gate; save cleanup receipts. The final review must reject the work if evidence sources, candidates, validation state, export, and report flow are not visible/reachable at each viewport.
  Must NOT do: Do not claim completion from screenshots alone; DOM assertions, syntax checks, cargo output, and cleanup receipts are required.

  Parallelization: Can parallel: final verifiers YES after implementation | Wave 3 | Blocks: [final] | Blocked by: [1, 2, 3, 4, 5, 6, 7, 8]

  References:
  - Pattern:  `docs/recovery-test-spec.md:16` - required command pattern includes cargo and node checks.
  - Pattern:  `scripts/windows/validate-release.ps1:117` - Windows validation includes viewer JavaScript syntax check.
  - Checklist:`opendesign/design-systems/frametrace-forensic-workstation/review_checklist.md:5` - forensic correctness checklist.
  - Checklist:`opendesign/design-systems/frametrace-forensic-workstation/review_checklist.md:14` - large-case usability checklist.
  - Checklist:`opendesign/design-systems/frametrace-forensic-workstation/review_checklist.md:22` - viewer quality checklist.
  - External: `https://playwright.dev/docs/locators` - role/text locator checks.

  Acceptance criteria:
  - [ ] `.omo/evidence/task-09-frametrace-evidence-viewer-first-screen-ia-heavy-final-browser.json` shows pass at 375, 768, and 1280.
  - [ ] Screenshots exist for 375, 768, and 1280 after implementation.
  - [ ] `node --check gui/evidence-viewer/app.js` passes.
  - [ ] `cargo test --locked --test cli_review --test cli_inventory -- --nocapture` passes if Rust was untouched; if Rust changed, the full command-surface range passes.
  - [ ] Visual QA/reviewer artifacts exist and include APPROVE/ITERATE/REJECT. ITERATE/REJECT blocks completion.
  - [ ] Cleanup receipt records no persistent browser/server/process left running and no unrelated dirty files reverted.

  QA scenarios:
  ```
  Scenario: final browser IA pass at 375/768/1280
    Tool:     bash + playwright(real Chrome)
    Steps:    npm exec --yes --package=playwright@1.61.0 -- node <<'NODE'
              const { chromium } = require("playwright");
              const fs = require("fs");
              const path = require("path");
              fs.mkdirSync(".omo/evidence", { recursive: true });
              (async () => {
                const browser = await chromium.launch({ channel: "chrome", headless: true });
                const page = await browser.newPage();
                const fileUrl = "file://" + path.resolve("gui/evidence-viewer/index.html");
                const out = [];
                for (const vp of [{w:375,h:812},{w:768,h:1024},{w:1280,h:800}]) {
                  await page.setViewportSize({ width: vp.w, height: vp.h });
                  await page.goto(fileUrl);
                  await page.waitForSelector("#fileRows .data-row");
                  const m = await page.evaluate(() => {
                    const text = document.body.innerText;
                    return {
                      innerWidth,
                      scrollWidth: document.documentElement.scrollWidth,
                      sources: /증거|Evidence sources|Evidence/.test(text),
                      candidates: /candidate-unvalidated|미검증 후보|후보|Candidate/.test(text),
                      validation: /검증|Validation|Verify/.test(text),
                      exportFlow: /MP4|AVI|내보내기|export/i.test(text),
                      reportFlow: /보고서|Report|Package|패키지/.test(text),
                      rowCount: document.querySelectorAll("#fileRows .data-row").length,
                    };
                  });
                  m.pass = m.scrollWidth <= m.innerWidth + 1 && m.sources && m.candidates && m.validation && m.exportFlow && m.reportFlow && m.rowCount > 0 && m.rowCount < 80;
                  out.push({ viewport: vp, ...m });
                  await page.screenshot({ path: `.omo/evidence/task-09-frametrace-evidence-viewer-first-screen-ia-heavy-${vp.w}.png`, fullPage: true });
                }
                await browser.close();
                fs.writeFileSync(".omo/evidence/task-09-frametrace-evidence-viewer-first-screen-ia-heavy-final-browser.json", JSON.stringify(out, null, 2));
                const failed = out.filter(x => !x.pass);
                if (failed.length) throw new Error("first-screen IA failed: " + JSON.stringify(failed));
              })().catch((error) => { console.error(error); process.exit(1); });
              NODE
    Expected: Command exits 0; JSON has `pass: true` for 375, 768, and 1280.
    Evidence: .omo/evidence/task-09-frametrace-evidence-viewer-first-screen-ia-heavy-final-browser.json

  Scenario: final command and cleanup gate
    Tool:     bash
    Steps:    { node --check gui/evidence-viewer/app.js; cargo test --locked --test cli_review --test cli_inventory -- --nocapture; git status --short; printf '\ncleanup: no persistent server required; Playwright browser closed by script; evidence retained under .omo/evidence.\n'; } 2>&1 | tee .omo/evidence/task-09-frametrace-evidence-viewer-first-screen-ia-heavy-final-command-cleanup.txt
    Expected: Node and cargo commands pass, or any failure is captured and blocks final approval; cleanup receipt is present.
    Evidence: .omo/evidence/task-09-frametrace-evidence-viewer-first-screen-ia-heavy-final-command-cleanup.txt
  ```

  Commit: YES | Message: `test(gui): verify first-screen evidence IA` | Files: [.omo/evidence/*]

## Final verification wave (MANDATORY - after all implementation tasks)
> Runs in PARALLEL. ALL must APPROVE. Surface results to the caller and wait for an explicit "okay" before declaring complete.
- [ ] F1. Plan compliance audit - every task done, every acceptance criterion met
- [ ] F2. Code quality review - diagnostics clean, idioms match, no dead code
- [ ] F3. Real manual QA - every QA scenario executed with evidence captured
- [ ] F4. Scope fidelity - nothing extra shipped beyond Must-Have, nothing Must-NOT-Have introduced

## Commit strategy
- One logical change per commit. Conventional Commits (`<type>(<scope>): <subject>`) plus the repository Lore trailers in the commit body.
- Atomic: every commit builds and passes its relevant checks on its own.
- No "WIP" / "fix typo squash later" commits on the final branch - clean up before merge.
- Reference the plan file path in the final commit footer: `Plan: .omo/plans/frametrace-evidence-viewer-first-screen-ia-heavy.md`.

## Success criteria
- All Must-Have shipped; all QA scenarios pass with captured evidence; F1-F4 approved; commit history clean.
