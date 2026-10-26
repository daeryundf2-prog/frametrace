# Builds release binaries and the portable Windows zip for examiners.
#
# The package contains FrameTrace only: frametrace.exe, frametrace-app.exe,
# README.md, LICENSE, THIRD_PARTY_NOTICES.md, docs, install.ps1 and an empty
# tools/bin with README-tools.txt. It bundles NO third-party tool binary
# (§Fix round 16 M2, option C): not FFmpeg (the build the guided installer
# fetches is not redistributable by us, docs/PLAN_TOOLS_C_2026-10-20.md T5),
# not libewf, not The Sleuth Kit. The examiner installs them after unzipping
# (`frametrace.exe tools install ffmpeg`, the home screen's install button,
# or by hand into tools/bin / PATH — docs/WINDOWS_USAGE.md). THIRD_PARTY_
# NOTICES.md states that the package holds no third-party executable; this
# script keeps that true:
#   - the run FAILS while the repository's tools/bin or the release
#     directory's tools/bin (the old bundling sources) holds any file — the
#     allowlist of FrameTrace-owned files there is empty — so nobody expects
#     a bundle that is no longer made;
#   - every staged file is refused when it is an FFmpeg file by name (any
#     name containing ffmpeg/ffprobe/ffplay or an FFmpeg library name, any
#     case, trailing dots/spaces ignored), by PE version information, or by
#     an FFmpeg signature string in its bytes (a renamed copy: real static
#     ffmpeg.exe builds have empty version info);
#   - every staged file is refused when it is an archive or a packed
#     executable (§Fix round 16 L-archives): by extension (.zip .7z .gz .tgz
#     .tar .cab .rar .xz .bz2 .zst) or by magic bytes (PK\x03\x04, PK\x05\x06,
#     1F 8B, 7z BC AF 27 1C, MSCF, Rar!, FD 7zXZ, "ustar" at 257, or "UPX!"
#     within the first 4 KiB of a PE) — compressed content would hide from
#     the signature scan;
#   - the written zip is checked again (entry names, FFmpeg names).
# A zip from an earlier run left in dist/ that contains FFmpeg also fails the
# run; SHA256SUMS lists only the zip written by this run.
#
# Signature scan (Fix round 6): case-insensitive `the FFmpeg developers`,
# `ffmpeg version <digit|n|N->`, `libavcodec`, `libavutil`, `libswscale`
# (Fix round 7; absent from our exes and staged docs), `Lavf<NN>.<N>` (the
# libavformat ident, e.g. Lavf61.7.100). They are chosen not to occur in our
# own staged files: "FFmpeg version text" (docs/WINDOWS_USAGE.md) and
# "lavfi" (frametrace.exe's trial encode) do not match.
#
# Binaries (§Fix round 16 M4): taken from where cargo really wrote them —
# the `executable` of the frametrace / frametrace-app compiler-artifact
# messages of `cargo build --release --message-format=json-render-diagnostics`;
# with -SkipBuild from `cargo metadata`'s target_directory + /release (so
# CARGO_TARGET_DIR, CARGO_BUILD_TARGET_DIR and .cargo/config.toml
# build.target-dir are honoured, never a stale target/release).
#
# Release guard: THIRD_PARTY_NOTICES.md still containing "Owner to confirm"
# (any whitespace between the words — a reflowed line still counts; a
# paragraph the owner has not confirmed yet) fails the run unless
# -AllowUnconfirmedNotices is passed (CI test packages only). The same
# applies (same switch) while any tool the app can install has no
# "notice_confirmed": true in tools/manifest.json — its distribution notice
# is compiled into frametrace.exe and the owner has not confirmed it (§Fix
# round 11 M2). Installable (§Fix round 16, src/tool_install/mod.rs): a
# "sha256" that is present, not null and not "" (archive tools), OR
# "kind": "pip" with a "requirements_file" that is present, not null and not
# "". Keys are matched exactly (case-sensitive, like serde): a
# "Notice_Confirmed": true does not count (§Fix round 16 L-keycase).
# The manifest checked is the one COMPILED INTO the packaged exes (§Fix round
# 16 hunch): both exes must contain the working tree's tools/manifest.json
# bytes (as-is, or with LF/CRLF line endings), else the run fails — an exe
# built before the notice was confirmed cannot be packaged as confirmed.
# With -AllowUnconfirmedNotices an unconfirmed package is named
# FrameTrace-<ver>-win64-UNCONFIRMED.zip, carries UNCONFIRMED.txt at its root
# (what is pending — survives renaming the zip; scripts/build-release.sh
# refuses any zip holding it), no SHA256SUMS is written
# (SHA256SUMS.unconfirmed instead; a stale SHA256SUMS is removed) and a
# warning says it is not a release.
#
# The zip is written with [IO.Compression.ZipFile]::CreateFromDirectory and
# every entry name is checked to use forward slashes. SHA-256 is computed
# through .NET (`Get-FileHash` fails to auto-load in Windows PowerShell 5.1
# started from pwsh 7 — GitHub runners).
#
# Usage:
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/make-portable.ps1 [-SkipBuild] [-Version <ver>] [-AllowUnconfirmedNotices]
#   -SkipBuild  use the binaries already in cargo's target directory (CI builds them first)
#   -Version    package version (default: Cargo.toml via cargo metadata)
#   -AllowUnconfirmedNotices  package although a notice is not confirmed
#               (a test package, never a release)
param(
    [switch]$SkipBuild,
    [string]$Version = "",
    [switch]$AllowUnconfirmedNotices
)
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
# Windows PowerShell 5.1 (.NET Framework) writes `\` into ZipFile entry names
# unless this switch is off; it is read once, so set it before any ZipFile
# call. The entry names are checked after the zip is written regardless.
[System.AppContext]::SetSwitch('Switch.System.IO.Compression.ZipFile.UseBackslash', $false)

function Get-Sha256Hex([string]$Path) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    $stream = [System.IO.File]::OpenRead((Resolve-Path -LiteralPath $Path).ProviderPath)
    try { ([System.BitConverter]::ToString($sha.ComputeHash($stream)) -replace '-', '').ToLowerInvariant() }
    finally { $stream.Dispose(); $sha.Dispose() }
}

function Test-SafeEntry([string]$name) {
    $trimmed = $name.TrimEnd('/')
    if ($trimmed.Length -eq 0 -or $trimmed.StartsWith('/')) { return $false }
    if ($name -match '[\\:\x00-\x1f]') { return $false }
    foreach ($part in $trimmed.Split('/')) {
        if ($part -eq '' -or $part -eq '.' -or $part -eq '..') { return $false }
    }
    return $true
}

# The leaf of a relative path, trimmed of spaces and trailing dots, lower case.
function Get-NormalLeaf([string]$rel) {
    return (($rel.TrimEnd('/', '\') -split '[/\\]')[-1]).Trim().TrimEnd('.', ' ').ToLowerInvariant()
}

# FFmpeg files that must never be packaged (relative, `/`-separated path):
# any leaf whose name — trimmed of spaces and trailing dots, any case —
# contains ffmpeg/ffprobe/ffplay or an FFmpeg library name (renamed copies,
# `ffmpeg.exe.bak`, `libavcodec-61.dll`, licenses/ffmpeg-*). The same rule is
# in scripts/build-release.sh (`ffmpeg_name`) and in the Windows CI smoke.
function Test-FfmpegFile([string]$rel) {
    $leaf = Get-NormalLeaf $rel
    if ($leaf -match 'ffmpeg|ffprobe|ffplay') { return $true }
    if ($leaf -match 'av(codec|format|util|device|filter)|swscale|swresample|postproc') { return $true }
    return $false
}

# Whether a PE file's version information names FFmpeg (a renamed binary).
function Test-FfmpegVersionInfo([string]$path) {
    $info = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($path)
    $text = @($info.ProductName, $info.FileDescription, $info.CompanyName, $info.InternalName, $info.OriginalFilename) -join ' '
    return ($text -match '(?i)ffmpeg|ffprobe|ffplay')
}

# FFmpeg signature strings (see the header): case-insensitive, ASCII.
$script:FfmpegSignature = New-Object System.Text.RegularExpressions.Regex(
    'the FFmpeg developers|ffmpeg version (?:n|N-)?[0-9]|libavcodec|libavutil|libswscale|Lavf[0-9]{2}\.[0-9]',
    ([System.Text.RegularExpressions.RegexOptions]::IgnoreCase -bor [System.Text.RegularExpressions.RegexOptions]::CultureInvariant))
# Longer than any signature: a match across a chunk boundary is not missed.
$script:SignatureOverlap = 64

# The first FFmpeg signature string in a file (any extension), or $null. Read
# in 4 MiB chunks; each chunk is decoded as Latin-1 (one char per byte) and
# keeps the last $SignatureOverlap bytes of the previous chunk in front.
function Get-FfmpegSignature([string]$path) {
    $latin1 = [System.Text.Encoding]::GetEncoding(28591)
    $chunk = 4MB
    $buffer = New-Object byte[] ($chunk + $script:SignatureOverlap)
    $stream = [System.IO.File]::OpenRead($path)
    try {
        $kept = 0
        while ($true) {
            $read = $stream.Read($buffer, $kept, $chunk)
            if ($read -le 0) { break }
            $total = $kept + $read
            $match = $script:FfmpegSignature.Match($latin1.GetString($buffer, 0, $total))
            if ($match.Success) { return $match.Value }
            $kept = [Math]::Min($script:SignatureOverlap, $total)
            [System.Array]::Copy($buffer, $total - $kept, $buffer, 0, $kept)
        }
    } finally {
        $stream.Dispose()
    }
    return $null
}

# Archive / packed-executable detection (§Fix round 16 L-archives): the
# reason a staged file is one, or $null.
$script:ArchiveExtension = '\.(zip|7z|gz|tgz|tar|cab|rar|xz|bz2|zst)$'
function Get-ArchiveReason([string]$path) {
    $leaf = Get-NormalLeaf $path
    if ($leaf -match $script:ArchiveExtension) { return "archive extension '$($Matches[0])'" }
    $head = New-Object byte[] 4096
    $stream = [System.IO.File]::OpenRead($path)
    try {
        $n = 0
        while ($n -lt $head.Length) {
            $read = $stream.Read($head, $n, $head.Length - $n)
            if ($read -le 0) { break }
            $n += $read
        }
    } finally {
        $stream.Dispose()
    }
    $starts = {
        param([byte[]]$sig, [int]$at)
        if ($n -lt $at + $sig.Length) { return $false }
        for ($i = 0; $i -lt $sig.Length; $i++) { if ($head[$at + $i] -ne $sig[$i]) { return $false } }
        return $true
    }
    if (& $starts ([byte[]](0x50, 0x4B, 0x03, 0x04)) 0) { return 'zip signature (PK\x03\x04)' }
    if (& $starts ([byte[]](0x50, 0x4B, 0x05, 0x06)) 0) { return 'zip signature (PK\x05\x06)' }
    if (& $starts ([byte[]](0x1F, 0x8B)) 0) { return 'gzip signature (1F 8B)' }
    if (& $starts ([byte[]](0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C)) 0) { return '7z signature' }
    if (& $starts ([byte[]](0x4D, 0x53, 0x43, 0x46)) 0) { return 'cabinet signature (MSCF)' }
    if (& $starts ([byte[]](0x52, 0x61, 0x72, 0x21, 0x1A, 0x07)) 0) { return 'rar signature' }
    if (& $starts ([byte[]](0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00)) 0) { return 'xz signature' }
    if (& $starts ([byte[]](0x75, 0x73, 0x74, 0x61, 0x72)) 257) { return 'tar signature (ustar)' }
    if (& $starts ([byte[]](0x4D, 0x5A)) 0) {
        $text = [System.Text.Encoding]::GetEncoding(28591).GetString($head, 0, $n)
        if ($text.Contains('UPX!')) { return 'UPX-packed executable (UPX! in the first 4 KiB)' }
    }
    return $null
}

# The first FFmpeg entry of a zip, or $null.
function Get-ZipFfmpegEntry([string]$path) {
    $zip = [System.IO.Compression.ZipFile]::OpenRead($path)
    try {
        foreach ($entry in $zip.Entries) {
            if (Test-FfmpegFile $entry.FullName) { return $entry.FullName }
        }
    } finally {
        $zip.Dispose()
    }
    return $null
}

# The property of a ConvertFrom-Json object whose name is exactly $name
# (case-sensitive, like serde — PSObject.Properties[...] ignores case), or $null.
function Get-ExactProperty($obj, [string]$name) {
    if ($null -eq $obj -or $obj -isnot [System.Management.Automation.PSCustomObject]) { return $null }
    foreach ($p in $obj.PSObject.Properties) {
        if ($p.Name -ceq $name) { return $p }
    }
    return $null
}

# Whether a property is present, not null and not "".
function Test-Given($prop) {
    return ($null -ne $prop -and $null -ne $prop.Value -and "$($prop.Value)" -ne '')
}

# Whether the app can install this tool (src/tool_install/mod.rs): an archive
# tool with a pinned sha256, or a pip tool with a requirements file.
function Test-InstallableTool($tool) {
    if (Test-Given (Get-ExactProperty $tool 'sha256')) { return $true }
    $kind = Get-ExactProperty $tool 'kind'
    if ($null -ne $kind -and $kind.Value -is [string] -and $kind.Value -ceq 'pip' -and (Test-Given (Get-ExactProperty $tool 'requirements_file'))) { return $true }
    return $false
}

# Every file under $dir (recursive, hidden included), or none.
function Get-FilesUnder([string]$dir) {
    if (-not (Test-Path -LiteralPath $dir -PathType Container)) { return @() }
    return @(Get-ChildItem -LiteralPath (Resolve-Path -LiteralPath $dir).ProviderPath -Recurse -File -Force)
}

# FrameTrace-owned files allowed in a tools/bin the package used to be
# bundled from. None: the package bundles no tool, so any file there means
# someone expects a bundle that is not made (§Fix round 16 M2).
$script:ToolsBinAllowlist = @()
function Assert-NoBundleSource([string]$dir) {
    $found = @(Get-FilesUnder $dir | Where-Object { $script:ToolsBinAllowlist -notcontains $_.Name })
    if ($found.Count -gt 0) {
        $shown = @($found | Select-Object -First 5 | ForEach-Object { $_.FullName }) -join ', '
        if ($found.Count -gt 5) { $shown += ", … ($($found.Count) files)" }
        throw "$dir contains files ($shown) — make-portable.ps1 bundles no third-party tool (libewf, Sleuth Kit, FFmpeg …; THIRD_PARTY_NOTICES.md: the package contains no third-party executable). Examiners install the tools themselves (frametrace.exe tools install / docs/WINDOWS_USAGE.md). Move these files out of $dir and run again."
    }
}

# The release directory cargo builds into (cargo metadata target_directory).
function Get-CargoMetadata {
    $text = @(cargo metadata --format-version 1 --no-deps) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw "cargo metadata failed (exit code $LASTEXITCODE)" }
    return ($text | ConvertFrom-Json)
}

# Whether $haystack (Latin-1 text of an exe) contains the manifest bytes as
# they are, or with LF / CRLF line endings.
function Test-EmbedsManifest([string]$haystack, [byte[]]$manifest) {
    $latin1 = [System.Text.Encoding]::GetEncoding(28591)
    $raw = $latin1.GetString($manifest)
    $lf = $raw.Replace("`r`n", "`n")
    $crlf = $lf.Replace("`n", "`r`n")
    foreach ($form in @($raw, $lf, $crlf)) {
        if ($form.Length -gt 0 -and $haystack.IndexOf($form, [System.StringComparison]::Ordinal) -ge 0) { return $true }
    }
    return $false
}

$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo

# M2: the repository's hand-assembled tools/bin used to be bundled.
Assert-NoBundleSource "tools/bin"

# Release guard (docs/PLAN_TOOLS_2026-10-20.md §Fix round 5 M2, §Fix round 6
# L-CI): any whitespace (a reflowed line break) between the words counts.
$unconfirmed = $false
$unconfirmedWhy = @()
if ((Get-Content -Raw -Encoding UTF8 -LiteralPath THIRD_PARTY_NOTICES.md) -match '(?i)Owner\s+to\s+confirm') {
    if (-not $AllowUnconfirmedNotices) {
        throw "THIRD_PARTY_NOTICES.md still contains 'Owner to confirm' — the owner has not confirmed that paragraph; no release package until then (pass -AllowUnconfirmedNotices for a test package)"
    }
    $unconfirmed = $true
    $unconfirmedWhy += "THIRD_PARTY_NOTICES.md still says 'Owner to confirm'."
    Write-Warning "THIRD_PARTY_NOTICES.md still contains 'Owner to confirm' — test package only (-AllowUnconfirmedNotices), not for release: the zip is named *-UNCONFIRMED.zip and no SHA256SUMS is written"
}

# Release guard (§Fix round 11 M2, §Fix round 16): the distribution notice of
# every tool the app can install is compiled into frametrace.exe and shown
# before the download; it needs "notice_confirmed": true from the owner.
if (-not (Test-Path -LiteralPath tools/manifest.json)) {
    throw "tools/manifest.json not found — cannot check the tools' distribution notices"
}
$manifestBytes = [System.IO.File]::ReadAllBytes((Resolve-Path -LiteralPath tools/manifest.json).ProviderPath)
$toolManifest = Get-Content -Raw -Encoding UTF8 -LiteralPath tools/manifest.json | ConvertFrom-Json
$toolsProp = Get-ExactProperty $toolManifest 'tools'
if ($null -eq $toolsProp -or $toolsProp.Value -isnot [System.Array]) {
    throw "tools/manifest.json has no exact `"tools`" array — cannot check the tools' distribution notices"
}
$noticePending = @()
foreach ($tool in @($toolsProp.Value)) {
    if ($tool -isnot [System.Management.Automation.PSCustomObject]) { throw "tools/manifest.json: a tool entry is not an object — cannot check the tools' distribution notices" }
    if (-not (Test-InstallableTool $tool)) { continue }
    $confirmed = Get-ExactProperty $tool 'notice_confirmed'
    if ($null -eq $confirmed -or $confirmed.Value -isnot [bool] -or -not $confirmed.Value) {
        $nameProp = Get-ExactProperty $tool 'name'
        $noticePending += $(if ($null -ne $nameProp) { "$($nameProp.Value)" } else { '(unnamed)' })
    }
}
if ($noticePending.Count -gt 0) {
    $names = $noticePending -join ', '
    if (-not $AllowUnconfirmedNotices) {
        throw "tools/manifest.json: the distribution notice of $names is not confirmed (notice_confirmed is not true) — the owner has not confirmed it; no release package until then (pass -AllowUnconfirmedNotices for a test package)"
    }
    $unconfirmed = $true
    $unconfirmedWhy += "tools/manifest.json notice_confirmed is not true for: $names."
    Write-Warning "tools/manifest.json: the distribution notice of $names is not confirmed — test package only (-AllowUnconfirmedNotices), not for release: the zip is named *-UNCONFIRMED.zip and no SHA256SUMS is written"
}

# A zip left in dist/ by an earlier packager (e.g. the old *-bundled.zip) must
# not sit next to this package with FFmpeg in it.
if (Test-Path dist) {
    foreach ($old in @(Get-ChildItem -LiteralPath (Resolve-Path dist).ProviderPath -Filter *.zip -File)) {
        $hit = $null
        try { $hit = Get-ZipFfmpegEntry $old.FullName } catch { throw "cannot read $($old.FullName): $($_.Exception.Message) — remove it from dist/" }
        if ($hit) { throw "dist/$($old.Name) contains FFmpeg ('$hit') — remove it; ffmpeg/ffprobe are never packaged (docs/PLAN_TOOLS_C_2026-10-20.md T5)" }
    }
}

# M4: the binaries cargo really built (never a stale target/release).
$metadata = $null
$exePaths = @{}
if (-not $SkipBuild) {
    $messages = @(cargo build --release --locked --message-format=json-render-diagnostics)
    if ($LASTEXITCODE -ne 0) { throw "cargo build --release failed (exit code $LASTEXITCODE)" }
    foreach ($line in $messages) {
        if ("$line" -notmatch '"reason"\s*:\s*"compiler-artifact"' -or "$line" -notmatch '"executable"\s*:\s*"') { continue }
        $msg = "$line" | ConvertFrom-Json
        $target = Get-ExactProperty $msg 'target'
        $exe = Get-ExactProperty $msg 'executable'
        if ($null -eq $target -or -not (Test-Given $exe)) { continue }
        $tname = Get-ExactProperty $target.Value 'name'
        $kinds = Get-ExactProperty $target.Value 'kind'
        if ($null -eq $tname -or $null -eq $kinds -or @($kinds.Value) -notcontains 'bin') { continue }
        if (@('frametrace', 'frametrace-app') -contains "$($tname.Value)") { $exePaths["$($tname.Value)"] = "$($exe.Value)" }
    }
    foreach ($bin in @('frametrace', 'frametrace-app')) {
        if (-not $exePaths.ContainsKey($bin)) { throw "cargo build reported no executable for the $bin binary (--message-format=json-render-diagnostics)" }
    }
} else {
    $metadata = Get-CargoMetadata
    $targetDir = Get-ExactProperty $metadata 'target_directory'
    if (-not (Test-Given $targetDir)) { throw "cargo metadata gave no target_directory" }
    foreach ($bin in @('frametrace', 'frametrace-app')) {
        $exePaths[$bin] = Join-Path (Join-Path "$($targetDir.Value)" 'release') "$bin.exe"
    }
}
foreach ($bin in @('frametrace', 'frametrace-app')) {
    if (-not (Test-Path -LiteralPath $exePaths[$bin] -PathType Leaf)) { throw "missing $($exePaths[$bin]) (build first or drop -SkipBuild)" }
}
# M2: the release directory's tools/bin (where scripts/build-libewf-tools.ps1
# and `tools install` put tools) was the other bundling source.
Assert-NoBundleSource (Join-Path (Split-Path -Parent $exePaths['frametrace']) 'tools/bin')

# Hunch (§Fix round 16): the manifest judged above must be the one compiled
# into the exes being packaged (include_str! — raw bytes in the binary).
$latin1 = [System.Text.Encoding]::GetEncoding(28591)
foreach ($bin in @('frametrace', 'frametrace-app')) {
    $exeText = $latin1.GetString([System.IO.File]::ReadAllBytes($exePaths[$bin]))
    $embeds = Test-EmbedsManifest $exeText $manifestBytes
    $exeText = $null
    if (-not $embeds) {
        throw "$($exePaths[$bin]) does not contain this working tree's tools/manifest.json — it was built from another manifest (e.g. before a notice was confirmed); rebuild it (drop -SkipBuild) so the checked manifest is the one compiled in"
    }
}

if (-not $Version) {
    if ($null -eq $metadata) { $metadata = Get-CargoMetadata }
    $Version = $metadata.packages[0].version
}
if ($Version -notmatch '^[0-9A-Za-z][0-9A-Za-z.+-]*$') { throw "bad -Version '$Version'" }

$stage = "dist/FrameTrace-$Version-win64"
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path "$stage/tools/bin" | Out-Null
Copy-Item -LiteralPath $exePaths['frametrace'] -Destination "$stage/frametrace.exe"
Copy-Item -LiteralPath $exePaths['frametrace-app'] -Destination "$stage/frametrace-app.exe"
Copy-Item README.md, LICENSE, THIRD_PARTY_NOTICES.md $stage/
New-Item -ItemType Directory -Path "$stage/docs" -Force | Out-Null
Copy-Item docs/WINDOWS_USAGE.md $stage/docs/
Copy-Item docs/WINDOWS_VALIDATION.md $stage/docs/ -ErrorAction SilentlyContinue
Copy-Item scripts/install.ps1 $stage/
@'
FrameTrace portable package
===========================
1. 이 패키지에는 외부 도구 실행 파일이 들어 있지 않습니다 (FFmpeg·libewf·Sleuth Kit 모두).
   이 tools/bin 폴더에 넣은 도구 실행 파일은 PATH 설정 없이 자동 인식됩니다.
   - ffmpeg.exe / ffprobe.exe — 패키지에 포함되지 않음. frametrace.exe tools install ffmpeg (또는 첫 화면의 [설치])로 BtbN에서 직접 내려받거나 PATH에 두십시오.
   - ewfinfo.exe / ewfverify.exe / ewfexport.exe (E01) — 패키지에 포함되지 않음. 직접 설치해 이 폴더나 PATH에 두십시오.
   - mmls.exe / fls.exe / icat.exe (Sleuth Kit) — 패키지에 포함되지 않음. 직접 설치해 이 폴더나 PATH에 두십시오.
   설치/빌드 안내는 docs/WINDOWS_USAGE.md 참고.
2. frametrace-app.exe를 실행하면 브라우저 검수 워크스테이션이 열립니다 (콘솔 없음).
   종료는 페이지 우측 상단 '종료' 버튼을 사용하십시오. 브라우저 탭만 닫으면
   서버가 계속 실행됩니다. 다시 실행하면 실행 중인 서버에 재접속됩니다.
3. frametrace.exe는 CLI입니다 (인자 없이 실행해도 워크스테이션이 시작됩니다).
4. 설치(시작 메뉴 등록): powershell -ExecutionPolicy Bypass -File install.ps1
   제거: powershell -ExecutionPolicy Bypass -File install.ps1 -Uninstall
   그냥 압축 풀고 frametrace-app.exe를 바로 실행해도 됩니다.
5. 실 Dahua DAV 샘플 검증: scripts/validate-dav-samples.ps1 -Samples <폴더>
'@ | Out-File -Encoding utf8 "$stage/tools/bin/README-tools.txt"
# L-marker (§Fix round 16): an unconfirmed package says so inside, not only
# in its file name (renaming the zip must not make it look like a release).
if ($unconfirmed) {
    $lines = @(
        'UNCONFIRMED TEST PACKAGE - NOT FOR RELEASE',
        '',
        'This package was built with make-portable.ps1 -AllowUnconfirmedNotices.',
        'The owner has not confirmed these notices yet:'
    ) + @($unconfirmedWhy | ForEach-Object { "  - $_" }) + @(
        '',
        'Do not publish or redistribute this zip. scripts/build-release.sh refuses',
        'any zip that contains this file.'
    )
    [System.IO.File]::WriteAllText((Join-Path (Resolve-Path -LiteralPath $stage).ProviderPath 'UNCONFIRMED.txt'),
        (($lines -join "`r`n") + "`r`n"), (New-Object System.Text.UTF8Encoding($false)))
}

$fullStage = (Resolve-Path -LiteralPath $stage).ProviderPath
$staged = @(Get-ChildItem -LiteralPath $fullStage -Recurse -File -Force)

# ---- no FFmpeg file by name (nothing above copies one; a safety net) --------
foreach ($file in $staged) {
    $rel = $file.FullName.Substring($fullStage.Length).TrimStart('\', '/') -replace '\\', '/'
    if (Test-FfmpegFile $rel) {
        throw "$($file.FullName) is an FFmpeg file by its name — ffmpeg/ffprobe are never packaged (docs/PLAN_TOOLS_C_2026-10-20.md T5)"
    }
}

# ---- no archive / packed executable (L-archives) -----------------------------
foreach ($file in $staged) {
    $why = Get-ArchiveReason $file.FullName
    if ($why) {
        throw "$($file.FullName) is an archive or packed executable ($why) — compressed content would bypass the FFmpeg checks; the package holds no archives"
    }
}

# ---- no renamed FFmpeg binary (PE version information) ----------------------
foreach ($pe in @($staged | Where-Object { $_.Extension -match '^\.(exe|dll)$' })) {
    if (Test-FfmpegVersionInfo $pe.FullName) {
        throw "$($pe.FullName) is an FFmpeg binary by its version information — ffmpeg/ffprobe are never packaged (docs/PLAN_TOOLS_C_2026-10-20.md T5)"
    }
}

# ---- no renamed FFmpeg file (signature strings, every staged file) ----------
foreach ($file in $staged) {
    $sig = Get-FfmpegSignature $file.FullName
    if ($sig) {
        throw "$($file.FullName) contains the FFmpeg signature '$sig' — a renamed FFmpeg file? ffmpeg/ffprobe are never packaged (docs/PLAN_TOOLS_C_2026-10-20.md T5)"
    }
}

# ---- zip (forward-slash entry names, no FFmpeg — both checked) ----------------
$dist = (Resolve-Path "dist").ProviderPath
$zipName = if ($unconfirmed) { "FrameTrace-$Version-win64-UNCONFIRMED.zip" } else { "FrameTrace-$Version-win64.zip" }
$fullZip = Join-Path $dist $zipName
# Never leave the other kind of zip of this version next to this one.
foreach ($other in @("FrameTrace-$Version-win64.zip", "FrameTrace-$Version-win64-UNCONFIRMED.zip")) {
    Remove-Item -Force -LiteralPath (Join-Path $dist $other) -ErrorAction SilentlyContinue
}
Remove-Item -Force -LiteralPath $fullZip -ErrorAction SilentlyContinue
[System.IO.Compression.ZipFile]::CreateFromDirectory($fullStage, $fullZip, [System.IO.Compression.CompressionLevel]::Optimal, $true)
$problem = $null
$check = [System.IO.Compression.ZipFile]::OpenRead($fullZip)
try {
    foreach ($entry in $check.Entries) {
        $name = $entry.FullName
        if ($name.Contains('\')) { $problem = "package zip entry uses a backslash: '$name' (other unzip tools would not see folders)"; break }
        if (-not (Test-SafeEntry $name)) { $problem = "package zip has an unsafe entry '$name'"; break }
        if (Test-FfmpegFile $name) { $problem = "package zip contains FFmpeg ('$name') — ffmpeg/ffprobe are never packaged (docs/PLAN_TOOLS_C_2026-10-20.md T5)"; break }
        if ((Get-NormalLeaf $name) -match $script:ArchiveExtension) { $problem = "package zip contains an archive ('$name')"; break }
    }
} finally {
    $check.Dispose()
}
if ($problem) {
    Remove-Item -Force -LiteralPath $fullZip
    throw $problem
}

# SHA256SUMS for the zip written by this run only (a stale zip in dist/ is
# never vouched for) — same manifest convention as scripts/build-release.sh;
# consumers verify with `sha256sum -c` (see docs/repro-build.md). LF endings,
# no BOM (`sha256sum -c` on Linux/macOS).
# An unconfirmed (test) package gets SHA256SUMS.unconfirmed instead, and a
# SHA256SUMS left by an earlier run is removed: nothing vouches for it as a
# release.
$sumLine = "{0}  {1}" -f (Get-Sha256Hex $fullZip), (Split-Path -Leaf $fullZip)
$sumsName = if ($unconfirmed) { "SHA256SUMS.unconfirmed" } else { "SHA256SUMS" }
foreach ($old in @("SHA256SUMS", "SHA256SUMS.unconfirmed")) {
    Remove-Item -Force -LiteralPath (Join-Path $dist $old) -ErrorAction SilentlyContinue
}
[System.IO.File]::WriteAllText((Join-Path $dist $sumsName), ($sumLine + "`n"), (New-Object System.Text.UTF8Encoding($false)))
Write-Host "package: $fullZip"
Write-Host "sums: $(Join-Path $dist $sumsName)"
if ($unconfirmed) {
    Write-Warning "=================================================================="
    Write-Warning " UNCONFIRMED TEST PACKAGE — NOT FOR RELEASE"
    foreach ($why in $unconfirmedWhy) { Write-Warning " $why" }
    Write-Warning " $zipName / $sumsName (no SHA256SUMS written; UNCONFIRMED.txt inside)"
    Write-Warning "=================================================================="
}
