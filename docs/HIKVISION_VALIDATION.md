# Hikvision IMKH Lane Receipt

Date: 2026-09-05  
Scope: ROADMAP M4 item 6 (first implementable slice without HDD corpus)

## Implemented

1. `src/hikvision.rs` — detect `IMKH` magic, strip 40-byte header, remux with
   `ffmpeg -fflags +genpts+discardcorrupt -c copy`.
2. CLI `export-hik` — audited export to `artifacts/clips/` + stripped payload under
   `artifacts/carved/`.
3. Scan magic recognizes `IMKH` / `DHAV` without relying on extension alone.
4. Carve signature `hikvision-imkh`.
5. Detector path needles expanded (`hikcentral`); recommended action points at `export-hik`.
6. Intake harness: `scripts/validate-hik-samples.ps1`.
7. Integration test: synthetic IMKH + MPEG-PS → export-hik → ffprobe confirmed
   (`FRAMETRACE_IT=1`).

## Explicitly out of scope (still)

- Proprietary Hikvision HDD filesystem (`HIKVISION@HANGZHOU`) recovery — needs real
  disk/image corpus and separate legal/forensic review.
- Encrypted / SDK-only player packages.

## Field gate

Place IMKH-prefixed iVMS/NVR downloads outside git, then:

```powershell
cargo build --release
powershell -File scripts/validate-hik-samples.ps1 -Samples <folder> -Exe .\target\release\frametrace.exe
```
