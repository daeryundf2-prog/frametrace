# FrameTrace UI IA Code Review

codeQualityStatus: WATCH
recommendation: APPROVE
blockers: none

## Scope Reviewed

- `gui/evidence-viewer/index.html`
- `gui/evidence-viewer/app.js`
- `gui/evidence-viewer/styles.css`
- `DESIGN.md`
- `.omo/ulw-loop/evidence/frametrace-ui-ia/*`

Unrelated dirty worktree files outside the requested scope were not reviewed or reverted.

## Skill-Perspective Check

- `remove-ai-slops`: ran by loading `/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/remove-ai-slops/SKILL.md`. No deletion-only tests, tautological tests, or production slop patterns were found in the scoped UI diff. Browser evidence is acceptance-relevant, but has the LOW coverage gap noted below.
- `programming`: ran by loading `/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/programming/SKILL.md`. The scoped implementation does not add parser/validation complexity, typed escape hatches, or needless abstractions. The design-source mismatch below is the main maintainability issue from this perspective.
- `code-review` skill was also loaded. Its suggested parallel subagent lanes were not used because the current tool policy only allows spawning subagents when the user explicitly requests subagents or delegation; this direct read-only reviewer pass was completed instead.

## Findings

### CRITICAL

- None.

### HIGH

- None.

### MEDIUM

- `DESIGN.md` is now inconsistent with the responsive implementation and the review acceptance evidence. It still documents a fixed-width workstation contract at `DESIGN.md:89` through `DESIGN.md:94`, including `Body minimum width: 1260px`, while the current CSS removes a body min-width and hides horizontal overflow at `gui/evidence-viewer/styles.css:26` through `gui/evidence-viewer/styles.css:30`, then switches the workspace to one column at `gui/evidence-viewer/styles.css:1175` through `gui/evidence-viewer/styles.css:1210`. The same design source also names the pending validation label as `verification-needed` at `DESIGN.md:5`, while the implemented and requested first-screen state is `verification required` at `gui/evidence-viewer/index.html:84` and `gui/evidence-viewer/app.js:285`. This is not a runtime blocker, but it can mislead future UI work back toward horizontal overflow or inconsistent verification wording.

### LOW

- `gui/evidence-viewer/styles.css:1046` references `var(--accent-strong)`, but the root token block at `gui/evidence-viewer/styles.css:1` through `gui/evidence-viewer/styles.css:20` defines `--accent` and `--accent-2`, not `--accent-strong`. A queued decision-gate row therefore loses the intended queued-state color. The text label remains explicit, so this is not blocking.

- `.omo/ulw-loop/evidence/frametrace-ui-ia/playwright-assertions.json` confirms the requested visible concepts and horizontal-overflow checks, but it would not fail on a JavaScript page error if the static HTML concepts still render. Future browser QA should also fail on `pageerror`/unexpected console errors and assert dynamic render targets such as `#sourceList .source-item`, `#fileRows .file-row`, and `.decision-gate-row`.

## Evidence Reviewed

- `node --check gui/evidence-viewer/app.js` passed locally.
- `.omo/ulw-loop/evidence/frametrace-ui-ia/cargo-test-lib.txt` reports `159 passed; 0 failed`.
- `.omo/ulw-loop/evidence/frametrace-ui-ia/playwright-assertions.json` reports no failures. Required concepts are visible at `desktop-1280`, `tablet-768`, and `mobile-375`; overflow checks report `scrollWidth == bodyScrollWidth == viewportWidth` for all three viewports.
- Latest screenshots inspected:
  - `.omo/ulw-loop/evidence/frametrace-ui-ia/desktop-1280.png`
  - `.omo/ulw-loop/evidence/frametrace-ui-ia/tablet-768.png`
  - `.omo/ulw-loop/evidence/frametrace-ui-ia/mobile-375.png`
- The old tablet status badge clipping is not visible in the latest tablet screenshot. The mobile screenshot shows the workflow steps, local-first / verification required / candidate-unvalidated states, workbench summary cards, filters, and inventory rows in the first screen with no horizontal overflow.
- `.omo/ulw-loop/evidence/frametrace-ui-ia/visual-diff/*.json` artifacts show matching dimensions and intact alpha channels; the large pixel diffs are expected because the first-screen IA changed substantially from baseline.
