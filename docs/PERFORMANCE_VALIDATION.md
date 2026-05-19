# Performance Validation

FrameTrace is designed for terabyte-scale evidence by avoiding full hashing/probing unless requested and by keeping the primary index in SQLite.

## SQLite Scale Check

Use the synthetic benchmark to validate the local machine and build:

```powershell
.\target\release\frametrace.exe benchmark-db C:\Temp\frametrace-db-bench --rows 1000000
```

The command creates `C:\Temp\frametrace-db-bench\db\case.db` and inserts synthetic video-index rows in a single transaction using the production `videos` schema. It does not claim end-to-end media scan speed; it validates SQLite write-path behavior for large indexes.

For routine development, use a smaller run:

```powershell
cargo run -- benchmark-db ./target/frametrace-db-bench --rows 10000
```

## Field Performance Rules

- Start with `scan-folder --no-ffprobe` and no `--hash`.
- Limit depth with `--max-depth` when the media layout is known.
- Hash only selected folders or final evidence subsets.
- Generate thumbnails/proxies lazily.
- Keep the case folder on fast local SSD storage when processing external media.
