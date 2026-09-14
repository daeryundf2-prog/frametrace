# Real E01 Validation Receipt

Date: 2026-09-14
Scope: C-2 — real acquired E01 image through the libewf pipeline

## Toolchain

- libewf 20240506 (ewfinfo/ewfverify/ewfexport) built from source via MSYS2
  UCRT64 (`scripts/build-libewf-tools.ps1` equivalent flow; the script's
  pacman-key step needed manual continuation after first-run bootstrap).
- Sleuth Kit 4.15.0-win32 binaries found on the analysis machine
  (`D:\rapid-triage-test\sleuthkit\...`) and staged into `tools/bin`.
- Tools deployed to `tools/bin` + `target/debug/{,deps}/tools/bin` so
  `tool_policy::resolve_tool_binary` picks them up.

## Evidence

- `D:\김원태, 한수일\이미지\4. 한주연 HDD\4. 한주연 HDD.E01` (+ .E02, .E03)
  — 3-segment FTK Imager set, ADI4.7.1.2, fixed disk, 931 GiB
  (1,000,204,886,016 bytes), deflate/no-compression, MD5+SHA1 present.

## Results

| Step | Command | Result |
| --- | --- | --- |
| Case init | `init-case` | OK |
| Inspect | `inspect-e01` | OK — ewfinfo parsed media size, MD5/SHA1, acquisition metadata; log written to `evidence/logs/e01-info-*.txt` |
| Import (bounded) | `import-e01 --skip-verify --max-bytes 2147483648` | OK — raw export + SHA-256 + audit entry |
| FS inspect | `inspect-image` | Failed as expected — partial 2 GiB export has no $MFT (mmls read the DOS partition table correctly: NTFS @ sector 2048) |
| Carve | `carve-file --max-bytes 67108864` | OK — 0 candidates in the truncated NTFS-resident prefix; job progress recorded to `jobs` (2147483648/2147483648) |
| Index | `scan-folder --hash` on 3 real MP4s | OK — 3 indexed, 69,350,948 bytes |
| Report/package | `make-review`, `make-report`, `package-case` | OK — 20-file package with manifest + checksums |
| Audit | `verify-audit evidence/logs/e01-audit.jsonl` | OK — integrity-structural-only |
| ITs | `FRAMETRACE_IT=1` integration + workstation E2E + perf budgets | All pass with real libewf/ffmpeg |

## Bugs found by real-E01 validation (fixed)

1. **ewfexport output name with dotted basenames** — for
   `4. 한주연 HDD.E01` the derived target prefix `4. 한주연 HDD` made
   `with_extension("raw")` produce `4.raw` while ewfexport wrote
   `4. 한주연 HDD.raw` → post-export resolution failed. Fixed by treating
   `-t` as a prefix and only stripping a literal `.raw` suffix
   (`ewfexport_target_for_output` / `expected_ewfexport_output`).
2. **Disk preflight ignored `--max-bytes`** — the check demanded the full
   media size even for bounded exports. Fixed: `min(media_size, max_bytes)`.

## Notes

- `verify-audit` takes the *log file path*, not the case dir — passing the
  case dir surfaces `access denied` on Windows rather than a usage hint;
  worth a friendlier error but not blocking.
- A hung `import-e01` (killed mid-export) leaves its `jobs` row in
  `running` forever; `latest_running_job` picks the newest running row so a
  later live job still reports correctly, but a stuck orphan would shadow
  it. Acceptable for now — consider a `jobs` status sweeper on case open.
- Full 931 GiB export not attempted (time); the bounded export exercised
  the same code path (ewfexport args, output resolution, hashing, audit).

## L01 (EnCase 7 logical evidence) — same session

- `D:\김원태, 한수일\이미지\topplus.L01` — LEF EnCase 7 v2, 64 GiB logical,
  4 KiB sectors.
- `inspect`-equivalent via ewfinfo: OK (case number, examiner, set
  identifier parsed).
- `import-e01 --skip-verify --max-bytes 1073741824`: OK — `topplus.raw`
  written, SHA-256 + audit entry, `src_*` source registered as `e01`.
- `carve-file --max-bytes 33554432` on the L01 raw export: **3 MP4 ftyp
  candidates carved from real evidence** (offsets 51 318 / 178 447 809 /
  234 078 087), each labeled `candidate-unvalidated`.
- `validate-artifact` on all three: `validation-failed` — correct behavior.
  The candidates carry a valid `ftyp mp42` head but were truncated at the
  32 MiB carve bound before `moov`; ffprobe legitimately cannot parse a
  headless MP4. The pipeline kept them as candidates with the explicit
  "verify moov/mdat before reporting as recovered" note — honest labeling
  verified end-to-end on real evidence.

## Second pass — damaged sets, complete-image pipeline, carve-to-playable

### Damaged/incomplete segment fixtures

- Missing segments: hardlinked `case.E01` (first segment of the #453 set)
  into an empty dir. `ewfinfo` reports `Is corrupted: yes`; `inspect-e01`
  previously reported success with no corruption signal. **Fixed:**
  `ewfinfo_is_corrupted` parser added; `inspect-e01` prints a warning and
  the flag is recorded in the audit event (`is_corrupted`) for both
  inspect and import.
- Truncated first segment (64 MiB head): `ewfverify` fails immediately,
  `import-e01` surfaces `ewfverify failed: Unable to verify input` and the
  job row lands as `failed` with the libewf error text.
- A killed `import-e01` left `jobs.status = 'running'` forever. **Fixed:**
  each job now holds an fs2 advisory lock under `db/job-locks/`; locks die
  with the process, so the next `start_job`/`latest_running_job` reaps
  orphaned rows to `interrupted`. Verified live: the killed import flipped
  to `interrupted` when a later carve job started.

### Complete-image end-to-end (synthetic FAT16 fixture)

`make_fat16.py` builds a 96 MiB MBR+FAT16 image: `LIVE.MP4` (6.5 MB,
inode 3) plus a deleted `_ELETED.MP4` (19.4 MB, inode 4) with intact FAT
chains. Acquired to `synth.E01` (EnCase 7, MD5+SHA256) via `ewfacquire`.

- `import-e01` **with ewfverify** completed; exported raw SHA-256 equals
  ewfacquire's hash over the source image — full provenance chain.
- `inspect-image`: mmls finds the DOS FAT16 partition at sector 2048;
  fls lists 6 entries incl. 1 deleted; 2 video candidates.
- `recover-inode 3`: byte-identical to the source MP4 (SHA-256 match).
  Inode 4 (deleted) yields the first cluster only — correct icat/FAT16
  semantics; full recovery is carving's job.
- `carve-file` on the raw export: 2 MP4 candidates, both
  `ffprobe-video-stream-confirmed` after `validate-artifact` — including
  the deleted file. Carve → validated playable video verified end-to-end.

### Real L01 re-carve at 256 MiB bound

`topplus.raw` re-carved with `--max-bytes 268435456`: the two candidates
that failed as 32 MiB truncations now validate as
`ffprobe-video-stream-confirmed` (178 MB and 55 MB). The 268 MiB one still
hits the cap and correctly remains `validation-failed` — larger media
needs a proportionally larger bound, which the receipt now documents.

### Partial physical-image carve (#453 set, first 4.9 GiB of a 223 GiB HDD)

64 candidates from the interrupted export: 61 `mp4-ftyp`, 2 `dahua-dhav`,
1 `riff-avi`. The two DAV hits carry `DHAV` magic but random frame data —
`export-dav` correctly rejects them (`frame at offset 0 extends past
EOF`); a signature hit is not a file, and the pipeline says so.

### Large real export attempt

`import-e01 --skip-verify` on the 223 GiB #453 set ran to ~4.9 GiB raw
before being stopped to bound session time — multi-segment export works
on real hardware but a full export of a large drive needs hours, not a
code change.

## Third pass — format coverage via ewfacquire-generated fixtures

All fixtures derived from `synth.img` (96 MiB MBR+FAT16, MD5
`61796c92…`, SHA-256 `90ea89fa…`).

| Format | ewfacquire flag | Extension | Result |
| --- | --- | --- | --- |
| E01 (EnCase 7) | `-f encase7` | .E01 | verify+export byte-identical to source |
| S01 (SMART) | `-f smart` | .s01 | verify+export byte-identical to source |
| L01 (logical) | `-f linen7` | (.E01 for physical source) | earlier real L01 import OK |
| Ex01 (EnCase 7 v2) | `-f encase7-v2` | .Ex01 | **export NOT bit-faithful in sparse regions** |

### Ex01 sparse-fill deviation (libewf 20240506)

The Ex01 export differs from the source only where the source is all-zero:
every 8-byte slot gets a 2-byte marker (`d0 81` near the start, values vary
by region — likely empty-block bookkeeping leaking into the raw stream).
36.7 M diff bytes, all in zero regions; the LIVE.MP4 data region is
byte-identical and `inspect-image` on the export works fully. Consequence:
`raw_sha256` in the audit log will NOT match the acquisition hash for Ex01
sources — the difference is expected tooling behavior, not tampering, and
ewfverify still validates the container itself. Recorded here so a future
examiner does not misread the mismatch.

### Missing middle segment

4-segment set (`-S 32 MiB`) with `.E02` removed:

- `inspect-e01` → `is_corrupted` warning surfaced.
- `import-e01` (verify on) → `ewfverify` fails exactly at the gap
  (offset 0x01ff8000); job row `failed` with libewf error text.
- `import-e01 --skip-verify` → `ewfexport` fails at the same boundary and
  the partial `.raw` is removed — torn output cannot masquerade as a
  completed export (a `.raw.info` ewfexport sidecar does remain behind).

## Fourth pass — many-segment image, GPT truncation boundary, partial-image triage

Evidence: `D:\한종인\#914_260812_PC_003\#914_260812_PC_SSD_004\Image\#914_260812_PC_SSD_004.E01`
(60-segment FTK set, Samsung 870 EVO 500 GB, 465 GiB media, MD5+SHA1) plus
16 MiB head exports of six other real sets (`BSH`, `HDD3`, `HJI`, `HJY`,
`NEXSTIN`, `PCG1`) and a 32 GiB bounded export of SSD_004.

### Bugs found and fixed

1. **ewfinfo probe timeout on large segment sets** — `inspect-e01` /
   `import-e01` forced a 120 s probe cap; this 60-segment set legitimately
   needs 8 min 21 s to parse. `E01Options.timeout_secs` is now honored
   (default `None` = unbounded, matching export semantics); `--timeout`
   still caps when set.
2. **GPT auto-selection picked Microsoft Reserved** — win32 mmls prints an
   empty description for the data partition on this image (non-ASCII GPT
   name), so keyword matching fell back to `allocated.first()` = the MSR
   at sector 34 → `fls` failed. Selection now prefers the largest
   allocated non-reserved slot → offset 32768, and `inspect-image` on the
   32 GiB export listed the real NTFS root ($MFT, $Extend, ~41 k entries).
3. **Truncated-GPT layout produced a cryptic error** — TSK's
   `gpt_load_table` discards the whole GPT when the first two populated
   entries start beyond the image bounds, leaving only the protective-MBR
   "GPT Safety Partition (0xee)" slot; `inspect-image` then ran fls at
   offset 1 and failed with `Cannot determine file system type`. All five
   16 MiB heads hit this. Now detected up front with a clear cause/remedy
   message ("partial export; larger --max-bytes or --partition-offset").
4. **fls failure gave no partial-image hint** — on the 6 GiB BSH head the
   NTFS volume opens (boot sector + $MFT at ~3.5 GiB in-range) but the
   root directory index sits ~8 GiB in → `Address missing in partial
   image`. When the chosen partition extends past the image end, the error
   now says so explicitly.
5. **`verify-audit <case-dir>` died on the directory read** — now
   discovers and verifies every `evidence/logs/*.jsonl` in the case,
   failing if any log fails.
6. **ffprobe `bin`/`bintext` guess counted as confirmed video** — a
   recovered NTFS `tracking.log` probed as format `bin` (score 50) with a
   "video" stream and was labeled `ffprobe-video-stream-confirmed`. The
   catch-all binary demuxer is a guess, not container recognition — now
   `validation-failed` with a keep-as-candidate note.
7. **Package README/manifest pointed at a report that may not exist** —
   both now state honestly when `reports/case-report.html` was not
   generated and name the producing command (`make-report`).
8. **All-zero recoveries presented as content** — `recover-inode` on
   TRIMmed/unallocated clusters (routine on SSD images) produced e.g.
   6.58 MB of zeros with no signal. Now emits an explicit warning in CLI
   output and the `recover-inode` audit event.

### Verified on the 32 GiB SSD_004 partial export

- `inspect-image`: GPT table parsed, MSR skipped, NTFS @ 32768 listed —
  41,453 entries (truncated to 20,000), 19,946 deleted entries
  (dominated by `$OrphanFiles`), 0 video candidates (the only `.mp4`-named
  items were Unity `.mp4.bundle` assets — correctly not flagged).
- `recover-inode 43-128-3` (`System Volume Information/tracking.log`):
  20,480 bytes of real content recovered; SHA-256 logged.
- `verify-audit case-ssd004`, `make-report`, `package-case` (44 files,
  manifest+checksums) — all OK.
- `carve-file --max-bytes 268435456` on the 34 GiB export: complete —
  0 candidates in the OS/system region prefix; job recorded cleanly.

### TSK reads E01 sets directly — no export needed for filesystem triage

`inspect-image`, `recover-inode`, and `recover-batch` run against the raw
`.E01` path and TSK 4.15 decompresses via libewf internally — the full
60-segment, 465 GiB SSD_004 set listed its NTFS tree and recovered files
without any raw export:

- `inspect-image <E01>` → same partition pick (32768), same 41 k-entry
  listing as the raw export.
- `recover-inode 43-128-3` → identical SHA-256 to the export-derived
  recovery.
- `recover-batch` on a 3-item selection → 2 ok, 1 failed
  (`Metadata address too large` for a nonexistent inode); batch audit
  logged, job completed.
- Deleted-file recoveries on this SSD return all-zero bytes (TRIMmed
  clusters) — `recover-inode` now flags `recovered content is entirely
  zero bytes` instead of presenting slack as content.

Note: `carve-file` still needs a linear image — signature scanning inside
a compressed E01 container is meaningless.

### Partial-export boundary reference (measured)

- GPT table itself: visible iff the first two populated GPT entries start
  within the export (TSK rule). EFI@2048 + MSR@534528 → >273 MiB needed;
  MSR@34 + data@32768 → >16 MiB needed.
- `fls` usability: needs volume boot sector + $MFT + the directory index
  extents. SSD_004: fully listed at 32 GiB. BSH: $MFT at ~3.5 GiB but
  root index ~8 GiB → 6 GiB export still aborts mid-walk.

## Still unvalidated

- ewfverify end-to-end on a real *large* image — relaunched against the
  #453 set in the background; bounded by disk throughput (223 GiB
  decompress+hash). The verify path itself is proven on the fixture.
- Real-recorder DAV: carved candidates on a real disk proved to be
  signature-only false positives, not files — still needs an actual DVR
  export sample.
- Multi-segment L01 (`.L02`+): only single-segment L01 seen so far.
