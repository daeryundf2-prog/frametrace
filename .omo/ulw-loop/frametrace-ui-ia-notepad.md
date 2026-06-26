# FrameTrace evidence-viewer ULW notepad

- skills: omo:ulw-loop because user explicitly requested evidence-led durable execution.
- skills: omo:frontend because gui/evidence-viewer IA/UX/UI is the primary surface.
- skills: omo:visual-qa because UI changed and screenshots at 375/768/1280 are required.
- skills: playwright because browser surface QA is required.
- tier: HEAVY because user explicitly asked to review IA/UX/UI and verify multiple browser breakpoints; change affects a full first-screen workflow surface.
- shape: delivery.
- success C1: evidence-viewer first screen shows evidence source, video candidate, validation, export, and report flow at a glance; Playwright screenshot/DOM assertions at 375/768/1280.
- success C2: visible copy uses local-first/status-honest forensic language, including verification needed and candidate-unvalidated; no launch hype.
- success C3: focused cargo test range passes or pre-existing blockers are identified without reverting dirty worktree.
- manual QA: Playwright opens gui/evidence-viewer/index.html via file:// or local static server, captures screenshots under .omo/ulw-loop/evidence/frametrace-ui-ia/.
- adversarial: stale_state from static assets/cache; dirty_worktree from existing modified files; misleading_success_output from visual claims without browser artifacts; malformed_input not triggered unless code adds input parsing; prompt_injection not triggered; cancel/resume not triggered; hung_long_commands from cargo/browser commands; flaky_tests from browser viewport assertions; repeated_interruptions not triggered.
