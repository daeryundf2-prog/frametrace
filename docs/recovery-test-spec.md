# Recovery Test Specification

Status: approved for implementation and regression execution.

## Test Layers

| Layer | Required Evidence |
| --- | --- |
| Unit | Parser/build-argument helpers, JSON structural checks, path policy, migration helpers. |
| Integration | CLI case lifecycle, QA commands, package generation, report/viewer generation. |
| Corpus | Typed ground-truth corpus manifest compared with `qa accuracy`. |
| Reproducibility | Same corpus scanned into repeat case directories and compared with `qa reproducibility`. |
| Scale | SQLite benchmark through `qa performance`. |
| Defensibility | Required artifact presence through `qa report-defense`. |

## Required Commands

Run from the repository root:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
node --check gui/evidence-viewer/app.js
```

Run on each validation case:

```bash
frametrace qa accuracy <case_dir> <corpus_manifest>
frametrace qa reproducibility <case_dir_a> <case_dir_b>
frametrace qa report-defense <case_dir>
frametrace qa performance <output_dir> --rows 100000
frametrace qa release <case_dir> --corpus-manifest <corpus_manifest> --comparison-case <case_dir_b> --review-manifest <release_review_manifest> --performance-output-dir <output_dir> --performance-rows 100000
```

## Corpus Manifest Format

`qa accuracy` consumes typed JSON corpus manifests for release evidence:

```json
{
  "schema_version": 1,
  "corpus_id": "synthetic-video-corpus",
  "corpus_kind": "synthetic",
  "release_keys": {
    "mixed_real_world_like": "unsupported"
  },
  "domains": [
    {
      "key": "video_recovery",
      "status": "supported",
      "ground_truth_schema": [
        "corpus_id",
        "source_artifact_id",
        "source_sha256",
        "expected_artifact_type",
        "expected_path_pattern",
        "expected_hash",
        "expected_timestamp_range",
        "expected_state",
        "negative_controls",
        "notes"
      ],
      "expected_outputs_schema": ["db/videos.jsonl", "evidence/logs/validation-log.jsonl"]
    }
  ],
  "cases": [
    {
      "case_id": "SYN-VID-001",
      "domain": "video_recovery",
      "source_path": "tests/fixtures/corpus/synthetic-video-case-a/source/video-a.mp4",
      "source_sha256": "55818161733f2a9bc13b60c48fcfc2623f267417e5e5d89e2c36283514eb95e6",
      "ground_truth": {
        "corpus_id": "synthetic-video-corpus",
        "source_artifact_id": "source-video-a",
        "source_sha256": "55818161733f2a9bc13b60c48fcfc2623f267417e5e5d89e2c36283514eb95e6",
        "expected_artifact_type": "source-video",
        "expected_path_pattern": "tests/fixtures/corpus/synthetic-video-case-a/source/video-a.mp4",
        "expected_hash": "55818161733f2a9bc13b60c48fcfc2623f267417e5e5d89e2c36283514eb95e6",
        "expected_timestamp_range": {
          "start_unix": 1782470000,
          "end_unix": 1782470003
        },
        "expected_state": "ffprobe-video-stream-confirmed",
        "negative_controls": [
          "tests/fixtures/corpus/synthetic-video-case-a/source/not-video.txt"
        ],
        "notes": "Lightweight committed non-client fixture file."
      },
      "expected_outputs": {
        "indexed": true,
        "validation_status": "ffprobe-video-stream-confirmed"
      }
    }
  ],
  "external_references": []
}
```

Rules:

- `schema_version` must be `1`.
- Every supported domain declares exactly `corpus_id`, `source_artifact_id`, `source_sha256`, `expected_artifact_type`, `expected_path_pattern`, `expected_hash`, `expected_timestamp_range`, `expected_state`, `negative_controls`, and `notes` as ground-truth fields, plus expected-output schema fields.
- Unsupported domains are recorded as `"unsupported"` with a reason and are not pass evidence.
- Synthetic-only manifests cannot satisfy the `mixed_real_world_like` release key.
- Hash-only external references are allowed for large non-client corpora that stay outside git.
- `source_path` must match one indexed evidence path:
  - canonical source path written to `db/videos.jsonl`
  - carved artifact `output_path` from `artifacts/carved/carve-log.jsonl`
  - recovered inode `output_path` from `evidence/logs/tsk-audit.jsonl`
  - validation `target_path` from `evidence/logs/validation-log.jsonl`
- `source_sha256` and `ground_truth.expected_hash` must match the indexed `sha256`/`target_sha256` for hashed evidence.
- Legacy UTF-8 TSV rows (`source_path<TAB>sha256`) remain accepted for compatibility, but they are not sufficient release evidence.

## Pass/Fail Thresholds

| Metric | Pass Criteria |
| --- | --- |
| Precision | `>= 0.98` |
| Recall | `>= 0.98` |
| Hash mismatch | `0` |
| False positives / false negatives | Reported as `false_positives` and `false_negatives`; any P0 false negative blocks release. |
| Reproducibility | Normalized scan, recovery, validation, filesystem, and package outputs differ by no more than the report's `allowed_diff_thresholds.normalized_core_differences` value, currently `0`. |
| Report defensibility | All required artifacts present |
| Performance | `>= 50000` rows/minute, `max_query_ms <= 2000` for indexed SQLite queries, `query_plan_full_scan_count == 0`, `max_rss_bytes <= max_rss_target_bytes`, and measured `cpu_average_percent` |

## Release Review Manifest Format

`qa release` must receive `--review-manifest`; otherwise the release readiness report is blocked. The manifest records non-automatable release gates that require human or external evidence:

```json
{
  "schema_version": 1,
  "gates": [
    {
      "key": "technical_review",
      "status": "PASS",
      "artifact_path": "release-review-artifacts/technical_review.json",
      "tool": "scripts/windows/validate-release.ps1",
      "evidence": "typed technical review receipt",
      "timestamp": "2026-06-27T00:00:00Z",
      "reviewer": "release-validator",
      "operator": "release-validator",
      "cleanup_status": "clean"
    }
  ]
}
```

Every required release gate must appear as a typed JSON entry with a status, artifact path, tool, evidence, timestamp, reviewer or operator, and clean cleanup status. Text key/value manifests and Markdown checkboxes are rejected. Any missing, non-`PASS`, unsupported, or artifact-missing gate is a release blocker. `reports/qa/release-decision.json` is written for each release run with `FIELD_PILOT_GO`, `NO_GO`, or `BLOCKED` plus exact blockers.

`scripts/windows/validate-release.ps1` requires an externally prepared typed manifest via `-ReviewManifestPath` for a release-ready run. If the path is missing or omitted, the script writes a typed BLOCKED manifest and `release-decision.json` before exiting.

## Required Regression Cases

1. Case output directory is rejected as a scan source.
2. Nested case output directory is skipped when scanning a parent source.
3. Package generation rejects symlinked package inputs.
4. Package generation rejects missing required files.
5. Unapproved external tool names are rejected.
6. Explicit recovery/export outputs outside the case directory are rejected.
7. Version 1 SQLite databases migrate to the current schema with ordered backups for every migration step.
8. Evidence viewer includes TSK inode recovery outputs.
9. Release readiness command writes `release-readiness.json` and `release-decision.json`, rejects text review manifests, and fails on missing review blockers.
10. Report defensibility command fails if report/viewer outputs contain disallowed legal-overclaim terms.
11. Reproducibility command compares recovery logs, validation status, filesystem listings, and package manifests while normalizing case-local paths and volatile timestamps.
12. Report defensibility command fails while any SQLite job remains `running`; the operator must complete it or mark it interrupted before release review.

## Evidence Retention

Keep generated QA artifacts under `<case_dir>/reports/qa` unless a test-specific output directory is required. Preserve `case.json`, `db/case.db`, `db/videos.jsonl`, `db/video_paths.tsv`, `reports/case-report.html`, `review/evidence-viewer.html`, typed release review manifest, `release-decision.json`, and QA JSON/HTML/Markdown reports for release review.
