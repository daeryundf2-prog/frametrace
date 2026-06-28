param(
    [string]$CaseRoot = "$env:TEMP\frametrace-release-smoke",
    [int]$PerformanceRows = 1000,
    [string]$ReviewManifestPath = "",
    [switch]$EngineOnly
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

function Release-GateKeys {
    return @(
        "technical_review",
        "security_review",
        "privacy_review",
        "supply_chain_review",
        "accuracy_validation",
        "reproducibility_validation",
        "performance_validation",
        "migration_validation",
        "operator_review",
        "report_defensibility_review",
        "legal_wording_review",
        "installer_package_validation",
        "windows_workstation_validation",
        "known_limitations_review",
        "release_notes_review",
        "support_triage_policy",
        "hotfix_policy",
        "incident_response_plan",
        "corpus_governance",
        "feature_intake_governance",
        "post_ga_monitoring",
        "external_review_readiness",
        "regression_schedule"
    )
}

function Write-TypedReviewManifest {
    param(
        [string]$OutputDir,
        [string]$Timestamp,
        [string]$Status,
        [string]$Evidence
    )
    New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
    $ArtifactDir = Join-Path $OutputDir "release-review-artifacts"
    New-Item -ItemType Directory -Force -Path $ArtifactDir | Out-Null
    $Gates = foreach ($Key in (Release-GateKeys)) {
        $ArtifactPath = Join-Path $ArtifactDir "$Key.json"
        [ordered]@{
            schema_version = 1
            qa_type = "manual_release_review"
            gate = $Key
            status = $Status
            generated_at = $Timestamp
            evidence = $Evidence
        } | ConvertTo-Json -Depth 4 | Out-File -LiteralPath $ArtifactPath -Encoding utf8
        [ordered]@{
            key = $Key
            status = $Status
            artifact_path = "release-review-artifacts/$Key.json"
            tool = "scripts/windows/validate-release.ps1"
            evidence = $Evidence
            timestamp = $Timestamp
            reviewer = "release-validator"
            operator = "release-validator"
            cleanup_status = "clean"
        }
    }
    $ManifestPath = Join-Path $OutputDir "release-review.json"
    [ordered]@{
        schema_version = 1
        gates = @($Gates)
    } | ConvertTo-Json -Depth 6 | Out-File -LiteralPath $ManifestPath -Encoding utf8
    return $ManifestPath
}

function Import-ReviewManifest {
    param(
        [string]$OutputDir,
        [string]$InputPath
    )
    New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
    $ManifestPath = Join-Path $OutputDir "release-review.json"
    Copy-Item -LiteralPath $InputPath -Destination $ManifestPath -Force
    return $ManifestPath
}

function Merge-ReleaseDecisionBlockers {
    param([array]$Blockers)
    $ByName = [ordered]@{}
    foreach ($Blocker in $Blockers) {
        $Name = [string]$Blocker.name
        $Evidence = [string]$Blocker.evidence
        if (-not $ByName.Contains($Name)) {
            $ByName[$Name] = [ordered]@{
                name = $Name
                status = $Blocker.status
                evidence = $Evidence
                evidence_details = @($Evidence)
            }
        } else {
            $Existing = $ByName[$Name]
            $EvidenceDetails = @($Existing["evidence_details"]) + @($Evidence)
            $Existing["evidence"] = $EvidenceDetails -join "; "
            $Existing["evidence_details"] = @($EvidenceDetails)
        }
    }
    return @($ByName.Values)
}

function Write-ReleaseDecision {
    param(
        [string]$OutputDir,
        [string]$Decision,
        [array]$Blockers,
        [string]$Timestamp
    )
    New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
    $ReleaseBlockers = @(Merge-ReleaseDecisionBlockers $Blockers)
    [ordered]@{
        schema_version = 1
        qa_type = "release_decision"
        decision = $Decision
        generated_at = $Timestamp
        blocker_count = $ReleaseBlockers.Count
        blockers = @($ReleaseBlockers)
    } | ConvertTo-Json -Depth 6 | Out-File -LiteralPath (Join-Path $OutputDir "release-decision.json") -Encoding utf8
}

function Get-ReleasePreflightBlockers {
    param(
        [string]$RepoRoot,
        [bool]$RunningOnWindows
    )
    $Blockers = @()
    if (-not $RunningOnWindows) {
        $Blockers += [ordered]@{
            name = "windows_host"
            status = "BLOCKED"
            evidence = "unsupported-host: Windows release validation must run on Windows"
        }
    }
    foreach ($CommandName in @("rustc", "cargo", "ffmpeg", "ffprobe", "dotnet")) {
        if (-not (Get-Command $CommandName -ErrorAction SilentlyContinue)) {
            $Blockers += [ordered]@{
                name = "windows_prerequisites"
                status = "BLOCKED"
                evidence = "missing required command: $CommandName"
            }
        }
    }
    $WinUiRoot = Join-Path $RepoRoot "gui\winui"
    if (-not (Test-Path -LiteralPath $WinUiRoot -PathType Container)) {
        $Blockers += [ordered]@{
            name = "winui_build_test"
            status = "BLOCKED"
            evidence = "missing WinUI project directory: $WinUiRoot"
        }
    } else {
        $WinUiProject = Get-ChildItem -LiteralPath $WinUiRoot -Recurse -File |
            Where-Object { $_.Extension -in ".sln", ".csproj" } |
            Sort-Object FullName |
            Select-Object -First 1
        if ($null -eq $WinUiProject) {
            $Blockers += [ordered]@{
                name = "winui_build_test"
                status = "BLOCKED"
                evidence = "missing WinUI .sln or .csproj under $WinUiRoot"
            }
        }
        $WinUiTestProject = Get-ChildItem -LiteralPath $WinUiRoot -Recurse -File -Filter *.csproj |
            Where-Object { $_.Name -match '(?i)test|tests' } |
            Sort-Object FullName |
            Select-Object -First 1
        if ($null -eq $WinUiTestProject) {
            $Blockers += [ordered]@{
                name = "winui_build_test"
                status = "BLOCKED"
                evidence = "missing WinUI test .csproj under $WinUiRoot"
            }
        }
    }
    return @($Blockers)
}

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $RepoRoot

if ($EngineOnly) {
    & (Join-Path $PSScriptRoot "validate-engine.ps1") -CaseRoot $CaseRoot -PerformanceRows $PerformanceRows
    if ($LASTEXITCODE -ne 0) {
        throw "engine-only validation failed with exit code $LASTEXITCODE"
    }
    return
}

Remove-Item -LiteralPath $CaseRoot -Recurse -Force -ErrorAction SilentlyContinue
$CaseDir = Join-Path $CaseRoot "case"
$SourceDir = Join-Path $CaseRoot "source-media"
$QaPerfDir = Join-Path $CaseRoot "qa-performance"
$QaReportDir = Join-Path $CaseDir "reports\qa"
$ReleaseTimestamp = (Get-Date).ToUniversalTime().ToString("o")
$ReviewManifestBlockers = @()
if ([string]::IsNullOrWhiteSpace($ReviewManifestPath)) {
    $ReviewManifest = Write-TypedReviewManifest `
        $QaReportDir `
        $ReleaseTimestamp `
        "BLOCKED" `
        "missing external typed review manifest; pass -ReviewManifestPath"
    $ReviewManifestBlockers += [ordered]@{
        name = "typed_review_manifest"
        status = "BLOCKED"
        evidence = "missing -ReviewManifestPath"
    }
} elseif (-not (Test-Path -LiteralPath $ReviewManifestPath -PathType Leaf)) {
    $ReviewManifest = Write-TypedReviewManifest `
        $QaReportDir `
        $ReleaseTimestamp `
        "BLOCKED" `
        "external typed review manifest path was not found"
    $ReviewManifestBlockers += [ordered]@{
        name = "typed_review_manifest"
        status = "BLOCKED"
        evidence = "review manifest not found: $ReviewManifestPath"
    }
} else {
    $ReviewManifest = Import-ReviewManifest $QaReportDir $ReviewManifestPath
}

$IsWindowsVariable = Get-Variable IsWindows -Scope Global -ErrorAction SilentlyContinue
$RunningOnWindows = if ($null -ne $IsWindowsVariable) {
    [bool]$IsWindowsVariable.Value
} else {
    [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
        [System.Runtime.InteropServices.OSPlatform]::Windows
    )
}
$PreflightBlockers = @($ReviewManifestBlockers) + @(Get-ReleasePreflightBlockers $RepoRoot $RunningOnWindows)
if ($PreflightBlockers.Count -gt 0) {
    Write-ReleaseDecision $QaReportDir "BLOCKED" $PreflightBlockers $ReleaseTimestamp
    $BlockerSummary = ($PreflightBlockers | ForEach-Object { "$($_.name): $($_.evidence)" }) -join "; "
    throw "release validation preflight blocked: $BlockerSummary"
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
$CanonicalVideoPath = (Resolve-Path $SampleVideo).Path
"source_path`tsha256`n$CanonicalVideoPath`t" | Out-File -LiteralPath $CorpusManifest -Encoding utf8

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
$ReleaseDecisionPath = Join-Path $CaseDir "reports\qa\release-decision.json"
$ReleaseWorkstationStatusPath = Join-Path $CaseDir "reports\qa\workstation-status.json"
Assert-File $ReleaseReadinessPath
Assert-File $ReleaseDecisionPath
Assert-File $ReleaseWorkstationStatusPath
Assert-Contains $ReleaseReadinessPath '"passed": true'
Assert-Contains $ReleaseDecisionPath '"decision": "FIELD_PILOT_GO"'
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
