# FrameTrace Phase 3 Media Validation Brief

Status: PASS for implemented Phase 3 slice.
Checked at: 2026-06-17T03:43:07Z.

Implemented:
- Engine-side validation provenance for `validate-artifact`.
- Required operator resolution for media validation and durable media outputs.
- Derived artifact provenance for export, proxy, thumbnail, and frame capture.
- `capture-frame` command and `artifacts/frames/frame-log.jsonl`.
- Source/output path safety guard for media outputs.
- Report and generated evidence viewer disclosure of source IDs, derived IDs, hashes, operator, method, and audit chain.
- Generated viewer no longer auto-loads original media; it shows a manual-open link and preserves original evidence boundaries.

Primary verification evidence:
- `evidence/red-cargo-test-media-contract.txt`
- `evidence/final-cargo-test.txt`
- `evidence/final-cargo-clippy.txt`
- `evidence/final-fmt-check.txt`
- `evidence/final-git-diff-check.txt`
- `evidence/real-surface-cli-flow.txt`
- `evidence/playwright-generated-html-clean.txt`
- `evidence/cleanup-receipt.txt`

Known limitations:
- LSP diagnostics could not run because `rust-analyzer` is missing from toolchain `1.94.0-aarch64-apple-darwin`.
- `npm test` is not applicable because this repository currently has no `package.json`.
- Several inherited/pre-existing large source files exceed the preferred 250 pure LOC ceiling and remain a structural refactor risk.
