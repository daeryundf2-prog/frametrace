# Gate Review: frametrace-agbrowse-ui-20260626

## recommendation

REJECT

## gateReview

FAIL

## criteriaCoverage

PASS

## blockers

- `gui/evidence-viewer/app.js` and `gui/evidence-viewer/styles.css` remain unresolved programming/remove-ai-slops blockers. Direct measurement found `app.js` at 1862 pure LOC and `styles.css` at 1363 pure LOC. The scoped GUI patch added substantial code to both oversized files, which violates the required 250 pure-LOC ceiling and creates maintenance burden.
- Hidden/dead UI remains unresolved: `gui/evidence-viewer/index.html:45` adds `.workflow-band`, while `gui/evidence-viewer/styles.css:1291` hides `.browser-pane > .workflow-band`. The visible workflow is carried by `.workbench-flow` and `#selectedEvidenceRail`, so the hidden band is dead/ambiguous production UI.
- No matching `frametrace-agbrowse-ui` notepad artifact was found. The ledger exists and is usable, but the required notepad path is an evidence gap.
- `.omo/ulw-loop/frametrace-agbrowse-ui-20260626/goals.json` still shows the session goal as `in_progress`; `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/quality-gate.json` still has `gateReview.status: pending` and `allowedToCheckpoint: false`.

## originalIntent

Improve the FrameTrace `gui/evidence-viewer` first screen so a reviewer lands on local video evidence review first, with source evidence, video candidates, candidate-unvalidated/local-first/verification-required states, export previews, and report packaging flow visible and testable.

## desiredOutcome

The shipped GUI should prioritize video evidence review on first load, expose source/candidate/status/report/export controls across desktop, tablet, and 375px mobile widths, and be backed by headed `agbrowse` evidence plus cargo/node regression checks.

## userOutcomeReview

The user-visible outcome is substantially satisfied. Direct inspection of the headed browser artifacts shows:

- Desktop evidence `desktop-1280.png`, `desktop-1280-overflow.json`, and `desktop-1280-rail-actions.json` show the video viewer first, selected source evidence rail, video candidates, candidate-unvalidated status, verification-required statuses, export preview, report set, package preview, and action-state changes.
- Mobile evidence `mobile-375.png`, `mobile-375-scrolled.png`, `mobile-375-report.png`, and `mobile-375-inspector.png` shows the same workflow reachable at true 375 CSS width, including report controls and export/verification controls after scroll.
- Tablet evidence `tablet-768.png`, `tablet-768-overflow.json`, `tablet-rail-clip.json`, and `tablet-rail-geometry.json` shows no horizontal overflow and keeps the selected evidence rail visible.
- Console artifacts for desktop, tablet, mobile, action flow, report flow, and inspector flow all report no console output captured.
- `node --check gui/evidence-viewer/app.js` was rerun during this gate and exited 0.
- `git diff --check -- gui/evidence-viewer/index.html gui/evidence-viewer/styles.css gui/evidence-viewer/app.js` was rerun during this gate and exited 0.
- Stored `cargo-test-lib.txt` reports `cargo test --lib` passed with 160 passed and 0 failed.
- Cleanup evidence reports `agbrowse` stopped and port 4181 free; a direct port check found no listener on 4181.

Despite that, final gate cannot approve because unresolved slop/maintainability blockers remain under the required `omo:remove-ai-slops` and `omo:programming` criteria.

## checkedArtifactPaths

- `.omo/ulw-loop/frametrace-agbrowse-ui-20260626/goals.json`
- `.omo/ulw-loop/frametrace-agbrowse-ui-20260626/ledger.jsonl`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/quality-gate.json`
- `.omo/evidence/frametrace-agbrowse-ui-code-review.md`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/gui-diff.patch`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/static-assertions.json`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/node-check-app.txt`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/cargo-test-lib.txt`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/git-diff-check-gui.txt`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/desktop-1280.png`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/desktop-1280-snapshot.txt`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/desktop-1280-console.txt`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/desktop-1280-overflow.json`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/desktop-1280-rail-actions.json`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/mobile-375.png`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/mobile-375-scrolled.png`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/mobile-375-report.png`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/mobile-375-inspector.png`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/mobile-375-overflow.json`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/mobile-375-report-overflow.json`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/mobile-375-inspector-overflow.json`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/mobile-375-scrolled-overflow.json`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/tablet-768.png`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/tablet-768-overflow.json`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/tablet-rail-clip.json`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/tablet-rail-geometry.json`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/cleanup-receipt.txt`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/cleanup-agbrowse-status-after-stop.txt`
- `.omo/ulw-loop/evidence/frametrace-agbrowse-ui/cleanup-port-4181-after-stop.txt`

## exactEvidenceGaps

- Required notepad path/artifact for this session was not found.
- Quality gate artifact still records gate review as pending and checkpoint disallowed.
- Goal status remains `in_progress`.
- Static assertion evidence is useful but mostly selector/text-presence based; the actual user-outcome confidence comes from the headed browser screenshots, overflow JSON, action JSON, and console artifacts.

## slopAndProgrammingPass

Direct `omo:remove-ai-slops`/`omo:programming` pass:

- No deletion-only tests, requested-removal-only tests, or tautological test removals were found in the scoped GUI evidence.
- `static-assertions.json` is narrow and selector-oriented, but it is supplemental to browser evidence rather than the only proof.
- Blocking slop remains in production code: oversized files and hidden `.workflow-band` markup/styles.
- The code review report does include skill-perspective coverage, but it downgrades the same unresolved issues to WATCH. Under final-gate rules, unresolved slop cannot be approved.

## remainingRisks

- Future GUI work will be difficult to review safely until `app.js` and `styles.css` are split by responsibility.
- Hidden `.workflow-band` may confuse future maintainers about the authoritative workflow UI.
- The broader worktree is dirty with many unrelated Rust/docs/test changes; this gate reviewed only the scoped GUI session artifacts and patch.

## updateGoalCheckpointAllowed

No. `update_goal`/checkpoint should remain blocked until the oversized-file and hidden/dead UI blockers are resolved, the missing notepad evidence gap is addressed or explicitly waived, and the session gate status is updated from pending to pass.
