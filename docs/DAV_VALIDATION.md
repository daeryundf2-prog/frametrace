# DAV Field Validation Receipt

Date: 2026-09-05  
Scope: ROADMAP M2-1 real-sample gate (track A)

## Done in this pass

1. Aligned `src/dav.rs` to FFmpeg `libavformat/dhav.c`:
   - optional `DAHUA` 0x400 preamble
   - frame types `0xFD`/`0xFC` video, `0xF0` audio, `0xF1` skip
   - footer magic `dhav` (not the previous incorrect `0xDC 0x4D 0x44 0x00`)
2. Remux prefers `ffmpeg -i file.dav -c copy` (native DHAV demuxer); ES extract is fallback.
3. Restored missing `scripts/validate-dav-samples.ps1` (documented in ROADMAP / portable README but previously absent from the tree).
4. Unit tests: 6/6 pass. Integration: `dav_export_remuxes_real_h264_and_validates` pass with `FRAMETRACE_IT=1`.

## Update — 2026-09-20 (false-positive fix)

1. **Type gate added**: `export-dav` now requires `DAHUA`/`DHAV`+valid frame-type
   magic up front (`is_dav_header` in `src/dav.rs`) and the native remux path
   forces `ffmpeg -f dhav`. Previously a plain MP4 passed through
   `ffmpeg -i … -c copy` and was miscounted as a DAV export.
2. **Harness now pre-classifies** every sample by container magic:
   `pass` / `fail` / `not-dav` are counted separately, and exit code only
   reaches 0 with ≥3 genuine DAV candidates and zero failures.
3. **Corpus re-run** (`case-pcg2` carved output, 129 files): **0 pass / 0 fail /
   129 not-dav** — the earlier "14 passes" were mislabeled ordinary MP4s, and
   the `.dav`-named files were false-positive carves (DHAV prefix + base64
   payload). Honest result: **no genuine DAV sample exists on this machine**.
4. Unit test `header_gate_rejects_mp4_and_accepts_dav` locks the gate in.

## Blocked

- No recorder-exported `.dav` files on this workstation.
- Public OSS fixtures do not exist (surveillance sensitivity).
- Harness result against empty intake folder:

```text
powershell -File scripts/validate-dav-samples.ps1 -Samples C:\Temp\frametrace-dav-samples
→ exit 2 BLOCKED: no .dav files
```

## Examiner action to close the gate

Place ≥3 real exports (continuous / event / parking) under a local folder outside git, then:

```powershell
cargo build --release
powershell -File scripts/validate-dav-samples.ps1 -Samples <folder> -Exe .\target\release\frametrace.exe
```

Pass criteria: all genuine-DAV samples remux + `ffprobe-video-stream-confirmed`; exit 0 only when ≥3 samples carry real `DAHUA`/`DHAV` magic and fail = 0 (non-DAV files are reported as `not-dav`, never as passes).
