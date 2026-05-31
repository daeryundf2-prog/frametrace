# Recovery Test Specification

Status: approved for implementation and regression execution.

## Test Layers

| Layer | Required Evidence |
| --- | --- |
| Unit | Parser/build-argument helpers, JSON structural checks, path policy, migration helpers. |
| Integration | CLI case lifecycle, QA commands, package generation, report/viewer generation. |
| Corpus | Ground-truth TSV manifest compared with `qa accuracy`. |
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
frametrace qa release <case_dir> --corpus-manifest <corpus_manifest> --comparison-case <case_dir_b> --performance-output-dir <output_dir> --performance-rows 100000
```

## Corpus Manifest Format

`qa accuracy` consumes UTF-8 tab-separated rows:

```text
source_path	sha256
/absolute/path/to/evidence/video001.mp4	optional_sha256
```

Rules:

- Header row is optional.
- Blank lines and `#` comments are ignored.
- `source_path` must match the canonical source path written to `db/videos.jsonl`.
- `sha256` may be blank when the scan was intentionally run without `--hash`.

## Pass/Fail Thresholds

| Metric | Pass Criteria |
| --- | --- |
| Precision | `>= 0.98` |
| Recall | `>= 0.98` |
| Hash mismatch | `0` |
| Reproducibility | Normalized core outputs exactly equal |
| Report defensibility | All required artifacts present |
| Performance | `>= 50000` rows/minute |

## Required Regression Cases

1. Case output directory is rejected as a scan source.
2. Nested case output directory is skipped when scanning a parent source.
3. Package generation rejects symlinked package inputs.
4. Package generation rejects missing required files.
5. Unapproved external tool names are rejected.
6. Explicit recovery/export outputs outside the case directory are rejected.
7. Version 1 SQLite databases migrate to version 2 with a backup.
8. Evidence viewer includes TSK inode recovery outputs.
9. Release readiness command writes `release-readiness.json` and fails on blockers.

## Evidence Retention

Keep generated QA artifacts under `<case_dir>/reports/qa` unless a test-specific output directory is required. Preserve `case.json`, `db/case.db`, `db/videos.jsonl`, `db/video_paths.tsv`, `reports/case-report.html`, `review/evidence-viewer.html`, and QA JSON/HTML/Markdown reports for release review.
