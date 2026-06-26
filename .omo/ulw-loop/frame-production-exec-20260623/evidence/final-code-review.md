# Final Code Quality Review - FrameTrace a42a320

codeQualityStatus: BLOCK
recommendation: REQUEST_CHANGES
reportPath: /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-code-review.md

Reviewed commit: a42a320be09eb85b05ff2b4f4f3964a3d69df8c3 (`HEAD^..HEAD`)

## Skill Perspective Check

- `remove-ai-slops` perspective: ran by consulting `/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/remove-ai-slops/SKILL.md`. The diff violates this perspective through a source-evidence mutation risk and a test that normalizes the unsafe behavior.
- `programming` perspective: ran by consulting `/Users/shinyoohag/.codex/plugins/cache/sisyphuslabs/omo/4.13.0/skills/programming/SKILL.md` and the Rust reference. The same unchecked output boundary violates the strict boundary/parsing expectation for production Rust CLI code.

## Evidence Reviewed

- Diff inspected: `git diff HEAD^..HEAD`, `git diff --name-status HEAD^..HEAD`, and targeted changed-file diffs for inventory export, CLI routing, Windows prerequisites, release gates, workstation status, media output policy, and report-defensibility code.
- Required evidence files inspected:
  - `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/full-validation.txt`
  - `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/windows-prereq-refresh-cli.txt`
  - `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/post-commit-validation.txt`
- Fresh check run during review: `git diff --check HEAD^..HEAD` passed.
- Evidence was not missing. The validation artifacts report passing Rust/node checks and targeted tests, and they show macOS release readiness correctly blocked on `windows_prerequisites`.

## Findings

### CRITICAL

None.

### HIGH

1. Inventory export can overwrite arbitrary files, including registered source evidence.

   References:
   - `/Users/shinyoohag/Desktop/frametrace/src/cli/commands.rs:261`
   - `/Users/shinyoohag/Desktop/frametrace/src/cli/inventory_cmd.rs:146`
   - `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs:33`
   - `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs:35`
   - `/Users/shinyoohag/Desktop/frametrace/src/case_db/inventory_export.rs:63`
   - `/Users/shinyoohag/Desktop/frametrace/src/util.rs:38`
   - `/Users/shinyoohag/Desktop/frametrace/tests/cli_inventory.rs:128`

   `inventory-export-manifest --output` is a public CLI surface, but the handler passes the requested path straight into `export_manifest`. `manifest_output_path` trusts any requested path, and `write_text` ultimately calls `fs::write`, which truncates or creates the target. Unlike the hardened media artifact paths in this same commit, this new path does not call `require_case_output_path`, does not reject source evidence paths, and does not reject pre-existing outputs. The CLI test currently asserts that an export outside the case directory succeeds.

   This is a direct source evidence mutation risk: an operator can point `--output` at a registered source media file or other external evidence-adjacent file and the command will overwrite it while emitting an audit event claiming `case_state_mutated:false`.

   Required fix: constrain requested manifest outputs to the case directory, reject outputs that resolve to any registered/source evidence path, reject pre-existing target files, and add regressions that outside-case and source-path outputs fail.

### MEDIUM

None.

### LOW

None.

## Windows/WinUI GA Claim Check

No blocking false Windows/WinUI GA claim was found. The release path appears fail-closed from macOS:

- `/Users/shinyoohag/Desktop/frametrace/src/windows_prerequisites.rs:36` blocks non-Windows hosts.
- `/Users/shinyoohag/Desktop/frametrace/src/windows_prerequisites.rs:58` requires a WinUI build/test receipt.
- `/Users/shinyoohag/Desktop/frametrace/src/qa_release.rs:26` includes `windows_prerequisites` in `qa release`.
- `/Users/shinyoohag/Desktop/frametrace/scripts/windows/validate-release.ps1:93` refuses non-Windows execution.
- `/Users/shinyoohag/Desktop/frametrace/scripts/windows/validate-release.ps1:201` writes the required WinUI receipt only after `dotnet build` and `dotnet test`.
- `/Users/shinyoohag/Desktop/frametrace/docs/FULL_PRODUCTION_GA_READINESS_REPORT.md:6` and `/Users/shinyoohag/Desktop/frametrace/docs/FULL_PRODUCTION_GA_READINESS_REPORT.md:143` explicitly state that macOS/missing WinUI evidence cannot be reported as release-ready and that GA GO must not be claimed.

## Blockers

- Fix the unchecked `inventory-export-manifest --output` path policy before approval. This is both a source evidence mutation risk and a misleading audit/reporting risk.

