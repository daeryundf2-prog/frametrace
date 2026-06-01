# Performance Validation

FrameTrace is designed for terabyte-scale evidence by avoiding full hashing/probing unless requested and by keeping the primary index in SQLite.

## SQLite Scale Check

Use the synthetic benchmark to validate the local machine and build:

```powershell
.\target\release\frametrace.exe benchmark-db C:\Temp\frametrace-db-bench --rows 1000000
```

The command creates `C:\Temp\frametrace-db-bench\db\case.db` and inserts synthetic video-index rows in a single transaction using the production `videos` schema. It does not claim end-to-end media scan speed; it validates SQLite write-path behavior for large indexes.

## Current macOS Baseline

Latest local validation on macOS (Apple Silicon host, release build):

- Command: `target/release/frametrace benchmark-db /tmp/frametrace-bench-1m --rows 1000000`
- Result: 1,000,000 synthetic `videos` rows inserted in 15.3 seconds.
- Database size: 670 MiB.
- SQLite integrity check: `ok`.

This baseline validates the large SQLite index write path on macOS. It does not replace Windows MSVC validation or terabyte-scale media scan testing against real evidence.

For routine development, use a smaller run:

```powershell
cargo run -- benchmark-db ./target/frametrace-db-bench --rows 10000
```

Run the QA performance check before release review:

```powershell
cargo run -- qa performance ./target/frametrace-performance --rows 100000
```

This writes both `performance-report.json` and `performance-report.md`. The JSON report includes insert throughput, max indexed query latency, representative `EXPLAIN QUERY PLAN` output, and `query_plan_full_scan_count`. The check fails if rows/minute or query latency miss target, or if any representative `videos` query falls back to a full table scan.

## Field Performance Rules

- Start with `scan-folder --no-ffprobe` and no `--hash`.
- Limit depth with `--max-depth` when the media layout is known.
- Hash only selected folders or final evidence subsets.
- Generate thumbnails/proxies lazily.
- Keep the case folder on fast local SSD storage when processing external media.
