# Final Security Review After Scan DB Symlink Fix

Repository: `/Users/shinyoohag/Desktop/frametrace`
Branch: `codex/frametrace-forensic-hardening`
HEAD reviewed: `f589deaafa1ef8d8c036b1734b6ca3230a266db3`
Review focus: output path and evidence integrity hardening across commits `f589dea`, `c6a7abc`, `151fd5c`, `a42c3af`, `14eba5e`, `541ec49`, and `a42a320`.

## Skill-Perspective Check

- `security-review`: ran. Reviewed path traversal, symlink redirection, evidence modification, command-output boundaries, and durable-output integrity.
- `omo:remove-ai-slops`: ran. Existing focused tests are not deletion-only, tautological, or simple removal checks. However, they overfit explicit final-leaf symlink cases and miss default-output parent-symlink attacks for derived artifacts.
- `omo:programming` plus Rust reference: ran. The reviewed hardening code benefits from a central output policy, but several derived-output writers still pass ordinary `PathBuf` values through direct filesystem writes instead of a type- or policy-checked case-output boundary.

Result: the diff violates the security and programming perspectives because not all durable derived output writes are routed through the canonical case-output policy before filesystem creation.

## Findings By Severity

### CRITICAL

None.

### HIGH

1. Default derived artifact outputs can be redirected through a symlinked case artifact parent into source evidence directories.

Files:
- `src/carve.rs:184` to `src/carve.rs:190`
- `src/carve.rs:316` to `src/carve.rs:324`
- `src/carve.rs:329` to `src/carve.rs:335`
- `src/audit.rs:74` to `src/audit.rs:99`
- `src/util.rs:38` to `src/util.rs:43`

`carve_file` builds its default output with `unique_path(case_dir.join("artifacts/carved/..."))` and then calls `copy_range` without `require_case_output_path`. `copy_range` creates the parent directory and then uses `File::create`, which follows a symlinked parent directory. `write_carve_outputs` then appends `artifacts/carved/carve-log.jsonl` through the same symlinked parent via `audit::append_chained_jsonl` and `write_text`; `write_text` rejects a symlink leaf but does not reject a symlinked parent.

Real-surface reproduction executed against `target/debug/frametrace`:

```sh
root=$(mktemp -d /tmp/frametrace-carve-symlink-repro.XXXXXX)
case_dir="$root/case"
source_dir="$root/source-evidence"
mkdir -p "$source_dir"
printf '\000\000\000\030ftypmp42payloadpayloadpayloadpayload' > "$source_dir/source.mp4"
target/debug/frametrace init-case "$case_dir" --title SymlinkRepro >/dev/null
rm -rf "$case_dir/artifacts/carved"
ln -s "$source_dir" "$case_dir/artifacts/carved"
target/debug/frametrace carve-file "$case_dir" "$source_dir/source.mp4" --max-candidates 1 --max-bytes 32
```

Observed result:

```text
status=0
source_dir_entries=carve-log.jsonl carve_000001_000000000000.mp4 source.mp4
carved_link=-> /tmp/frametrace-carve-symlink-repro.gEaI5N/source-evidence
```

Impact: an attacker or operator mistake can cause FrameTrace to write derived artifacts and audit logs into the original evidence directory by replacing `case/artifacts/carved` with a symlink. This breaks the requested forensic invariant that original evidence remains unmodified and that durable outputs cannot escape through symlinked parent directories.

Related same-pattern risk, not separately reproduced in this pass:
- `src/video_export.rs:68` to `src/video_export.rs:80` computes default clip outputs under `artifacts/clips` and creates the parent without a canonical output policy check.
- `src/artifacts.rs:92` to `src/artifacts.rs:100`, `src/artifacts.rs:153` to `src/artifacts.rs:160`, and `src/artifacts.rs:214` to `src/artifacts.rs:221` compute default proxy, thumbnail, and frame outputs under artifact directories without the same policy check used for explicit `--output` paths.

Required fix: route every durable derived output path, including default paths, through the same canonical case-output policy before any parent creation, external tool invocation, `File::create`, `fs::write`, or audit append. Add adversarial tests for symlinked artifact parent directories, not only dangling symlink leaves.

### MEDIUM

None.

### LOW

None.

## Verified Passing Areas

- Canonical output policy: `src/tool_policy.rs:79` to `src/tool_policy.rs:106` canonicalizes the case root, canonicalizes the nearest existing output parent, rejects parents outside the case root, and rejects symlink leaf outputs.
- SQLite case DB writes: `src/case_db/core.rs:11` to `src/case_db/core.rs:19` checks `db/case.db` with `require_case_output_path` before opening the SQLite DB.
- Scan durable outputs: `src/scan.rs:241` to `src/scan.rs:289` routes scan snapshot, `video_index.json`, `videos.jsonl`, and `video_paths.tsv` through `write_case_text`, which calls `require_case_output_path` before `write_text`. `src/case_db/scan.rs:7` to `src/case_db/scan.rs:27` writes SQLite scan state through `open_case_db`.
- Review/report HTML outputs: `src/cli/handlers.rs:256` to `src/cli/handlers.rs:345` checks `review/index.html`, `review/evidence-viewer.html`, and `reports/case-report.html` before writing.
- Inventory export outputs: `src/case_db/inventory_export.rs:66` to `src/case_db/inventory_export.rs:124` checks output containment, rejects symlink/existing leaves, and rejects registered source evidence file targets.

## Test And Reproduction Evidence

Commands run:

```sh
cargo test --test cli_output_policy --test cli_inventory --test cli_review
cargo test --lib
cargo test --test media_contract
```

Results:

- `cli_output_policy`, `cli_inventory`, and `cli_review`: PASS, 8 tests.
- Library tests: PASS, 117 tests.
- `media_contract`: PASS, 3 tests.

Test relevance assessment:

- `tests/cli_output_policy.rs:55` to `tests/cli_output_policy.rs:200` provides real CLI coverage for review/report symlink leaves, review/report symlinked parent directories, and scan-folder `db` symlink rejection.
- `tests/cli_inventory.rs:146` to `tests/cli_inventory.rs:223` provides real CLI coverage for inventory export inside-case, outside-case, existing-output, and symlink-leaf behavior.
- `src/derived_output_policy_tests.rs:67` to `src/derived_output_policy_tests.rs:160` covers explicit derived-output symlink leaves before external tools, but it does not cover default output paths or symlinked artifact parent directories.

The test suite passing is therefore real evidence for the newly added checks, but it is not sufficient evidence for the broader durable derived-output invariant.

## Invariant Verdict

- Original evidence not modified: BLOCKED. Reproduced `carve-file` writes derived files into a symlinked source evidence directory.
- Derived outputs cannot be written into source evidence paths: BLOCKED for symlinked artifact parent directories.
- Final leaf symlinks and symlinked parent dirs rejected for durable outputs: PASS for scan/report/review/inventory paths reviewed; BLOCKED for default derived artifact outputs.
- Inventory exports cannot write to evidence paths: PASS for registered source evidence file targets and symlink/existing leaves.
- Report/review generated HTML writes are bounded to canonical case outputs: PASS.
- Scan-folder cannot write DB/index/JSONL/TSV through symlinked `case/db`: PASS.

## Recommendation

REQUEST_CHANGES. The scan DB symlink fix itself is sound, and the report/review/inventory fixes are appropriately covered, but the same durable-output invariant is not enforced across default derived artifact writers. Fix the derived-output parent-symlink bypass and add real-surface tests before approval.

BLOCKED
