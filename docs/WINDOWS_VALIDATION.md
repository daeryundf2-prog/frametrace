# Windows Validation

FrameTrace targets Windows 10/11 x64 with the MSVC Rust toolchain.

## Local Windows Commands

Run these from the repository root on Windows:

```powershell
rustup component add clippy rustfmt
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
.\target\release\frametrace.exe benchmark-db C:\Temp\frametrace-db-bench --rows 1000000
```

Then run a smoke case with a small sample video:

```powershell
.\target\release\frametrace.exe init-case C:\Temp\frametrace-case --title "Windows smoke"
.\target\release\frametrace.exe register-source C:\Temp\sample-media --kind folder --write-protect "copied evidence"
.\target\release\frametrace.exe scan-folder C:\Temp\frametrace-case C:\Temp\sample-media --no-ffprobe
.\target\release\frametrace.exe make-review C:\Temp\frametrace-case
.\target\release\frametrace.exe make-report C:\Temp\frametrace-case
.\target\release\frametrace.exe package-case C:\Temp\frametrace-case
.\target\release\frametrace.exe inspect C:\Temp\frametrace-case
```

## CI

The repository includes `.github/workflows/windows-ci.yml`, which runs format, clippy, tests, and a release build on `windows-latest`.

The current macOS validation can prove Rust behavior and command contracts, but the Windows MSVC release build is only fully closed when the workflow or a local Windows machine passes.
