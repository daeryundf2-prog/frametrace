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
.\target\release\frametrace.exe scan-folder C:\Temp\frametrace-case C:\Temp\sample-media --hash
.\target\release\frametrace.exe validate-artifact C:\Temp\frametrace-case vid_000001
.\target\release\frametrace.exe confirm-playback C:\Temp\frametrace-case vid_000001 --playback-tool "Windows Media Player"
.\target\release\frametrace.exe make-review C:\Temp\frametrace-case
.\target\release\frametrace.exe make-report C:\Temp\frametrace-case
.\target\release\frametrace.exe package-case C:\Temp\frametrace-case
.\target\release\frametrace.exe inspect C:\Temp\frametrace-case
.\target\release\frametrace.exe workstation-status C:\Temp\frametrace-case
```

Or run the full release smoke script:

```powershell
scripts\windows\validate-release.ps1
```

The release smoke script fails closed on non-Windows hosts, non-MSVC Rust hosts, missing `rustc`, `cargo`, `ffmpeg`, `ffprobe`, `dotnet`, missing concrete WinUI `.sln`/`.csproj` files, or missing WinUI test project. It runs `dotnet build` and `dotnet test`, writes `reports\qa\winui-build.json`, and only then runs `qa release`. It also asserts that `qa release` records `workstation_shell_contract` and `windows_prerequisites`, and writes `reports\qa\workstation-status.json`. That artifact is the WinUI/HTML shell readiness proof: UI durable state is forbidden, Rust/SQLite/audit remain the source of truth, large inventories must use bounded queries rather than full JSON browser loads, and Windows readiness is not claimed until the prerequisite gate passes.

## CI

The repository includes `.github/workflows/windows-ci.yml`, which runs format, clippy, tests, and a release build on `windows-latest`.

The current macOS validation can prove Rust behavior and command contracts, but the Windows MSVC release build is only fully closed when the workflow or a local Windows machine passes.
