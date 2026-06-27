# Remaining UI Backlog

## P1

- Replace static generated sample thumbnails with real case thumbnail/proxy availability indicators when the Rust pipeline exposes those assets in the static GUI fixture.
- Add an explicit mobile jump control from the viewer rail to the first candidate row after the browser pane if field reviewers need faster triage on 375px screens.
- Consider a denser 1280 layout preset where the candidate browser receives more width without hiding inspector state.

## P2

- Add automated screenshot regression checks for `gui/evidence-viewer` at 375, 768, and 1280 widths.
- Add a non-cache query helper for local dev only instead of hard-coded static asset version strings.
- Review Korean microcopy for consistency between `검증 대기`, `검증 필요`, and `검토자 확인`.

## Known Constraints

- Headed agbrowse/Chrome on this macOS session would not resize below a 500px viewport, so exact 375px evidence was captured with bundled Playwright as supplemental browser QA.
- Generated `review/evidence-viewer.html` now has narrow-viewport browser evidence, but it remains a simpler static review page than the richer `gui/evidence-viewer` workbench.
- Native Windows QA was not run in this macOS session; Windows-specific validation remains a separate gate.
