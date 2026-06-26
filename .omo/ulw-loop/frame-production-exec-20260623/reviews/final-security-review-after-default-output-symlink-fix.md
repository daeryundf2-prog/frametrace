# Final Security Review After Default Output Symlink Fix

Repository: `/Users/shinyoohag/Desktop/frametrace`  
Branch: `codex/frametrace-forensic-hardening`  
Reviewed HEAD: `552b3fc` (`Block default artifact symlink escapes`)  
Review focus: output path safety and evidence integrity across the requested Rust modules and CLI policy tests.

## Verdict

Recommendation: REQUEST_CHANGES

Security status: BLOCK

Reason: a case-owned durable log path in `inspect-image` can still be redirected through a symlinked parent directory and can append directly to the selected source evidence path. This violates the original-evidence read-only invariant and the case-contained default-output invariant.

## Skill Perspective Check

The requested skill-perspective check ran.

- `omo:remove-ai-slops` was loaded and applied as a review lens. The new default-output tests for export/proxy/thumbnail/frame/carve/package are CLI-level behavioral tests, not deletion-only or tautological tests. However, the overall scoped test set is incomplete because it does not cover the same symlinked-parent invariant for `inspect-image`, `inspect-e01`, or `import-e01`.
- `omo:programming` and its Rust reference were loaded and applied as a review lens. The remaining issue violates the programming boundary rule: path safety is not consistently parsed/enforced at the output boundary before writes. The diff itself is small and mostly follows the existing guard pattern, but the broader scoped code still has unguarded durable writes through `write_text` and audit append helpers.

## Findings By Severity

### CRITICAL

None.

### HIGH

1. `inspect-image` can append to original source evidence through a symlinked `evidence/logs` parent.

References:

- `src/tsk.rs:131` to `src/tsk.rs:141` builds `case_dir/evidence/logs/tsk-mmls-*.txt` and writes it with `write_text` without `require_case_output_path`.
- `src/tsk.rs:164` to `src/tsk.rs:172` writes `tsk-fls-*.txt` the same way.
- `src/tsk.rs:223` to `src/tsk.rs:242` records the inspection event, and `src/tsk.rs:393` to `src/tsk.rs:394` appends to `case_dir/evidence/logs/tsk-audit.jsonl` without a case-output check.
- `src/tsk.rs:397` to `src/tsk.rs:401` accepts any existing file as an image path, so a source evidence file named `tsk-audit.jsonl` is valid.
- `src/util.rs:38` to `src/util.rs:43` rejects only the final symlink leaf and does not canonicalize or reject a symlinked parent directory.
- `src/tool_policy.rs:79` to `src/tool_policy.rs:105` contains the correct parent-canonicalization guard, but these `tsk.rs` writes do not use it.

Impact:

An attacker or accidental operator action that replaces `case/evidence/logs` with a symlink can make `inspect-image` write outside the case. If the selected image path is the symlink target's `tsk-audit.jsonl`, FrameTrace appends audit JSON directly to the source evidence file while reporting success. This breaks both "original evidence remains read-only" and "source evidence path cannot be targeted by derived outputs."

Exploit-style reproduction executed during review:

```sh
BIN="$PWD/target/debug/frametrace"
ROOT="$(mktemp -d /tmp/frametrace-security-review.XXXXXX)"
CASE="$ROOT/case"
SOURCE="$ROOT/source-evidence"
FAKE="$ROOT/fake-bin"
mkdir -p "$SOURCE" "$FAKE"
printf 'ORIGINAL_EVIDENCE\n' > "$SOURCE/tsk-audit.jsonl"

cat > "$FAKE/mmls" <<'SH'
#!/bin/sh
case "$1" in -V|--version|-version) echo 'mmls fake 1.0'; exit 0 ;; esac
exit 0
SH

cat > "$FAKE/fls" <<'SH'
#!/bin/sh
case "$1" in -V|--version|-version) echo 'fls fake 1.0'; exit 0 ;; esac
echo 'r/r 5: clip.mp4'
exit 0
SH

chmod +x "$FAKE/mmls" "$FAKE/fls"
"$BIN" init-case "$CASE" --title OutputPolicy
rm -rf "$CASE/evidence/logs"
ln -s "$SOURCE" "$CASE/evidence/logs"
"$BIN" inspect-image "$CASE" "$SOURCE/tsk-audit.jsonl" --mmls "$FAKE/mmls" --fls "$FAKE/fls"
```

Observed result:

```text
status=0
filesystem image inspected
image: /tmp/frametrace-security-review.j3UYkV/source-evidence/tsk-audit.jsonl
mmls log: /tmp/frametrace-security-review.j3UYkV/case/evidence/logs/tsk-mmls-1782218427.txt
fls log: /tmp/frametrace-security-review.j3UYkV/case/evidence/logs/tsk-fls-1782218427.txt

source dir entries:
tsk-audit.jsonl      948 bytes
tsk-fls-1782218427.txt       57 bytes
tsk-mmls-1782218427.txt       41 bytes

tsk-audit head:
ORIGINAL_EVIDENCE
{"schema_version":1,"event":"inspect-image-filesystem",...}
```

The command should have failed with an "inside the case directory" policy error and left the outside/source directory untouched.

2. E01 inspection/import logs can still escape through symlinked `evidence/logs`.

References:

- `src/e01.rs:51` to `src/e01.rs:57` writes `e01-info-*.txt` with no `require_case_output_path`.
- `src/e01.rs:90` to `src/e01.rs:96` repeats the same write during import.
- `src/e01.rs:101` to `src/e01.rs:115` passes an unguarded `e01-verify-*.txt` path to `ewfverify`.
- `src/e01.rs:143` to `src/e01.rs:149` passes an unguarded `e01-export-*.txt` path to `ewfexport`.
- `src/e01.rs:341` to `src/e01.rs:342` appends `evidence/logs/e01-audit.jsonl` without the case-output guard.

Impact:

Default generated E01 logs are case-owned durable outputs. A symlinked `evidence/logs` directory redirects those logs outside the case and can leave the case audit trail incomplete or stored in attacker-controlled/external locations. This violates the requested invariant that default generated artifact/log/package outputs are case-contained.

### MEDIUM

1. Existing focused tests cover the prior blockers but not the equivalent TSK/E01 log surfaces.

References:

- `tests/cli_default_output_policy.rs:63` to `tests/cli_default_output_policy.rs:160` covers default export-video, proxy/thumbnail/frame, carve-file, and package-case symlinked-parent blockers.
- `tests/cli_output_policy.rs:168` to `tests/cli_output_policy.rs:200` covers scan-folder's symlinked `db` parent.
- No equivalent CLI policy test covers `inspect-image` with symlinked `evidence/logs` or `db/filesystem`, or `inspect-e01`/`import-e01` with symlinked `evidence/logs`.

Impact:

The fixed blocker is tested, but the invariant is not regression-locked across all scoped case-owned durable output paths. This allowed the `tsk.rs` and `e01.rs` paths above to remain unguarded.

### LOW

None.

## Positive Findings

The previous default artifact/package blocker is fixed for the changed paths reviewed:

- `src/video_export.rs:75` checks the generated default clip path before creating the output parent, and `src/video_export.rs:230` checks the export log path before appending.
- `src/artifacts.rs:98`, `src/artifacts.rs:160`, and `src/artifacts.rs:222` check generated proxy/thumbnail/frame paths before parent creation; `src/artifacts.rs:344` checks derived artifact logs.
- `src/carve.rs:190` checks generated carved artifact paths before `copy_range`, and `src/carve.rs:333` plus `src/carve.rs:338` check carve results/log paths.
- `src/package.rs:43` to `src/package.rs:49` checks the default package output under `reports/package_*`.
- `src/package.rs:26` to `src/package.rs:27` and `src/package.rs:115` to `src/package.rs:127` reject an explicit package output symlink leaf.
- `src/tool_policy.rs:109` to `src/tool_policy.rs:130` rejects source/output equality for existing explicit derived-media output paths, and the generated defaults reviewed above are now bounded by `require_case_output_path`.

## Verification Commands

Fresh repository checks:

```text
git rev-parse --short HEAD
552b3fc

git rev-parse --abbrev-ref HEAD
codex/frametrace-forensic-hardening
```

Focused tests run:

```text
cargo test --locked --test cli_default_output_policy -- --nocapture
4 passed; 0 failed

cargo test --locked --test cli_output_policy -- --nocapture
5 passed; 0 failed

cargo test --locked symlink -- --nocapture
9 passed; 0 failed; 108 filtered out in src/lib.rs, plus the focused CLI symlink suites passed
```

These results confirm the prior export/carve/package/report/scan symlink tests are green, but they do not clear the remaining `inspect-image`/E01 log escape.

## Blockers

- Add `require_case_output_path` checks before every case-owned durable TSK write: `mmls_log_path`, `fls_log_path`, `entries_jsonl_path`, `summary_path`, and `evidence/logs/tsk-audit.jsonl`. Also reject any output path that resolves to the selected source image path.
- Add `require_case_output_path` checks before every case-owned durable E01 log path, including paths handed to external tools (`ewfverify -l`, `ewfexport -l`) and `evidence/logs/e01-audit.jsonl`.
- Add CLI regression tests that reproduce symlinked `evidence/logs` and `db/filesystem` parents for `inspect-image`, plus symlinked `evidence/logs` for `inspect-e01`/`import-e01`, asserting failure and no outside/source mutation.

BLOCKED
