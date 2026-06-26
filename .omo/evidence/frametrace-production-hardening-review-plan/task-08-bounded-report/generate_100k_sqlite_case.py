#!/usr/bin/env python3
import json
import sqlite3
import sys
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: generate_100k_sqlite_case.py CASE_DIR ROWS", file=sys.stderr)
        return 2
    case_dir = Path(sys.argv[1])
    rows = int(sys.argv[2])
    db_dir = case_dir / "db"
    db_dir.mkdir(parents=True, exist_ok=True)
    (case_dir / "case.json").write_text(
        json.dumps(
            {
                "schema_version": 1,
                "case_id": f"FT-MANUAL-{rows}",
                "title": f"FT-MANUAL-{rows}",
                "created_unix": 1,
                "tool_name": "frametrace",
                "tool_version": "manual-qa",
                "platform": "manual-qa",
                "operator": None,
                "host": None,
                "device_id": None,
                "device_serial": None,
                "write_protect": None,
                "acquisition_tool": None,
                "evidence_hash": None,
                "notes": "SQLite-only bounded report manual QA case",
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    conn = sqlite3.connect(db_dir / "case.db")
    conn.executescript(
        """
        CREATE TABLE schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
        INSERT INTO schema_meta (key, value) VALUES ('schema_version', '3');
        CREATE TABLE scan_runs (
            run_pk INTEGER PRIMARY KEY AUTOINCREMENT,
            source_path TEXT NOT NULL,
            scanned_unix INTEGER NOT NULL,
            hash_files INTEGER NOT NULL,
            use_ffprobe INTEGER NOT NULL,
            max_depth INTEGER,
            video_count INTEGER NOT NULL,
            total_bytes INTEGER NOT NULL,
            warnings_json TEXT NOT NULL
        );
        CREATE TABLE videos (
            id TEXT PRIMARY KEY,
            source_path TEXT NOT NULL UNIQUE,
            file_url TEXT NOT NULL,
            relative_path TEXT NOT NULL,
            extension TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            modified_unix INTEGER,
            sha256 TEXT,
            hash_status TEXT NOT NULL,
            confidence TEXT NOT NULL,
            source_profile_json TEXT NOT NULL,
            duration_seconds REAL,
            format_name TEXT,
            video_codec TEXT,
            audio_codec TEXT,
            width INTEGER,
            height INTEGER,
            ffprobe_ok INTEGER NOT NULL,
            ffprobe_error TEXT,
            ffprobe_json TEXT,
            first_indexed_unix INTEGER NOT NULL,
            last_indexed_unix INTEGER NOT NULL,
            last_scanned_unix INTEGER,
            record_json TEXT NOT NULL
        );
        CREATE INDEX videos_inventory_default_idx ON videos (ffprobe_ok, modified_unix, id);
        CREATE INDEX videos_relative_path_idx ON videos (relative_path);
        """
    )
    conn.execute(
        "INSERT INTO scan_runs (source_path, scanned_unix, hash_files, use_ffprobe, max_depth, video_count, total_bytes, warnings_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        ("/evidence/manual-100k", 1, 0, 0, None, rows, rows, "[]"),
    )
    source_profile = json.dumps(
        {
            "lane": "manual-qa",
            "vendor": "Manual QA",
            "parser": "manual_sqlite_generator",
            "confidence": "test",
            "recommended_action": "Use bounded inventory review.",
            "evidence": ["manual QA synthetic row"],
        },
        separators=(",", ":"),
    )
    batch = []
    for index in range(rows):
        video_id = f"vid_{index:06d}"
        rel = f"{index:06d}.mp4"
        source_path = f"/evidence/manual-100k/{rel}"
        record = {
            "id": video_id,
            "source_path": source_path,
            "relative_path": rel,
            "extension": "mp4",
            "size_bytes": 1,
            "hash_status": "skipped",
            "source_profile": json.loads(source_profile),
            "ffprobe_ok": False,
        }
        batch.append(
            (
                video_id,
                source_path,
                f"file://{source_path}",
                rel,
                "mp4",
                1,
                1,
                None,
                "skipped",
                "extension-candidate",
                source_profile,
                None,
                None,
                None,
                None,
                None,
                None,
                0,
                "ffprobe skipped",
                None,
                1,
                1,
                1,
                json.dumps(record, separators=(",", ":")),
            )
        )
        if len(batch) == 1000:
            insert_batch(conn, batch)
            batch.clear()
    if batch:
        insert_batch(conn, batch)
    conn.commit()
    conn.close()
    print(case_dir)
    return 0


def insert_batch(conn: sqlite3.Connection, batch: list[tuple[object, ...]]) -> None:
    conn.executemany(
        "INSERT INTO videos (id, source_path, file_url, relative_path, extension, size_bytes, modified_unix, sha256, hash_status, confidence, source_profile_json, duration_seconds, format_name, video_codec, audio_codec, width, height, ffprobe_ok, ffprobe_error, ffprobe_json, first_indexed_unix, last_indexed_unix, last_scanned_unix, record_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        batch,
    )


if __name__ == "__main__":
    raise SystemExit(main())
