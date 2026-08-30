# Builds libewf tools (ewfacquire/ewfinfo/ewfverify/ewfexport) from source
# inside a per-user MSYS2 environment (no admin required) and installs them
# into tools/bin next to the release binary.
#
# Usage: powershell -File scripts/build-libewf-tools.ps1
# Notes:
#   - libewf upstream ships source only (GitHub/SourceForge), so building is
#     the supported way to get Windows binaries.
#   - Takes ~10-20 minutes; downloads ~350 MB (MSYS2 + gcc).
#   - The produced tools are statically linked against libewf and only need
#     the system msvcrt.dll.
param(
    [string]$Msys2Dir = "$env:USERPROFILE\msys64",
    [string]$LibewfVersion = "20240506"
)
$ErrorActionPreference = "Stop"

# 1. MSYS2 bootstrap
if (-not (Test-Path "$Msys2Dir/usr/bin/bash.exe")) {
    Write-Host "downloading MSYS2 bootstrap..."
    New-Item -ItemType Directory -Force -Path (Split-Path $Msys2Dir) | Out-Null
    $sfx = Join-Path $env:TEMP "msys2.sfx.exe"
    Invoke-WebRequest -Uri "https://repo.msys2.org/distrib/msys2-x86_64-latest.sfx.exe" -OutFile $sfx
    Push-Location (Split-Path $Msys2Dir)
    & $sfx -y
    Pop-Location
    if (-not (Test-Path "$Msys2Dir/usr/bin/bash.exe")) { throw "MSYS2 extraction failed" }
}

$bash = "$Msys2Dir/usr/bin/bash.exe"

# 2. First-run init + keyring
& $bash -lc "pacman-key --init; pacman-key --populate msys2" 2>&1 | Select-Object -Last 1

# 3. mingw64 toolchain
& $bash -lc "pacman -S --noconfirm --needed mingw-w64-x86_64-gcc make" 2>&1 | Select-Object -Last 2

# 4. libewf source
$srcRoot = "$Msys2Dir/build"
New-Item -ItemType Directory -Force -Path $srcRoot | Out-Null
$tarball = "$srcRoot/libewf-experimental-$LibewfVersion.tar.gz"
if (-not (Test-Path $tarball)) {
    Invoke-WebRequest -Uri "https://github.com/libyal/libewf/releases/download/$LibewfVersion/libewf-experimental-$LibewfVersion.tar.gz" -OutFile $tarball
}
if (-not (Test-Path "$srcRoot/libewf-$LibewfVersion")) {
    tar -xzf $tarball -C $srcRoot
}

# 5. build + install
$env:MSYSTEM = "MINGW64"
$srcUnix = & $bash -lc ("cygpath -u " + "'" + "$srcRoot" + "'")
& $bash -lc "export MSYSTEM=MINGW64; cd '$srcUnix/libewf-$LibewfVersion'; ./configure --prefix='$srcUnix/out' --disable-nls > configure.log 2>&1; make -j`$(nproc) > make.log 2>&1 && make install > install.log 2>&1; echo BUILD_EXIT=`$?"
if ($LASTEXITCODE -ne 0) { throw "libewf build failed; see $srcRoot/libewf-$LibewfVersion/*.log" }

# 6. deploy to tools/bin
$dest = "target/release/tools/bin"
New-Item -ItemType Directory -Force -Path $dest | Out-Null
Copy-Item "$srcRoot/out/bin/ewfacquire.exe", "$srcRoot/out/bin/ewfinfo.exe", "$srcRoot/out/bin/ewfverify.exe", "$srcRoot/out/bin/ewfexport.exe", "$srcRoot/out/bin/libewf-3.dll" $dest -Force
# integration tests discover tools/bin next to the test binary too
New-Item -ItemType Directory -Force -Path "target/debug/deps/tools/bin" | Out-Null
Copy-Item "$dest/*" "target/debug/deps/tools/bin/" -Force
Write-Host "libewf tools installed: $dest"
Write-Host "verify: $dest/ewfinfo.exe -V"
