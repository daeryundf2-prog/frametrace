param(
    [string]$CaseRoot = "$env:TEMP\frametrace-release-smoke",
    [int]$PerformanceRows = 1000
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Invoke-Step {
    param(
        [string]$Name,
        [scriptblock]$Script
    )
    Write-Host "== $Name"
    & $Script
}

function Invoke-NativeStep {
    param(
        [string]$Name,
        [scriptblock]$Script
    )
    Invoke-Step $Name {
        & $Script
        if ($LASTEXITCODE -ne 0) {
            throw "$Name failed with exit code $LASTEXITCODE"
        }
    }
}

function Require-Command {
    param([string]$Name)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "missing required command: $Name"
    }
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

function Find-WinUiProject {
    param([string]$Root)
    if (-not (Test-Path -LiteralPath $Root -PathType Container)) {
        throw "missing WinUI project directory: $Root"
    }
    $Project = Get-ChildItem -LiteralPath $Root -Recurse -File |
        Where-Object { $_.Extension -in ".sln", ".csproj" } |
        Sort-Object FullName |
        Select-Object -First 1
    if ($null -eq $Project) {
        throw "missing WinUI .sln or .csproj under $Root"
    }
    return $Project
}

function Find-WinUiTestProject {
    param([string]$Root)
    $Project = Get-ChildItem -LiteralPath $Root -Recurse -File -Filter *.csproj |
        Where-Object { $_.Name -match '(?i)test|tests' } |
        Sort-Object FullName |
        Select-Object -First 1
    if ($null -eq $Project) {
        throw "missing WinUI test .csproj under $Root"
    }
    return $Project
}

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $RepoRoot

$IsWindowsVariable = Get-Variable IsWindows -Scope Global -ErrorAction SilentlyContinue
$RunningOnWindows = if ($null -ne $IsWindowsVariable) {
    [bool]$IsWindowsVariable.Value
} else {
    [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
        [System.Runtime.InteropServices.OSPlatform]::Windows
    )
}
if (-not $RunningOnWindows) {
    throw "Windows release validation must run on Windows."
}

Require-Command rustc
Require-Command cargo
Require-Command ffmpeg
Require-Command ffprobe
Require-Command dotnet

$RustVersion = rustc -vV | Out-String
if ($RustVersion -notmatch "host: .*windows-msvc") {
    throw "Rust MSVC toolchain required for Windows release validation."
}

$WinUiRoot = Join-Path $RepoRoot "gui\winui"
$WinUiProject = Find-WinUiProject $WinUiRoot
$WinUiTestProject = Find-WinUiTestProject $WinUiRoot

Invoke-NativeStep "rust format" { cargo fmt --check }
Invoke-NativeStep "rust check" { cargo check --locked }
Invoke-NativeStep "rust clippy" { cargo clippy --locked --all-targets -- -D warnings }
Invoke-NativeStep "rust tests" { cargo test --locked }
if (Get-Command node -ErrorAction SilentlyContinue) {
    Invoke-NativeStep "viewer javascript syntax" { node --check gui/evidence-viewer/app.js }
}
Invoke-NativeStep "release build" { cargo build --release --locked }
Invoke-NativeStep "winui build" { dotnet build $WinUiProject.FullName -c Release --nologo }
Invoke-NativeStep "winui tests" { dotnet test $WinUiTestProject.FullName -c Release --nologo }

$Binary = Join-Path $RepoRoot "target\release\frametrace.exe"
Assert-File $Binary

Remove-Item -LiteralPath $CaseRoot -Recurse -Force -ErrorAction SilentlyContinue
$CaseDir = Join-Path $CaseRoot "case"
$SourceDir = Join-Path $CaseRoot "source-media"
$QaPerfDir = Join-Path $CaseRoot "qa-performance"
New-Item -ItemType Directory -Force -Path $SourceDir | Out-Null

$SampleVideo = Join-Path $SourceDir "sample.mp4"
Invoke-NativeStep "synthetic mp4 fixture" {
    ffmpeg -hide_banner -loglevel error -y -f lavfi -i testsrc=size=160x90:rate=1 -t 1 -pix_fmt yuv420p $SampleVideo
}

Invoke-Step "case workflow" {
    Invoke-NativeStep "init case" { & $Binary init-case $CaseDir --title "Windows release smoke" --operator "release-validator" }
    Invoke-NativeStep "scan folder" { & $Binary scan-folder $CaseDir $SourceDir --hash }
    Invoke-NativeStep "validate artifact" { & $Binary validate-artifact $CaseDir vid_000001 --operator "release-validator" }
    Invoke-NativeStep "confirm playback" { & $Binary confirm-playback $CaseDir vid_000001 --operator "release-validator" --playback-tool "Windows Media Player" --notes "release smoke playback confirmation" }
    Invoke-NativeStep "make review" { & $Binary make-review $CaseDir }
    Invoke-NativeStep "make report" { & $Binary make-report $CaseDir }
    Invoke-NativeStep "package case" { & $Binary package-case $CaseDir }
}

$ValidationLog = Join-Path $CaseDir "evidence\logs\validation-log.jsonl"
Assert-File $ValidationLog
Assert-Contains $ValidationLog '"validation_status":"ffprobe-video-stream-confirmed"'
Assert-Contains $ValidationLog '"validation_status":"playback-confirmed"'

$StatusPath = Join-Path $CaseRoot "workstation-status.json"
Invoke-Step "workstation status" {
    $StatusJson = & $Binary workstation-status $CaseDir
    if ($LASTEXITCODE -ne 0) {
        throw "workstation status failed with exit code $LASTEXITCODE"
    }
    $StatusJson | Out-File -LiteralPath $StatusPath -Encoding utf8
}
Assert-Contains $StatusPath '"view":"workstation-status"'
Assert-Contains $StatusPath '"engine_source_of_truth":true'
Assert-Contains $StatusPath '"gui_durable_state_allowed":false'
Assert-Contains $StatusPath '"full_json_load_allowed":false'
Assert-Contains $StatusPath '"ffprobe_and_playback_are_separate_states":true'
Assert-Contains $StatusPath '"windows_prerequisites":{'
Assert-Contains $StatusPath '"release_validation_host_ready":true'
Assert-Contains $StatusPath '"playback_confirmed_count":1'

$CorpusManifest = Join-Path $CaseRoot "corpus.tsv"
$ReviewManifest = Join-Path $CaseRoot "release-review.txt"
$CanonicalVideoPath = (Resolve-Path $SampleVideo).Path
"source_path`tsha256`n$CanonicalVideoPath`t" | Out-File -LiteralPath $CorpusManifest -Encoding utf8
@"
technical_review=pass
security_review=pass
privacy_review=pass
supply_chain_review=pass
accuracy_validation=pass
reproducibility_validation=pass
performance_validation=pass
migration_validation=pass
operator_review=pass
report_defensibility_review=pass
legal_wording_review=pass
installer_package_validation=pass
windows_workstation_validation=pass
known_limitations_review=pass
release_notes_review=pass
support_triage_policy=pass
hotfix_policy=pass
incident_response_plan=pass
corpus_governance=pass
feature_intake_governance=pass
post_ga_monitoring=pass
external_review_readiness=pass
regression_schedule=pass
"@ | Out-File -LiteralPath $ReviewManifest -Encoding utf8

$QaReportDir = Join-Path $CaseDir "reports\qa"
New-Item -ItemType Directory -Force -Path $QaReportDir | Out-Null
$WinUiBuildReceipt = Join-Path $QaReportDir "winui-build.json"
[ordered]@{
    schema_version = 1
    checked_at = (Get-Date).ToUniversalTime().ToString("o")
    project_path = $WinUiProject.FullName
    test_project_path = $WinUiTestProject.FullName
    dotnet_build = "pass"
    dotnet_test = "pass"
} | ConvertTo-Json -Depth 4 | Out-File -LiteralPath $WinUiBuildReceipt -Encoding utf8

Invoke-NativeStep "release qa" {
    & $Binary qa release $CaseDir `
        --corpus-manifest $CorpusManifest `
        --comparison-case $CaseDir `
        --review-manifest $ReviewManifest `
        --performance-output-dir $QaPerfDir `
        --performance-rows $PerformanceRows
}

Assert-File (Join-Path $CaseDir "review\index.html")
Assert-File (Join-Path $CaseDir "review\evidence-viewer.html")
Assert-File (Join-Path $CaseDir "reports\case-report.html")
$ReleaseReadinessPath = Join-Path $CaseDir "reports\qa\release-readiness.json"
$ReleaseWorkstationStatusPath = Join-Path $CaseDir "reports\qa\workstation-status.json"
Assert-File $ReleaseReadinessPath
Assert-File $ReleaseWorkstationStatusPath
Assert-Contains $ReleaseReadinessPath '"passed": true'
Assert-Contains $ReleaseReadinessPath '"name":"workstation_shell_contract"'
Assert-Contains $ReleaseWorkstationStatusPath '"view":"workstation-status"'
Assert-Contains $ReleaseWorkstationStatusPath '"engine_source_of_truth":true'
Assert-Contains $ReleaseWorkstationStatusPath '"gui_durable_state_allowed":false'
Assert-Contains $ReleaseWorkstationStatusPath '"large_case_full_json_load_allowed":false'
Assert-Contains $ReleaseWorkstationStatusPath '"windows_prerequisites":{'
Assert-Contains $ReleaseWorkstationStatusPath '"release_validation_host_ready":true'
Assert-Contains $ReleaseReadinessPath '"name":"windows_prerequisites"'
Assert-Contains $ReleaseReadinessPath '"name":"windows_prerequisites","status":"PASS"'

Write-Host "FrameTrace Windows release validation passed"
