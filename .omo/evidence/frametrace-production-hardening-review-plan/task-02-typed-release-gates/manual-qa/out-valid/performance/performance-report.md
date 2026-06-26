# Performance Report

- Status: PASS
- Rows: 100000
- Elapsed ms: 4923
- Rows per minute: 1218769 (target: 50000)
- Max indexed query ms: 4 (target: 2000)
- Query plan full scans: 0
- Max RSS: 8.86 MiB (target: 2560.00 MiB)
- Average CPU: 99.45% (target: 95%, enforced: false)
- Resource samples: 46
- Database: `.omo/evidence/frametrace-production-hardening-review-plan/task-02-typed-release-gates/manual-qa/out-valid/performance/db/case.db`

## Resource Metrics

| Metric | Value | Target | Status |
| --- | ---: | ---: | --- |
| Max RSS | 8.86 MiB | 2560.00 MiB | PASS |
| Average CPU | 99.45% | 95% | MEASURED_NOT_GATED |

## Query Plans

### extension_count

```sql
EXPLAIN QUERY PLAN SELECT COUNT(*) FROM videos WHERE extension = ?1
```

```text
SEARCH videos USING COVERING INDEX videos_extension_modified_idx (extension=?)
```

### source_lookup

```sql
EXPLAIN QUERY PLAN SELECT id FROM videos WHERE source_path = ?1
```

```text
SEARCH videos USING INDEX sqlite_autoindex_videos_2 (source_path=?)
```

### sha256_lookup

```sql
EXPLAIN QUERY PLAN SELECT id FROM videos WHERE sha256 = ?1
```

```text
SEARCH videos USING INDEX videos_sha256_idx (sha256=?)
```

### timeline_recent

```sql
EXPLAIN QUERY PLAN SELECT id FROM videos WHERE extension = ?1 ORDER BY modified_unix DESC LIMIT 100
```

```text
SEARCH videos USING INDEX videos_extension_modified_idx (extension=?)
```

### inventory_validation_candidates

```sql
EXPLAIN QUERY PLAN SELECT id FROM videos WHERE ffprobe_ok = ?1 ORDER BY modified_unix ASC, id ASC LIMIT 100
```

```text
SEARCH videos USING COVERING INDEX videos_inventory_default_idx (ffprobe_ok=?)
```

### inventory_hash_state

```sql
EXPLAIN QUERY PLAN SELECT id FROM videos WHERE hash_status = ?1 ORDER BY id LIMIT 100
```

```text
SEARCH videos USING COVERING INDEX videos_hash_status_idx (hash_status=?)
```

### inventory_path_prefix

```sql
EXPLAIN QUERY PLAN SELECT id FROM videos WHERE relative_path >= ?1 AND relative_path < ?2 ORDER BY relative_path LIMIT 100
```

```text
SEARCH videos USING INDEX videos_relative_path_idx (relative_path>? AND relative_path<?)
```

### inventory_recent_since

```sql
EXPLAIN QUERY PLAN SELECT id FROM videos WHERE modified_unix >= ?1 ORDER BY modified_unix ASC LIMIT 100
```

```text
SEARCH videos USING INDEX videos_modified_unix_idx (modified_unix>?)
```

