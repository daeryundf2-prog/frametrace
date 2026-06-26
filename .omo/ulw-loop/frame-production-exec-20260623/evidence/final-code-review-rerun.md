# Final Code Quality Review Rerun - FrameTrace

codeQualityStatus: BLOCK
recommendation: REQUEST_CHANGES
reportPath: /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-code-review-rerun.md

Reviewed commits:
- a42a320 Harden forensic workstation release gates
- 541ec49 Block inventory exports from evidence paths

Compared range: 1e07753..HEAD

## Skill Perspective Check

- `remove-ai-slops`: ran by loading `/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/remove-ai-slops/SKILL.md`. The rerun still violates this perspective because the production output policy has an over-defensive-looking but incomplete path check: it validates the parent and ordinary existing targets, but does not reject a symlink final component before a write.
- `programming`: ran by loading `/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/programming/SKILL.md`, `/references/rust/README.md`, `/references/rust/clap-stack.md`, `/references/rust/proptest-insta.md`, and `/references/code-smells.md`. The rerun still violates the boundary-safety perspective for Rust CLI output handling because an untrusted CLI output path is not parsed into a filesystem-safe case-contained destination before `fs::write`.
- `code-review`: loaded `/Users/shinyoohag/.codex/skills/code-review/SKILL.md`. Independent subagent lanes were not spawned because the currently exposed subagent tool contract permits spawning only when the user explicitly requests delegation. This report is therefore a direct read-only reviewer pass, not a delegated two-lane review.

## Evidence Reviewed

- `git status --short`
- `git log --oneline -5`
- `git diff --stat 1e07753..HEAD`
- `git diff --name-only 1e07753..HEAD`
- `git show --stat --oneline a42a320`
- `git show --stat --oneline 541ec49`
- `git show --unified=80 541ec49 -- src/case_db/inventory_export.rs src/case_db/inventory_tests.rs tests/cli_inventory.rs src/tool_policy.rs`
- Source files inspected with line numbers:
  - `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs`
  - `/Users/shinyoohag/Desktop/frametrace/src/tool_policy.rs`
  - `/Users/shinyoohag/Desktop/frametrace/src/util.rs`
  - `/Users/shinyoohag/Desktop/frametrace/tests/cli_inventory.rs`
  - `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_tests.rs`
  - `/Users/shinyoohag/Desktop/frametrace/src/windows_prerequisites.rs`
  - `/Users/shinyoohag/Desktop/frametrace/tests/cli_windows_prereq.rs`
  - `/Users/shinyoohag/Desktop/frametrace/scripts/windows/validate-release.ps1`
  - `/Users/shinyoohag/Desktop/frametrace/src/qa_release.rs`
- Required evidence artifacts inspected:
  - `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/inventory-export-output-policy-fix.txt`
  - `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/full-validation.txt`
  - `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/media-audit-report-cli-proof.txt`
  - `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/gui-browser-proof.txt`
  - `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/post-commit-validation.txt`

I did not rerun cargo/node/browser commands during this read-only review because the user allowed writing only this report file. I treated the provided evidence as untrusted until inspected.

## Findings

### CRITICAL

None.

### HIGH

1. Dangling symlink output paths can still escape the case directory before `case_state_mutated:false` is emitted.

   References:
   - `/Users/shinyoohag/Desktop/frametrace/src/tool_policy.rs:88`
   - `/Users/shinyoohag/Desktop/frametrace/src/tool_policy.rs:93`
   - `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs:73`
   - `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs:75`
   - `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs:112`
   - `/Users/shinyoohag/Desktop/frametrace/src/util.rs:42`

   `541ec49` fixes normal outside paths, normal existing files, and direct registered source evidence paths. However, `require_case_output_path` canonicalizes only the nearest existing parent of the requested output, not the final output component. `manifest_output_path` then treats `output_path.exists()` as the non-existing target gate. On Unix, `Path::exists()` follows symlinks and returns false for a dangling symlink. `write_text` finally calls `fs::write`, which follows the symlink target. A dangling symlink located inside the case directory can therefore pass the policy as "non-existing" while the actual write creates the symlink target outside the case. If the symlink target is a registered source path that is temporarily missing, this also recreates/mutates a registered evidence path while the audit event still claims `case_state_mutated:false`.

   Required fix: make the output policy symlink-aware before writing. At minimum, reject any final output component with `symlink_metadata` present, including dangling symlinks, and add an adversarial regression where `<case>/reports/out.json` is a dangling symlink to an outside path and `inventory-export-manifest --output` must fail without creating the outside target. Prefer an atomic create-new flow under a canonical case-contained directory, with no symlink-following behavior for the final path where the platform supports it.

### MEDIUM

None.

### LOW

None.

## Positive Checks

- Direct source evidence overwrite is fixed for ordinary registered paths: `inventory_export.rs:73-80` now requires a case-contained output, rejects registered source paths, and rejects existing targets before writing.
- The added unit tests are relevant, not deletion-only or tautological: `inventory_tests.rs:212-284` covers outside-case, existing-output, and registered source evidence rejection. The missing adversarial case is dangling-symlink escape.
- The CLI test now exercises the public `inventory-export-manifest` surface for case-contained output, outside-case failure, source/outside failure, and existing-output failure in `tests/cli_inventory.rs:146-201`.
- Windows/WinUI readiness remains fail-closed on macOS: `windows_prerequisites.rs:36-46` records unsupported host and missing WinUI project blockers, `windows_prerequisites.rs:58-66` requires a WinUI build receipt, `qa_release.rs:26-28` makes `windows_prerequisites` a release check, and `scripts/windows/validate-release.ps1:93-95` refuses non-Windows execution.
- The provided evidence has concrete artifact paths and is not misleading success output without artifacts. `inventory-export-output-policy-fix.txt` records targeted cargo fmt/clippy/tests plus full `cargo test --locked`; `gui-browser-proof.txt` records a screenshot path and cleanup; `media-audit-report-cli-proof.txt` records case paths and retained browser proof setup; `post-commit-validation.txt` records earlier a42a320 validation.

## Blockers

- Fix the dangling-symlink output escape for `inventory-export-manifest --output` and add a regression proving the outside symlink target is not created or modified.

## Final Decision

REQUEST_CHANGES
