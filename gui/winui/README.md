# FrameTrace WinUI Shell

Thin Windows desktop shell. **Evidence logic stays in `frametrace.exe`.**

## Prerequisites

- .NET SDK 8+ (user-local install is fine):
  `powershell -File https://dot.net/v1/dotnet-install.ps1` style, or
  already under `%LOCALAPPDATA%\Microsoft\dotnet`
- Windows App SDK templates:
  `dotnet new install Microsoft.WindowsAppSDK.WinUI.CSharp.Templates`
- Release engine nearby: `target/release/frametrace.exe` (+ optional `frametrace-app.exe`)

## Build / run

```powershell
$env:Path = "$env:LOCALAPPDATA\Microsoft\dotnet;$env:Path"
cd gui/winui
dotnet build -c Release -p:Platform=x64
dotnet run -c Release -p:Platform=x64
```

## Behavior

- Launch examiner workstation → starts `frametrace-app.exe` (falls back to `frametrace.exe`).
- `make-review` / `make-report` / `qa anomalies` → subprocess to CLI with the case folder.
- Does not reimplement scan/carve/validation.
