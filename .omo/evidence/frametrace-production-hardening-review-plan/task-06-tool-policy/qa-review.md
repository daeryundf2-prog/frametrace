# T6 Evidence Re-Audit: Tool Policy

Verdict: <verdict>PASS</verdict>

Scope: evidence-only review under `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/`.

Surface and invocation used for this re-audit:

```sh
rg --files /Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy
ls -la /Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy
sed -n '1,220p' zero-byte-artifacts-after-repair.txt
jq '.' manual-happy-log-summary.json
sed -n '1,240p' manual-happy-transcript.log
sed -n '1,220p' manual-failure-transcript.log
sed -n '1,120p' manual-failure-command.stdout
sed -n '1,120p' manual-failure-command.stderr
for f in manual-happy-logs/*.jsonl; do sed -n '1,40p' "$f"; done
sed -n '1,220p' verify-cli-ffprobe-policy.log
tail -n 80 verify-full-cargo-test-after-ffprobe.log
sed -n '1,120p' verify-clippy-after-ffprobe.log
sed -n '1,120p' manual-happy-cleanup.txt
sed -n '1,120p' manual-failure-cleanup.txt
find . -type f -size 0 -print
```

## manualQa

### surfaceEvidence

| scenario id | criterion reference | surface | exact invocation | verdict | artifactRefs |
| --- | --- | --- | --- | --- | --- |
| T6-EVID-HYGIENE | zero-byte-artifacts-after-repair.txt says no zero-byte artifacts | evidence artifact receipt plus filesystem metadata check | `sed -n '1,220p' zero-byte-artifacts-after-repair.txt`; `find . -type f -size 0 -print` | PASS | A1 |
| T6-MANUAL-HAPPY-SUMMARY | manual-happy-log-summary.json includes `command_args` and `operator` for proxy/thumbnail/frame/export | JSON summary | `jq '.' manual-happy-log-summary.json` | PASS | A2 |
| T6-MANUAL-HAPPY-RAW | manual logs and transcripts prove approved fake ffmpeg | copied JSONL logs and transcript | `for f in manual-happy-logs/*.jsonl; do sed -n '1,40p' "$f"; done`; `sed -n '1,240p' manual-happy-transcript.log` | PASS | A3, A4, A5, A6, A7 |
| T6-FFPROBE-POLICY | verify-cli-ffprobe-policy.log proves approved/rejected ffprobe | targeted cargo test transcript | `sed -n '1,220p' verify-cli-ffprobe-policy.log` | PASS | A8 |
| T6-FULL-TEST | verify-full-cargo-test-after-ffprobe.log passes | full cargo test transcript | `tail -n 80 verify-full-cargo-test-after-ffprobe.log` | PASS | A9 |
| T6-CLIPPY | verify-clippy-after-ffprobe.log passes | clippy transcript | `sed -n '1,120p' verify-clippy-after-ffprobe.log` | PASS | A10 |
| T6-CLEANUP | cleanup receipts present | cleanup receipt artifacts | `sed -n '1,120p' manual-happy-cleanup.txt`; `sed -n '1,120p' manual-failure-cleanup.txt` | PASS | A11, A12 |

### adversarialCases

| scenario id | criterion reference | adversarial class | expected behavior | verdict | artifactRefs |
| --- | --- | --- | --- | --- | --- |
| T6-DENY-FAKE-FFMPEG | manual failure proves disallowed fake ffmpeg failed before output | disallowed executable basename/path | CLI exits non-zero with `unsupported tool binary`; rejected output path is absent before any output artifact is created | PASS | A13, A14, A15 |
| T6-DENY-FAKE-FFPROBE | verify-cli-ffprobe-policy.log proves approved/rejected ffprobe | disallowed ffprobe tool path/name | Targeted policy test passes, covering approved ffprobe metadata and rejected fake ffprobe fail-closed behavior | PASS | A8, A16, A17 |
| T6-ARTIFACT-HYGIENE-REPAIR | zero-byte artifacts after repair | repaired artifact hygiene regression | No zero-byte evidence artifacts remain after repair | PASS | A1 |

### artifactRefs

| id | kind | description | path |
| --- | --- | --- | --- |
| A1 | text receipt | Reports `PASS: no zero-byte evidence artifacts remain after repair`; direct `find . -type f -size 0 -print` produced no paths during re-audit. | `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/zero-byte-artifacts-after-repair.txt` |
| A2 | JSON summary | Four entries for export, frame, proxy, and thumbnail; each includes `operator: qa-tool-policy`, `command_args`, resolved fake `ffmpeg`, tool version, output hash, and entry hash. | `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/manual-happy-log-summary.json` |
| A3 | JSONL audit log | Raw export log with approved fake `ffmpeg`, command args, operator, source/output provenance, and entry hash. | `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/manual-happy-logs/export-log.jsonl` |
| A4 | JSONL audit log | Raw frame capture log with approved fake `ffmpeg`, command args, operator, source/output provenance, and entry hash. | `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/manual-happy-logs/frame-log.jsonl` |
| A5 | JSONL audit log | Raw proxy log with approved fake `ffmpeg`, command args, operator, source/output provenance, and entry hash. | `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/manual-happy-logs/proxy-log.jsonl` |
| A6 | JSONL audit log | Raw thumbnail log with approved fake `ffmpeg`, command args, operator, source/output provenance, and entry hash. | `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/manual-happy-logs/thumbnail-log.jsonl` |
| A7 | terminal transcript | Happy-path manual scenario invocation and outputs for make-proxy, make-thumbnail, capture-frame, export-video, and audit verification. | `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/manual-happy-transcript.log` |
| A8 | cargo test transcript | Targeted ffprobe policy test passes: `validate_artifact_requires_policy_approved_ffprobe_and_logs_resolved_tool_metadata ... ok`. | `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-cli-ffprobe-policy.log` |
| A9 | cargo test transcript | Full `cargo test --locked` after ffprobe fix passes through doc-tests with no failures in the inspected tail. | `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-full-cargo-test-after-ffprobe.log` |
| A10 | clippy transcript | `cargo clippy --locked --all-targets --all-features -- -D warnings` finishes successfully. | `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/verify-clippy-after-ffprobe.log` |
| A11 | cleanup receipt | Happy-path temp directory cleanup receipt with `status=0`. | `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/manual-happy-cleanup.txt` |
| A12 | cleanup receipt | Failure-path temp directory cleanup receipt with `status=0`. | `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/manual-failure-cleanup.txt` |
| A13 | terminal transcript | Failure scenario shows non-zero status for fake `fake-ffmpeg`, `unsupported tool binary`, and absent rejected output path. | `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/manual-failure-transcript.log` |
| A14 | stderr capture | Failure stderr contains `unsupported tool binary ... fake-ffmpeg; allowed tools: ffmpeg`. | `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/manual-failure-command.stderr` |
| A15 | stdout capture | Failure stdout repaired from zero-byte state and documents stdout intentionally empty while pointing to stderr and transcript evidence. | `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/manual-failure-command.stdout` |
| A16 | red ffprobe transcript | Pre-fix red evidence shows fake ffprobe policy failure: command unexpectedly succeeded before the fix. | `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/red-ffprobe-cli-policy.log` |
| A17 | green ffprobe transcript | Post-fix green evidence shows the same ffprobe policy test passes. | `/Users/shinyoohag/Desktop/frametrace/.omo/evidence/frametrace-production-hardening-review-plan/task-06-tool-policy/green-ffprobe-cli-policy.log` |

## Conclusion

All required T6 evidence checks are present, non-empty, and coherent after artifact hygiene repair. The manual happy path proves approved fake `ffmpeg` usage for proxy, thumbnail, frame, and export. The manual failure path proves disallowed fake `ffmpeg` fails before output creation. The targeted ffprobe policy, full cargo test, clippy, and cleanup receipts are present and passing.

<verdict>PASS</verdict>
