# FrameTrace fuzz targets

Coverage-guided fuzzing (cargo-fuzz / libFuzzer) for the hand-rolled parsers
that touch untrusted bytes: recorded evidence metadata, audit logs, and raw
carve input.

## Targets

| Target | Surface |
| --- | --- |
| `audit_jsonl_verify` | `audit::verify_chained_jsonl_text` — hash-chain JSONL verification (`entry_sha256` placement, `previous_entry_sha256` chaining, torn-tail handling) |
| `index_jsonl_records` | `scan::json_record_lines` record splitter + `serde` `VideoRecord` parse + `scan::set_json_field` byte-stable field editor |
| `carve_signature_scan` | `carve::find_video_signatures_in` — chunked MP4/AVI/DHAV/IMKH signature scan with overlap bookkeeping |
| `creation_time_unix` | `timeline::parse_creation_time_unix` — ISO-8601 subset parser for ffprobe `creation_time` tags |

## Running

Requires a nightly toolchain (libFuzzer needs `-Z` flags) and cargo-fuzz:

```sh
rustup toolchain install nightly
cargo install cargo-fuzz
cargo +nightly fuzz run audit_jsonl_verify
```

Run from the repository root. Use `-j` for parallel workers, e.g.
`cargo +nightly fuzz run index_jsonl_records -- -jobs=8`. Crash artifacts land
in `fuzz/artifacts/<target>/`; copy any crash into a regression test before
fixing.

The fuzz crate is its own workspace (`fuzz/Cargo.toml` carries an explicit
`[workspace]`), so the main `cargo build`/`cargo test --locked` never
compiles libfuzzer-sys. The `#[doc(hidden)]` functions the targets call are
fuzz seams, not supported API.
