param(
    [string]$CaseRoot = "$env:TEMP\frametrace-engine-smoke",
    [int]$PerformanceRows = 1000
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Invoke-Step {
    param(
        [string]$Name,
        [scriptblock]$StepAction
    )
    Write-Host "== $Name"
    & $StepAction
}

function Invoke-NativeStep {
    param(
        [string]$Name,
        [scriptblock]$NativeAction
    )
    $WrappedAction = {
        & $NativeAction
        if ($LASTEXITCODE -ne 0) {
            throw "$Name failed with exit code $LASTEXITCODE"
        }
    }.GetNewClosure()
    Invoke-Step $Name $WrappedAction
}

function Require-Command {
    param([string]$Name)
    $Command = Get-Command $Name -ErrorAction SilentlyContinue
    if (-not $Command) {
        throw "missing required command: $Name"
    }
    return $Command.Source
}

function Assert-File {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "expected file missing: $Path"
    }
}

function Assert-Contains {
    param(
        [string]$Path,
        [string]$Needle
    )
    if (-not (Select-String -LiteralPath $Path -SimpleMatch $Needle -Quiet)) {
        throw "expected '$Needle' in $Path"
    }
}

function Write-EngineValidationReceipt {
    param(
        [string]$Path,
        [string]$Status,
        [array]$Blockers,
        [hashtable]$Details
    )
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Path) | Out-Null
    [ordered]@{
        schema_version = 1
        qa_type = "windows_engine_validation"
        status = $Status
        checked_at = (Get-Date).ToUniversalTime().ToString("o")
        host = [ordered]@{
            os = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
            architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
        }
        blockers = @($Blockers)
        details = $Details
        downstream = [ordered]@{
            t8 = "N/A until this receipt is PASS on real Windows 10/11 x64 MSVC"
            t9 = "N/A until this receipt is PASS on real Windows 10/11 x64 MSVC"
            t10 = "N/A until this receipt is PASS on real Windows 10/11 x64 MSVC"
            t11 = "N/A until this receipt is PASS on real Windows 10/11 x64 MSVC"
            t12 = "If this status is BLOCKED, T12 must write release-decision.json as BLOCKED"
        }
    } | ConvertTo-Json -Depth 8 | Out-File -LiteralPath $Path -Encoding utf8
}

function Test-RunningOnWindows {
    $IsWindowsVariable = Get-Variable IsWindows -Scope Global -ErrorAction SilentlyContinue
    if ($null -ne $IsWindowsVariable) {
        return [bool]$IsWindowsVariable.Value
    }
    return [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
        [System.Runtime.InteropServices.OSPlatform]::Windows
    )
}

function ConvertTo-ComparablePath {
    param([string]$Path)
    return [System.IO.Path]::GetFullPath($Path).TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    )
}

function Test-SamePath {
    param(
        [string]$Left,
        [string]$Right
    )
    if ([string]::IsNullOrWhiteSpace($Left) -or [string]::IsNullOrWhiteSpace($Right)) {
        return $false
    }
    return [StringComparer]::OrdinalIgnoreCase.Equals(
        (ConvertTo-ComparablePath $Left),
        (ConvertTo-ComparablePath $Right)
    )
}

function Test-FullyQualifiedPath {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $false
    }

    try {
        $FullPath = [System.IO.Path]::GetFullPath($Path)
        $RootPath = [System.IO.Path]::GetPathRoot($Path)
    } catch {
        return $false
    }

    if ([string]::IsNullOrWhiteSpace($RootPath)) {
        return $false
    }
    if ($Path -match '^[\\/](?![\\/])') {
        return $false
    }
    if ($Path -match '^[A-Za-z]:($|[^\\/])') {
        return $false
    }

    $ComparableInput = $Path.TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    )
    $ComparableFull = $FullPath.TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    )
    return [StringComparer]::OrdinalIgnoreCase.Equals($ComparableInput, $ComparableFull)
}

function Assert-SafeCaseRoot {
    param(
        [string]$Path,
        [string]$RepoRoot
    )
    if ([string]::IsNullOrWhiteSpace($Path)) {
        throw "unsafe CaseRoot: value must be a non-empty frametrace-* scratch directory"
    }

    $ExpandedPath = [Environment]::ExpandEnvironmentVariables($Path)
    if (-not (Test-FullyQualifiedPath $ExpandedPath)) {
        throw "unsafe CaseRoot: path must be fully qualified"
    }

    $FullPath = ConvertTo-ComparablePath $ExpandedPath
    $LeafName = Split-Path -Leaf $FullPath
    if ($LeafName -notlike "frametrace-*") {
        throw "unsafe CaseRoot: leaf directory must match frametrace-*"
    }

    $RootPath = ConvertTo-ComparablePath ([System.IO.Path]::GetPathRoot($FullPath))
    if (Test-SamePath $FullPath $RootPath) {
        throw "unsafe CaseRoot: drive or filesystem root is not allowed"
    }
    if (Test-SamePath $FullPath $RepoRoot) {
        throw "unsafe CaseRoot: repository root is not allowed"
    }

    $UserProfile = [Environment]::GetFolderPath([Environment+SpecialFolder]::UserProfile)
    if (Test-SamePath $FullPath $UserProfile) {
        throw "unsafe CaseRoot: user profile root is not allowed"
    }

    $TempRoot = [System.IO.Path]::GetTempPath()
    if (Test-SamePath $FullPath $TempRoot) {
        throw "unsafe CaseRoot: temp root itself is not allowed"
    }

    return $FullPath
}

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $RepoRoot

# This script recursively clears CaseRoot; constrain it to an explicit
# frametrace-* scratch directory before any destructive operation.
$CaseRoot = Assert-SafeCaseRoot $CaseRoot $RepoRoot
Remove-Item -LiteralPath $CaseRoot -Recurse -Force -ErrorAction SilentlyContinue
$CaseDir = Join-Path $CaseRoot "case"
$SourceDir = Join-Path $CaseRoot "source-media"
$QaReportDir = Join-Path $CaseDir "reports\qa"
$ReceiptPath = Join-Path $QaReportDir "windows-engine-validation.json"
$ToolPaths = [ordered]@{}
$Details = @{
    case_root = $CaseRoot
    case_dir = $CaseDir
    performance_rows = $PerformanceRows
    tools = $ToolPaths
    scenarios = @()
}

try {
    if (-not (Test-RunningOnWindows)) {
        throw "Windows engine validation must run on Windows 10/11 x64."
    }

    foreach ($CommandName in @("rustc", "cargo", "ffmpeg", "ffprobe")) {
        $ToolPaths[$CommandName] = Require-Command $CommandName
    }

    $RustVersion = rustc -vV | Out-String
    $Details["rustc_vv"] = $RustVersion.Trim()
    if ($RustVersion -notmatch "host: .*windows-msvc") {
        throw "Rust MSVC toolchain required for Windows engine validation."
    }

    Invoke-NativeStep "rust format" { cargo fmt --all -- --check }
    Invoke-NativeStep "rust check" { cargo check --locked }
    Invoke-NativeStep "rust clippy" { cargo clippy --locked --all-targets --all-features -- -D warnings }
    Invoke-NativeStep "rust tests" { cargo test --locked }
    Invoke-NativeStep "release build" { cargo build --release --locked }

    $Binary = Join-Path $RepoRoot "target\release\frametrace.exe"
    Assert-File $Binary
    New-Item -ItemType Directory -Force -Path $SourceDir | Out-Null

    $SampleVideo = Join-Path $SourceDir "sample.mp4"
    Invoke-NativeStep "synthetic mp4 fixture" {
        ffmpeg -hide_banner -loglevel error -y -f lavfi -i testsrc=size=160x90:rate=1 -t 1 -pix_fmt yuv420p $SampleVideo
    }

    Invoke-Step "engine workflow" {
        Invoke-NativeStep "init case" { & $Binary init-case $CaseDir --title "Windows engine smoke" --operator "engine-validator" }
        Invoke-NativeStep "register source" { & $Binary register-source $CaseDir $SourceDir --kind folder --write-protect "synthetic fixture" }
        Invoke-NativeStep "scan folder" { & $Binary scan-folder $CaseDir $SourceDir --hash }
        Invoke-NativeStep "repeated scan" { & $Binary scan-folder $CaseDir $SourceDir --hash }
        Invoke-NativeStep "validate artifact" { & $Binary validate-artifact $CaseDir vid_000001 --operator "engine-validator" }
        Invoke-NativeStep "confirm playback" { & $Binary confirm-playback $CaseDir vid_000001 --operator "engine-validator" --playback-tool "Windows Media Player" --notes "engine validation playback confirmation" }
        Invoke-NativeStep "bounded inventory" { & $Binary inventory $CaseDir --limit 10 }
        Invoke-NativeStep "inventory facets" { & $Binary inventory $CaseDir --facets }
        Invoke-NativeStep "inventory bulk preview" { & $Binary inventory-bulk-preview $CaseDir --action add-to-report --operator "engine-validator" vid_000001 }
        Invoke-NativeStep "inventory export manifest" { & $Binary inventory-export-manifest $CaseDir --operator "engine-validator" --output (Join-Path $CaseDir "reports\inventory-export.json") vid_000001 }
        Invoke-NativeStep "workstation status" { & $Binary workstation-status $CaseDir }
    }

    $UnicodePathSuffix = -join ([char[]](0xAE34, 0xACBD, 0xB85C))
    $UnicodeFilePrefix = -join ([char[]](0xAC80, 0xC99D))
    $UnicodeDir = Join-Path $CaseRoot "source-media-unicode-$UnicodePathSuffix"
    New-Item -ItemType Directory -Force -Path $UnicodeDir | Out-Null
    Copy-Item -LiteralPath $SampleVideo -Destination (Join-Path $UnicodeDir "$UnicodeFilePrefix-sample-long-path-name-000000000000000000000000000000000000000000000000000000000000.mp4")
    Invoke-NativeStep "unicode long path scan" { & $Binary scan-folder $CaseDir $UnicodeDir --hash }

    $LockPath = Join-Path $SourceDir "locked-sample.mp4"
    Copy-Item -LiteralPath $SampleVideo -Destination $LockPath
    $LockStream = [System.IO.File]::Open($LockPath, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::None)
    try {
        Invoke-NativeStep "file lock scan" { & $Binary scan-folder $CaseDir $SourceDir --hash }
    } finally {
        $LockStream.Dispose()
    }

    $ValidationLog = Join-Path $CaseDir "evidence\logs\validation-log.jsonl"
    Assert-File $ValidationLog
    Assert-Contains $ValidationLog '"validation_status":"ffprobe-video-stream-confirmed"'
    Assert-Contains $ValidationLog '"validation_status":"playback-confirmed"'

    New-Item -ItemType Directory -Force -Path $QaReportDir | Out-Null

    $StatusPath = Join-Path $QaReportDir "workstation-status.json"
    & $Binary workstation-status $CaseDir | Out-File -LiteralPath $StatusPath -Encoding utf8
    if ($LASTEXITCODE -ne 0) {
        throw "workstation status failed with exit code $LASTEXITCODE"
    }
    Assert-Contains $StatusPath '"view":"workstation-status"'
    Assert-Contains $StatusPath '"engine_source_of_truth":true'
    Assert-Contains $StatusPath '"full_json_load_allowed":false'
    Assert-Contains $StatusPath '"ffprobe_and_playback_are_separate_states":true'
    Assert-Contains $StatusPath '"playback_confirmed_count":1'

    $InventoryPath = Join-Path $QaReportDir "inventory-page.json"
    & $Binary inventory $CaseDir --limit 10 | Out-File -LiteralPath $InventoryPath -Encoding utf8
    if ($LASTEXITCODE -ne 0) {
        throw "inventory page failed with exit code $LASTEXITCODE"
    }
    Assert-Contains $InventoryPath '"view":"inventory"'
    Assert-Contains $InventoryPath '"page_size":10'

    Invoke-NativeStep "performance smoke" { & $Binary qa performance (Join-Path $CaseRoot "qa-performance") --rows $PerformanceRows }

    $Details["scenarios"] = @(
        "rust_fmt_check_clippy_test_build",
        "ffmpeg_ffprobe_discovery",
        "synthetic_mp4_validation_probe",
        "validation_playback_separation",
        "unicode_long_path_scan",
        "repeated_scans",
        "file_lock_scan",
        "bounded_inventory",
        "workstation_status",
        "performance_smoke"
    )
    Write-EngineValidationReceipt $ReceiptPath "PASS" @() $Details
    Write-Host "FrameTrace Windows engine validation passed: $ReceiptPath"
} catch {
    $Blockers = @([ordered]@{
        name = "windows_engine_validation"
        status = "BLOCKED"
        evidence = $_.Exception.Message
    })
    Write-EngineValidationReceipt $ReceiptPath "BLOCKED" $Blockers $Details
    throw
}
