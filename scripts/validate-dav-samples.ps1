# Real Dahua DAV sample intake harness.
# Walks every .dav under -Samples, remuxes through frametrace export-dav,
# then ffprobe-validates the MP4. Does not commit evidence into git.
#
# Usage:
#   powershell -File scripts/validate-dav-samples.ps1 -Samples C:\Evidence\dav-corpus
#   powershell -File scripts/validate-dav-samples.ps1 -Samples C:\Evidence\dav-corpus -Exe .\target\release\frametrace.exe
#
# Expected intake (ROADMAP M2-1): at least 3 recorder exports
# (e.g. continuous / event / parking) before claiming field validation.

param(
	[Parameter(Mandatory = $true)]
	[string]$Samples,
	[string]$Exe = "",
	[string]$WorkRoot = "$env:TEMP\frametrace-dav-validation",
	[int]$TimeoutSecs = 120
)

$ErrorActionPreference = "Stop"

function Resolve-FrametraceExe {
	param([string]$Preferred)
	if ($Preferred -and (Test-Path -LiteralPath $Preferred)) {
		return (Resolve-Path -LiteralPath $Preferred).Path
	}
	$candidates = @(
		".\target\release\frametrace.exe",
		".\target\debug\frametrace.exe"
	)
	foreach ($candidate in $candidates) {
		if (Test-Path -LiteralPath $candidate) {
			return (Resolve-Path -LiteralPath $candidate).Path
		}
	}
	throw "frametrace.exe not found. Build with 'cargo build --release' or pass -Exe."
}

if (-not (Test-Path -LiteralPath $Samples)) {
	throw "Samples folder not found: $Samples"
}

$davFiles = @(Get-ChildItem -LiteralPath $Samples -Recurse -File -Include *.dav, *.DAV |
	Sort-Object FullName)
if ($davFiles.Count -eq 0) {
	Write-Host "BLOCKED: no .dav files under $Samples"
	Write-Host "Place at least 3 recorder exports (continuous / event / parking) and re-run."
	exit 2
}

$exePath = Resolve-FrametraceExe -Preferred $Exe
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$caseDir = Join-Path $WorkRoot "case-$stamp"
$reportPath = Join-Path $WorkRoot "dav-validation-$stamp.jsonl"

New-Item -ItemType Directory -Force -Path $caseDir | Out-Null
& $exePath init-case $caseDir --title "DAV sample validation $stamp" | Out-Host

$passed = 0
$failed = 0
$results = @()

foreach ($dav in $davFiles) {
	$relative = $dav.FullName.Substring((Resolve-Path -LiteralPath $Samples).Path.Length).TrimStart('\', '/')
	Write-Host "----"
	Write-Host "sample: $($dav.FullName)"
	$entry = [ordered]@{
		sample      = $dav.FullName
		relative    = $relative
		size_bytes  = $dav.Length
		status      = "fail"
		output_mp4  = $null
		error       = $null
		export_log  = $null
	}
	try {
		& $exePath export-dav $caseDir $dav.FullName --timeout $TimeoutSecs | Out-Host
		if ($LASTEXITCODE -ne 0) {
			throw "export-dav exit $LASTEXITCODE"
		}
		$stem = [System.IO.Path]::GetFileNameWithoutExtension($dav.Name)
		$mp4 = Get-ChildItem -LiteralPath (Join-Path $caseDir "artifacts\clips") -Filter "$stem*.mp4" |
			Sort-Object LastWriteTime -Descending |
			Select-Object -First 1
		if (-not $mp4) {
			throw "remuxed mp4 not found for stem $stem"
		}
		& $exePath validate-artifact $caseDir $mp4.FullName | Out-Host
		if ($LASTEXITCODE -ne 0) {
			throw "validate-artifact exit $LASTEXITCODE"
		}
		$validationLog = Get-Content -LiteralPath (Join-Path $caseDir "evidence\logs\validation-log.jsonl") -Raw
		if ($validationLog -notmatch "ffprobe-video-stream-confirmed") {
			throw "validation log missing ffprobe-video-stream-confirmed"
		}
		$entry.status = "pass"
		$entry.output_mp4 = $mp4.FullName
		$passed++
	}
	catch {
		$entry.error = "$_"
		$failed++
		Write-Host "FAIL: $_"
	}
	($entry | ConvertTo-Json -Compress) | Add-Content -LiteralPath $reportPath -Encoding utf8
	$results += $entry
}

Write-Host "===="
Write-Host "samples: $($davFiles.Count)  pass: $passed  fail: $failed"
Write-Host "case: $caseDir"
Write-Host "report: $reportPath"

if ($davFiles.Count -lt 3) {
	Write-Host "NOTE: ROADMAP M2-1 asks for >=3 recorder exports before field validation is claimed."
}

if ($failed -gt 0) {
	exit 1
}
if ($davFiles.Count -lt 3) {
	exit 3
}
exit 0
