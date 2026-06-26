# Final Code Review After Scan DB Symlink Fix

codeQualityStatus: BLOCK
recommendation: REQUEST_CHANGES
reviewedHead: f589dea Block scan state symlink escapes
reportPath: /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/reviews/final-code-review-after-scan-db-symlink-fix.md

## Scope And Skill Checks

Reviewed current HEAD `f589dea` on branch `codex/frametrace-forensic-hardening`, with emphasis on `f589dea`, `c6a7abc`, `src/case_db/core.rs`, `src/scan.rs`, `src/util.rs`, `src/cli/handlers.rs`, `src/cli/inventory.rs`, `src/tool_policy.rs`, `tests/cli_output_policy.rs`, `tests/cli_inventory.rs`, and `tests/media_contract.rs`.

Skill perspective check ran:

- `code-review` skill loaded for severity ordering and code-review structure.
- `omo:programming` skill loaded, including the Rust reference, before judging Rust maintainability and boundary quality.
- `omo:remove-ai-slops` skill loaded before judging test relevance and overfit/slop risk.

Skill-perspective result:

- `remove-ai-slops`: the new output-policy tests are not deletion-only, not tautological, and they exercise CLI behavior with outside-target assertions. The remaining problem is coverage shape: existing derived-output tests cover explicit symlink leaves but not default generated artifact directories.
- `programming`: no new untyped escape hatch or needless abstraction was found in the reviewed fix. The violation is boundary enforcement: case-owned default output paths are still not uniformly parsed/guarded through `require_case_output_path` before durable writes.

## Verification Performed

- `git show --stat --oneline f589dea`: confirmed `src/case_db/core.rs`, `src/scan.rs`, and `tests/cli_output_policy.rs` changed for the scan DB fix.
- `git show --stat --oneline c6a7abc`: confirmed `src/cli/handlers.rs`, `src/util.rs`, and `tests/cli_output_policy.rs` changed for report/review symlink-leaf and parent checks.
- `cargo test --locked --test cli_output_policy -- --nocapture`: PASS, 5 tests.
- `cargo test --locked --test cli_inventory -- --nocapture`: PASS, 1 test.
- `cargo test --locked symlink -- --nocapture`: PASS, 14 relevant symlink-filtered tests across lib and CLI tests.
- `cargo test --locked derived_output_policy_tests -- --nocapture`: PASS, 5 tests, but only explicit `output_path: Some(...)` cases.
- `cargo test --locked --test media_contract -- --nocapture`: PASS, 3 tests.
- `git diff --check`: PASS.

I also ran a temporary repro with a fake `ffmpeg`: initialize a case, scan one media file, replace `case/artifacts/clips` with a symlink to an outside directory, then run `target/debug/frametrace export-video <case> vid_000001 --format mp4`. The command succeeded and the outside directory contained both the generated clip and `export-log.jsonl`.

## CRITICAL

None.

## HIGH

1. Default derived artifact outputs can still escape through symlinked case-owned parents.

Evidence:

- `src/video_export.rs:57` applies `require_case_output_path` only when the user supplies `--output`; the default path branch at `src/video_export.rs:67` builds `case/artifacts/clips/...` without the guard.
- `src/video_export.rs:77` then creates the output parent, and `src/video_export.rs:82` runs ffmpeg against that path.
- `src/video_export.rs:228` uses `case/artifacts/clips/export-log.jsonl`, and `src/video_export.rs:242` appends the export log without a case-output parent containment guard.
- `src/artifacts.rs:81`, `src/artifacts.rs:142`, and `src/artifacts.rs:203` guard only explicit output paths for proxy, thumbnail, and frame capture. The default branches at `src/artifacts.rs:92`, `src/artifacts.rs:153`, and `src/artifacts.rs:214` build case-owned paths without the guard, then `src/artifacts.rs:326` creates the parent and ffmpeg writes there.
- `src/artifacts.rs:340` constructs case-owned artifact log paths, and `src/artifacts.rs:350` writes them through `append_chained_jsonl` without parent containment.
- Existing derived-output policy tests in `src/derived_output_policy_tests.rs:68`, `src/derived_output_policy_tests.rs:86`, `src/derived_output_policy_tests.rs:104`, and `src/derived_output_policy_tests.rs:122` all pass `output_path: Some(...)`; they do not cover the default generated artifact directories.

Impact:

The previous class of blocker remains for default generated artifacts. A symlinked `case/artifacts/clips`, `case/artifacts/proxies`, `case/artifacts/thumbnails`, or `case/artifacts/frames` parent can redirect generated artifacts and logs outside the canonical case tree. This is not hypothetical: the temporary `export-video` repro wrote both the derived clip and `export-log.jsonl` to an outside directory while reporting success.

Minimal fix recommendation:

Compute the final output path first for both explicit and default branches, then call `require_case_output_path(case_dir, &output_path, "<label>")` unconditionally before `create_dir_all`, `File::create`, or ffmpeg. Apply the same case-output guard to the associated case-owned log path before `append_chained_jsonl`. Add tests that replace each default artifact parent with a symlinked outside directory and assert the command fails without creating outside files.

2. Other default generated case outputs have the same unguarded-parent pattern.

Evidence:

- `src/carve.rs:184` builds default carved outputs under `case/artifacts/carved`, then `src/carve.rs:189` calls `copy_range`; `copy_range` creates the parent at `src/carve.rs:316` and opens the output with `File::create` at `src/carve.rs:323` without a case-output guard.
- `src/carve.rs:330` writes `db/carve_results.json` via `write_text`, and `src/carve.rs:333` writes `artifacts/carved/carve-log.jsonl` via `append_chained_jsonl`; final symlink leaves are rejected by `write_text`, but symlinked parents are not contained unless the caller first uses `require_case_output_path`.
- `src/package.rs:39` defaults package output to `case/reports/package_<timestamp>`, then `src/package.rs:46` creates that output directory and `src/package.rs:74`, `src/package.rs:79`, and `src/package.rs:82` write generated package files without validating the default output parent against the canonical case root.

Impact:

The fixes in `c6a7abc` cover `make-review` and `make-report`, and the fix in `f589dea` covers scan DB/state outputs. They do not establish a uniform invariant for all default case-owned generated outputs. Any future or existing case-owned writer that relies only on `write_text` or direct `File::create` remains vulnerable to symlinked parent escapes.

Minimal fix recommendation:

Route every case-owned generated output path through one shared guard before creating parents or files. For package output, distinguish explicit external `--output-dir` behavior from the default in-case output: default in-case package paths should be guarded with `require_case_output_path`, while explicit external output should at least reject symlinked final/output directory leaves and recursive writes into the case tree.

## MEDIUM

None.

## LOW

None.

## Positive Findings

- `src/case_db/core.rs:11` now validates `db/case.db` with `require_case_output_path` before creating `db` and opening SQLite. This addresses the previous scan-folder DB symlink-parent blocker for SQLite creation.
- `src/scan.rs:241` routes scan run JSON, `video_index.json`, `videos.jsonl`, and `video_paths.tsv` through `write_case_text`, which calls `require_case_output_path` at `src/scan.rs:287`.
- `src/cli/handlers.rs:263`, `src/cli/handlers.rs:295`, and `src/cli/handlers.rs:343` now guard review HTML, evidence viewer HTML, and report HTML before writing.
- New-case initialization and normal scan were not broken in the targeted tests or temporary repro: `init-case` followed by `scan-folder --no-ffprobe` succeeded before replacing a default artifact directory with a symlink.

## Blockers

- Fix default derived/generated output paths so they cannot write through symlinked case-owned parents outside the canonical case tree.
- Add regression tests for default output parent symlink escapes, not just explicit `--output` symlink leaves.

BLOCKED
