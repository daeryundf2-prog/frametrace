# Builds release binaries and a portable zip for examiners.
# Usage: powershell -File scripts/make-portable.ps1
$ErrorActionPreference = "Stop"
cargo build --release --locked
$version = (cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).packages[0].version
$stage = "dist/FrameTrace-$version-win64"
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path "$stage/tools" | Out-Null
Copy-Item target/release/frametrace.exe $stage/
Copy-Item target/release/frametrace-app.exe $stage/
Copy-Item README.md $stage/
Copy-Item docs/WINDOWS_USAGE.md $stage/docs/WINDOWS_USAGE.md -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path "$stage/docs" -Force | Out-Null
Copy-Item docs/WINDOWS_USAGE.md $stage/docs/
Copy-Item docs/WINDOWS_VALIDATION.md $stage/docs/ -ErrorAction SilentlyContinue
@'
FrameTrace portable package
===========================
1. (선택) tools/bin 아래에 ffmpeg/ffprobe, ewfinfo/ewfverify/ewfexport, mmls/fls/icat 실행 파일을 넣고
   그 폴더를 PATH에 추가하십시오. 설치 안내는 docs/WINDOWS_USAGE.md 참고.
2. frametrace-app.exe를 실행하면 브라우저 검수 워크스테이션이 열립니다 (콘솔 없음).
3. frametrace.exe는 CLI입니다 (인자 없이 실행해도 워크스테이션이 시작됩니다).
'@ | Out-File -Encoding utf8 "$stage/tools/README-tools.txt"
Compress-Archive -Path $stage -DestinationPath "$stage.zip" -Force
Write-Host "package: $stage.zip"
