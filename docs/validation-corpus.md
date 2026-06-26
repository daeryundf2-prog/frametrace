# Validation Corpus Manifest

Status: T12 validation corpus manifests and non-client fixtures are defined. Real client evidence files remain external and must not be committed.

## Storage Rule

Keep media, raw images, and E01 files outside git. Commit only manifests, hashes, lightweight non-client JSONL fixtures, expected-output summaries, and generated QA reports that do not contain sensitive evidence.

The committed corpus entry point is `corpus/manifest/synthetic-video-corpus.json`. It references lightweight fixture files in `tests/fixtures/corpus/`. The committed fixtures are synthetic/non-client and therefore do not satisfy the `mixed_real_world_like` release key. No hash-only external corpus is claimed until a real external corpus hash and provenance record are available.

## Corpus A: Deleted File Recovery

- Purpose: validate deleted-video discovery and inode recovery from a raw filesystem image.
- Ground truth: generated filesystem fixture notes plus SHA-256 of deleted source file before deletion.
- Expected outputs: `inspect-image` identifies candidate path/inode; `recover-inode` writes a hashed artifact; `validate-artifact` marks it confirmed or failed.
- Pass criteria: required deleted video recovered or explicitly marked unsupported with reason; no output outside the case directory.

## Corpus B: Browser Artifacts

- Purpose: future browser-history/media-reference parsing.
- Ground truth: browser fixture export and expected URL/file-reference table.
- Expected outputs: design-candidate only until parser PRD exists.
- Pass criteria: not release-blocking for current video recovery release.

## Corpus C: Windows Event Logs

- Purpose: future event-log timeline context.
- Ground truth: EVTX fixture with known timestamps and event IDs.
- Expected outputs: design-candidate only until parser PRD exists.
- Pass criteria: not release-blocking for current video recovery release.

## Corpus D: Timeline Reconstruction

- Purpose: validate ordering across scan time, modified time, export time, recovery time, and validation time.
- Ground truth: fixture event table.
- Expected outputs: sorted report rows and deterministic normalized output.
- Pass criteria: timestamp order matches ground truth with zero P0 ordering errors.

## Corpus E: Large Evidence Dataset

- Purpose: validate that indexing and SQLite operations survive large case sizes.
- Ground truth: generated synthetic row count and optional media hash manifest.
- Expected outputs: `qa performance` report and completion log.
- Pass criteria: `rows_per_minute >= 50000`, `max_query_ms <= 2000` for indexed SQLite queries, `query_plan_full_scan_count == 0`, `max_rss_bytes <= max_rss_target_bytes`, measured `cpu_average_percent`, and no database migration/indexing failure.

## Corpus F: Mixed Real-World Case Dataset

- Purpose: validate operator workflow across generic video, DVR extensions, carved candidates, and filesystem recovery.
- Ground truth: examiner-approved TSV manifest plus source acquisition notes.
- Expected outputs: accuracy, reproducibility, report-defense, and package artifacts.
- Pass criteria: precision >= 0.98, recall >= 0.98, hash mismatch = 0, report-defense pass.

## Typed Manifest Schema

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
    },
    {
      "key": "browser_artifacts",
      "status": "unsupported",
      "reason": "parser PRD is not approved for this release"
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
- `corpus_kind: "synthetic"` cannot claim `release_keys.mixed_real_world_like` as `pass`, `passed`, `supported`, or `true`.
- Supported domains must declare the exact ground-truth schema fields `corpus_id`, `source_artifact_id`, `source_sha256`, `expected_artifact_type`, `expected_path_pattern`, `expected_hash`, `expected_timestamp_range`, `expected_state`, `negative_controls`, and `notes`, plus non-empty expected-output schema fields.
- Unsupported domains must be recorded with `"status": "unsupported"` and a reason; they are not counted as pass evidence.
- Manifest case `source_path` entries must be committed lightweight non-client fixture files with verifiable hashes for this synthetic corpus. Large real-world-like corpora must stay outside git and may be represented only by real hash-only external references with provenance unless explicitly approved for publication. Placeholder external hashes are not release evidence.
- Legacy TSV manifests remain accepted for existing automation, but typed JSON manifests are the release evidence format.
