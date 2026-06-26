FrameTrace agbrowse UI hardening note

- Scope: gui/evidence-viewer only.
- User outcome: make the first screen behave like a local-first forensic video evidence review workstation.
- Visible flow: selected video evidence first, source evidence rail, candidate-unvalidated status, validation-required gate, export preview, report/package controls.
- QA: headed agbrowse screenshots and overflow JSON for 1280 desktop, 768 tablet, and true 375 CSS px mobile.
- Constraints: unrelated dirty worktree preserved; Rust engine unchanged.
