# SQLite Schema Audit

Phase 3 audit records the current schema and migration risks before destructive schema changes.

## Current Schema Version

- `SCHEMA_VERSION = "3"`
- New databases are initialized at schema version 3.
- Existing version 1 and 2 databases migrate forward with a pre-migration backup
  (`case.db.backup-v<N>-to-v3`).
- Version 3 adds the `review_marks` table (examiner marks imported via `import-marks`).
- Unsupported versions still fail closed.

## Tables

| Table | Purpose | Current Status |
| --- | --- | --- |
| `schema_meta` | Stores schema version. | Required core. |
| `scan_runs` | Stores scan metadata and warnings. | Required provenance. |
| `videos` | Primary indexed video table. | Required core. |
| `evidence_sources` | Tracks registered evidence sources. | Required provenance. |
| `jobs` | Tracks long-running jobs. | Required operational/provenance. |
| `job_events` | Tracks job event history. | Required provenance; currently write-heavy and under-read. |

## Column Classification Summary

| Table | Column Group | Classification |
| --- | --- | --- |
| `videos` | `id`, `source_path`, `relative_path`, `extension`, `size_bytes` | `REQUIRED_CORE` |
| `videos` | `modified_unix`, `sha256`, `hash_status`, `confidence` | `REQUIRED_CORE` / `REQUIRED_REPORT` |
| `videos` | `source_profile_json`, `ffprobe_json`, `record_json` | `REQUIRED_COMPATIBILITY`, possible future normalization candidates |
| `videos` | `first_indexed_unix`, `last_indexed_unix`, `last_scanned_unix` | `REQUIRED_PROVENANCE` |
| `scan_runs` | `warnings_json` | `REQUIRED_REPORT`, improve report use before pruning |
| `job_events` | all fields | `REQUIRED_PROVENANCE`, add read/report surfaces before pruning |

## Existing Indexes

| Index | Purpose |
| --- | --- |
| `scan_runs_scanned_unix_idx` | Scan chronology. |
| `videos_sha256_idx` | Hash lookup and duplicate grouping. |
| `videos_extension_idx` | Extension filters. |
| `videos_last_indexed_idx` | Indexed recency. |
| `videos_modified_unix_idx` | Timeline browsing. |
| `videos_ffprobe_ok_idx` | Validation failure filtering. |
| `videos_confidence_idx` | Triage filtering. |
| `videos_extension_modified_idx` | File type plus timeline filters. |
| `videos_last_scanned_idx` | Scan freshness. |
| `evidence_sources_kind_path_idx` | Source uniqueness. |
| `evidence_sources_hash_idx` | Evidence hash lookup. |
| `jobs_status_idx` | Active/completed job filtering. |
| `jobs_type_started_idx` | Job type chronology. |
| `job_events_job_idx` | Job event timeline. |

## Migration Contract Implemented

- Version table: `schema_meta(key='schema_version')`.
- Migration path: version 1 -> version 2.
- Backup name: `case.db.backup-v1-to-v2-{unix_timestamp}` beside `db/case.db`.
- Migration verification: unit tests assert new-db version 2 initialization and v1 migration with backup creation.
- Rollback: restore the generated backup over `db/case.db` before rerunning the tool.

## Migration Risks

1. SQLite and JSONL/TSV compatibility artifacts can diverge.
2. `record_json` duplicates typed columns but preserves compatibility.
3. `job_events` is written but not yet surfaced enough in reports/viewer.
4. Only v1->v2 exists; any future schema requires a named migration plus fixture.

## Deletion Policy

No table, column, or index is approved for deletion yet. All removals remain `UNUSED_CANDIDATE` until migration fixtures, query-plan evidence, and compatibility checks exist.
