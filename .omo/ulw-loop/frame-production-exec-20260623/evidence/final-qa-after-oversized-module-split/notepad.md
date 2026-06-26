ULW final QA after oversized module split

Objective: read-only final QA verification for FrameTrace current HEAD after c74e897.

Skills:
- omo:ulw-loop: used for evidence-bound QA, cleanup receipt, and final report discipline.
- omo:programming: relevant because requested gates exercise Rust and JavaScript toolchains; no production code edits are in scope.

Tier: HEAVY.
Justification: final QA after a module split, with explicit emphasis on macOS-compatible gates and a requested review artifact.

Success criteria:
- SC1: all requested macOS-compatible gates pass on current HEAD: cargo fmt check, cargo clippy locked all targets/features with warnings denied, cargo test locked, node syntax check for gui/evidence-viewer/app.js, and git diff whitespace check.
- SC2: cleanup receipt records that no browser/server/worker remains, or that cleanup is not applicable because none were started.

Scenarios:
- `cargo fmt --all -- --check` -> PASS if exit status is 0.
- `cargo clippy --locked --all-targets --all-features -- -D warnings` -> PASS if exit status is 0.
- `cargo test --locked` -> PASS if exit status is 0.
- `node --check gui/evidence-viewer/app.js` -> PASS if exit status is 0.
- `git diff --check` -> PASS if exit status is 0.
- Cleanup check via `pgrep -fl` for known FrameTrace/browser/server/worker patterns and `lsof` for repo-local Node/Rust processes -> PASS if no QA-started process exists; not applicable if this QA did not start any browser/server/worker.
