#!/usr/bin/env bash
# Fixture harness for scripts/build-release.sh (no real build, no network):
# copies the script into a throwaway repo, puts stub `cargo`, `rustc` and
# `zip` first on PATH, and checks that
#   - any *.zip in dist/ the run did not build (a Windows UNCONFIRMED test
#     package by any name, an old *-bundled.zip, an outer zip, another
#     target's zip) stops the run before the build and again before
#     SHA256SUMS, and SHA256SUMS lists exactly the zip the run built (§Fix
#     round 16 M3);
#   - the zip the run built is refused (and deleted) when it holds an FFmpeg
#     entry, an UNCONFIRMED.txt, a nested archive or THIRD_PARTY_NOTICES.md
#     with the "Owner to confirm" marker — read with each zip reader, failing
#     closed (M3, L-marker);
#   - an unconfirmed working tree (tools/manifest.json notice_confirmed not
#     the boolean true under its exact key for a tool the app can install —
#     sha256 pinned, or a pip tool with a requirements file — or
#     THIRD_PARTY_NOTICES.md missing, unreadable or still saying "Owner to
#     confirm") stops it before cargo runs;
#   - the binaries are the ones cargo reports in its JSON messages
#     (CARGO_TARGET_DIR, CARGO_BUILD_TARGET_DIR, .cargo/config.toml) (M4).
#
# Runs from Git Bash on Windows and from bash on Linux/macOS:
#   bash scripts/test-build-release.sh
# Needs: shasum (the script under test), python3 or python (to make the
# fixture zips, as the stub `zip` and as the script's first zip reader;
# without it the zip-content cases are skipped and the rest run with
# FRAMETRACE_SKIP_NOTICES_CHECK=1 — the manifest check then needs node, the
# script's second manifest reader; node also runs its own cases),
# unzip (optional: the unzip-reader cases are skipped without it), bsdtar
# (optional: `tar` when it is bsdtar, or FRAMETRACE_TEST_BSDTAR=<path>, e.g.
# C:/Windows/System32/tar.exe; the tar-reader cases are skipped without it).
# Skipped cases are counted and shown as "SKIPPED n" in the summary.
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="$(mktemp -d 2>/dev/null || mktemp -d -t ft-build-release)"
trap 'chmod -R u+rw "$WORK" 2>/dev/null; rm -rf "$WORK"' EXIT

pass=0
fail=0
skip=0
ok()   { pass=$((pass + 1)); echo "ok   $*"; }
bad()  { fail=$((fail + 1)); echo "FAIL $*"; }
skp()  { skip=$((skip + 1)); echo "skip $*"; }
check() { # check <description> <command...>
    local d="$1"; shift
    if "$@"; then ok "$d"; else bad "$d"; fi
}

# Same probe as the script's find_python: the first of python3/python that
# can `import zipfile` (a Microsoft Store alias python3 is skipped).
PY=""
for c in python3 python; do
    command -v "$c" >/dev/null 2>&1 || continue
    if "$c" -I -c 'import zipfile' >/dev/null 2>&1; then PY="$(command -v "$c")"; break; fi
done
HAVE_UNZIP=0
command -v unzip >/dev/null 2>&1 && HAVE_UNZIP=1
# Same probe as the script's find_node.
NODE=""
if command -v node >/dev/null 2>&1 && node -e 0 >/dev/null 2>&1; then NODE=node; fi
REAL_CAT="$(command -v cat)"

# --- stubs ------------------------------------------------------------------
STUBS="$WORK/stubs"
mkdir -p "$STUBS"
cat > "$STUBS/cargo" <<'EOF'
#!/usr/bin/env bash
echo "cargo $*" >> "$FT_LOG"
case " $* " in
    *" --message-format=json-render-diagnostics "*) ;;
    *) echo "stub cargo: expected --message-format=json-render-diagnostics" >&2; exit 2 ;;
esac
# Like cargo (Fix round 16 M4): CARGO_TARGET_DIR, else CARGO_BUILD_TARGET_DIR,
# else .cargo/config.toml build.target-dir, else target (relative = to the
# cwd, the repo root). A configured directory gets a fresh `frametrace`
# (FT_CARGO_NOWRITE=1: nothing); the default target/ is left as it is.
dir="${CARGO_TARGET_DIR:-${CARGO_BUILD_TARGET_DIR:-}}"
if [ -z "$dir" ] && [ -f .cargo/config.toml ]; then
    dir="$(sed -n 's/^[[:space:]]*target-dir[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' .cargo/config.toml | head -1)"
fi
custom=1
if [ -z "$dir" ]; then dir=target; custom=0; fi
mkdir -p "$dir/release"
abs="$(cd "$dir/release" && pwd)"
if [ "$custom" -eq 1 ] && [ "${FT_CARGO_NOWRITE:-}" != 1 ]; then
    echo fresh-from-cargo > "$abs/frametrace"
fi
case "${PLANT:-}" in
    name) : > "dist/FrameTrace-9.9.9-win64-UNCONFIRMED (1).zip" ;;
    marker) cp "$FT_MARKER_ZIP" "dist/FrameTrace-9.9.9-win64.zip" ;;
esac
echo '{"reason":"compiler-artifact","target":{"name":"frametrace","kind":["lib"]},"executable":null}'
case ",${FT_CARGO_OMIT:-}," in
    *,frametrace,*) ;;
    *) printf '{"reason":"compiler-artifact","target":{"name":"frametrace","kind":["bin"]},"executable":"%s"}\n' "$abs/frametrace" ;;
esac
if [ -f "$abs/frametrace-app" ]; then
    printf '{"reason":"compiler-artifact","target":{"name":"frametrace-app","kind":["bin"]},"executable":"%s"}\n' "$abs/frametrace-app"
fi
echo '{"reason":"build-finished","success":true}'
exit 0
EOF
cat > "$STUBS/rustc" <<'EOF'
#!/usr/bin/env bash
echo "rustc 1.0.0 (stub)"
echo "host: x86_64-unknown-linux-gnu"
EOF
cat > "$STUBS/zip" <<'EOF'
#!/usr/bin/env bash
# zip -qr <out.zip> <dir>: a real (readable) zip of every file under <dir>
# (an empty zip without python). FT_STAGE_ADD=<path> first adds a file to
# the staged <dir>; FT_ZIP_ADD='[[name, text], ...]' (JSON) adds entries to
# the zip only; FT_ZIP_BROKEN=1 writes a file that is not a zip.
if [ -n "${FT_STAGE_ADD:-}" ]; then
    mkdir -p "$(dirname "$3/$FT_STAGE_ADD")"
    echo staged > "$3/$FT_STAGE_ADD"
fi
if [ "${FT_ZIP_BROKEN:-}" = 1 ]; then
    echo "not a zip" > "$2"
    exit 0
fi
if [ -n "${FT_PY:-}" ]; then
    "$FT_PY" -I -W ignore -c '
import json, os, sys, zipfile
with zipfile.ZipFile(sys.argv[1], "w") as z:
    for root, _dirs, files in os.walk(sys.argv[2]):
        for f in sorted(files):
            p = os.path.join(root, f)
            z.write(p, p.replace(os.sep, "/"))
    for name, text in json.loads(os.environ.get("FT_ZIP_ADD") or "[]"):
        z.writestr(name, text.encode("utf-8"))
' "$2" "$3"
else
    { printf 'PK\005\006'; head -c 18 /dev/zero; } > "$2"
fi
EOF
chmod +x "$STUBS/cargo" "$STUBS/rustc" "$STUBS/zip"

# make_zip <out.zip> <entry name> <content>: a real zip with one entry.
make_zip() {
    "$PY" -I -c '
import sys, zipfile
with zipfile.ZipFile(sys.argv[1], "w") as z:
    z.writestr(sys.argv[2], sys.argv[3].encode("utf-8"))
' "$1" "$2" "$3"
}
# jpairs <name> <text> [<name> <text>...]: FT_ZIP_ADD JSON.
jpairs() {
    "$PY" -I -c 'import json, sys; a = sys.argv[1:]; print(json.dumps([[a[i], a[i + 1]] for i in range(0, len(a), 2)]))' "$@"
}
# stub_dir <name> <script body>: a new directory with one executable stub
# <name>, for XPATH (put before the shared stubs on PATH).
stub_dir() {
    local d="$WORK/x-$1-$RANDOM$RANDOM"
    mkdir -p "$d"
    printf '#!/usr/bin/env bash\n%s\n' "$2" > "$d/$1"
    chmod +x "$d/$1"
    printf '%s' "$d"
}
MARKER_TEXT=$'# Third-party notices\r\n\r\nFFmpeg is not bundled. (Owner to\r\n   confirm this paragraph.)\r\n'
CLEAN_TEXT=$'# Third-party notices\r\n\r\nFFmpeg is not bundled; the owner confirmed this paragraph.\r\n'
MARKER_ZIP="$WORK/marker.zip"
if [ -n "$PY" ]; then
    make_zip "$MARKER_ZIP" "FrameTrace-9.9.9-win64/THIRD_PARTY_NOTICES.md" "$MARKER_TEXT"
fi

# write_manifest <ffmpeg notice_confirmed JSON value | -> [extra tool JSON]:
# one installable tool (sha256 pinned) and one manual-only tool (sha256
# null, no notice_confirmed); "-" leaves ffmpeg's key out.
SHA64="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
write_manifest() {
    local conf="" extra=""
    [ "$1" = - ] || conf=", \"notice_confirmed\": $1"
    [ -n "${2:-}" ] && extra=", $2"
    mkdir -p "$REPO/tools"
    printf '{"schema": 1, "tools": [{"name": "ffmpeg", "kind": "archive", "sha256": "%s"%s}, {"name": "libewf", "kind": "archive", "sha256": null}%s]}\n' \
        "$SHA64" "$conf" "$extra" > "$REPO/tools/manifest.json"
}
# fresh_repo: a new fixture repo at $REPO with an empty dist/, a confirmed
# tools/manifest.json, a clean THIRD_PARTY_NOTICES.md and a LICENSE.
n=0
fresh_repo() {
    n=$((n + 1))
    REPO="$WORK/repo$n"
    mkdir -p "$REPO/scripts" "$REPO/target/release" "$REPO/dist"
    cp "$ROOT/scripts/build-release.sh" "$REPO/scripts/"
    printf '[package]\nname = "frametrace"\nversion = "9.9.9"\n' > "$REPO/Cargo.toml"
    echo readme > "$REPO/README.md"
    printf '%s' "$CLEAN_TEXT" > "$REPO/THIRD_PARTY_NOTICES.md"
    echo "MIT License (fixture)" > "$REPO/LICENSE"
    write_manifest true
    echo bin > "$REPO/target/release/frametrace"
    : > "$WORK/log$n"
}
# run_script [env...]: runs the copy; sets RC, OUT, CARGO_CALLS.
# XPATH (optional, colon-separated) goes before the shared stubs on PATH.
SKIP_DEFAULT=""
[ -n "$PY" ] || SKIP_DEFAULT=1
run_script() {
    OUT="$(env -u CARGO_TARGET_DIR -u CARGO_BUILD_TARGET_DIR PATH="${XPATH:+$XPATH:}$STUBS:$PATH" FT_LOG="$WORK/log$n" FT_MARKER_ZIP="$MARKER_ZIP" \
        FT_PY="$PY" FRAMETRACE_SKIP_NOTICES_CHECK="$SKIP_DEFAULT" "$@" \
        bash "$REPO/scripts/build-release.sh" 2>&1)"
    RC=$?
    CARGO_CALLS="$(grep -c '^cargo ' "$WORK/log$n" || true)"
}
OUR_ZIP_NAME="frametrace-9.9.9-x86_64-unknown-linux-gnu"
has_sums() { [ -f "$REPO/dist/SHA256SUMS" ]; }
no_sums() { [ ! -f "$REPO/dist/SHA256SUMS" ]; }
out_has() { printf '%s' "$OUT" | grep -qF -- "$1"; }
sums_has() { grep -qF -- "$1" "$REPO/dist/SHA256SUMS"; }
sums_lacks() { ! tr '[:upper:]' '[:lower:]' < "$REPO/dist/SHA256SUMS" | grep -qF -- "$1"; }
# SHA256SUMS is exactly one line: the hash of the zip this run built.
sums_only_ours() {
    [ -f "$REPO/dist/SHA256SUMS" ] && [ -f "$REPO/dist/$OUR_ZIP_NAME.zip" ] || return 1
    [ "$(cat "$REPO/dist/SHA256SUMS")" = "$(cd "$REPO/dist" && shasum -a 256 -- "$OUR_ZIP_NAME.zip")" ]
}
no_zip() { [ ! -f "$REPO/dist/$OUR_ZIP_NAME.zip" ]; }
# refused_src <label>: exit 1, cargo not run, no zip, no SHA256SUMS, message.
refused_src() {
    check "$1: exit 1" [ "$RC" -eq 1 ]
    check "$1: cargo not run" [ "$CARGO_CALLS" -eq 0 ]
    check "$1: no SHA256SUMS" no_sums
    check "$1: no zip built" no_zip
    check "$1: says not releasable" out_has "the working tree is not releasable — NOT FOR RELEASE"
}
# refused_built <label> <expected problem text>: exit 1 after the build, the
# built zip deleted, no SHA256SUMS, the problem named.
refused_built() {
    check "$1: exit 1" [ "$RC" -eq 1 ]
    check "$1: cargo ran once" [ "$CARGO_CALLS" -eq 1 ]
    check "$1: no SHA256SUMS" no_sums
    check "$1: the built zip was deleted" no_zip
    check "$1: says the built zip is not releasable" out_has "the zip this run built is not releasable — NOT FOR RELEASE"
    check "$1: names '$2'" out_has "$2"
}

# --- M3: any zip in dist/ this run did not build stops it, before the build --
for name in \
    "FrameTrace-9.9.9-win64-UNCONFIRMED.zip" \
    "FrameTrace-9.9.9-win64-unconfirmed.zip" \
    "FrameTrace-9.9.9-win64-UNCONFIRMED (1).zip" \
    "FrameTrace-9.9.9-win64-UNCONFIRMED - 복사본.zip" \
    "복사본 - FrameTrace-9.9.9-win64-Unconfirmed.ZIP" \
    "FrameTrace-9.9.9-win64-bundled.zip" \
    "FrameTrace-9.9.9-win64.zip" \
    "other-1.0-aarch64.zip" \
    "Outer.ZIP" \
    ".hidden.zip" \
    "$OUR_ZIP_NAME.zip"; do
    fresh_repo
    : > "$REPO/dist/$name"
    run_script
    check "stray '$name': exit 1" [ "$RC" -eq 1 ]
    check "stray '$name': cargo not run" [ "$CARGO_CALLS" -eq 0 ]
    check "stray '$name': no SHA256SUMS" no_sums
    check "stray '$name': message names it" out_has "dist/$name"
    check "stray '$name': tells the releaser to clean dist/" out_has "clean dist/"
    check "stray '$name': NOT FOR RELEASE" out_has "NOT FOR RELEASE"
    case "$(printf '%s' "$name" | tr '[:upper:]' '[:lower:]')" in
        *unconfirmed*) check "stray '$name': called an UNCONFIRMED test package" out_has "an UNCONFIRMED test package by its name" ;;
    esac
done
# a stray zip and an unconfirmed working tree: both are reported
fresh_repo
: > "$REPO/dist/old.zip"
printf '%s' "$MARKER_TEXT" > "$REPO/THIRD_PARTY_NOTICES.md"
run_script
check "stray + working-tree marker: exit 1" [ "$RC" -eq 1 ]
check "stray + working-tree marker: names the zip" out_has "dist/old.zip"
check "stray + working-tree marker: names the notices" out_has "still says 'Owner to confirm'"

# --- planted during the build: the second check ------------------------------
fresh_repo
run_script PLANT=name
check "planted copy name: exit 1" [ "$RC" -eq 1 ]
check "planted copy name: cargo ran once" [ "$CARGO_CALLS" -eq 1 ]
check "planted copy name: no SHA256SUMS" no_sums
check "planted copy name: says the run did not build it" out_has "dist/ holds zips this run did not build"
if [ -n "$PY" ]; then
    fresh_repo
    run_script PLANT=marker
    check "planted win64 zip: exit 1" [ "$RC" -eq 1 ]
    check "planted win64 zip: no SHA256SUMS" no_sums
    check "planted win64 zip: names it" out_has "dist/FrameTrace-9.9.9-win64.zip"
fi

# --- clean dist/ -----------------------------------------------------------
fresh_repo
run_script
check "clean: exit 0" [ "$RC" -eq 0 ]
check "clean: SHA256SUMS lists exactly the zip this run built" sums_only_ours
# other files (not zips) in dist/ are left alone and never listed; a stale
# SHA256SUMS is replaced
fresh_repo
echo notes > "$REPO/dist/notes.txt"
mkdir -p "$REPO/dist/FrameTrace-9.9.9-win64"
echo "deadbeef  bogus.zip" > "$REPO/dist/SHA256SUMS"
run_script
check "non-zip files in dist/: exit 0" [ "$RC" -eq 0 ]
check "non-zip files in dist/: SHA256SUMS lists exactly our zip (stale one replaced)" sums_only_ours

# --- SHA256SUMS lists the built zip by path, even without the second check -----
fresh_repo
# Drop the second stray-zip check (the last bare `refuse_stray_zips` line).
awk '{a[NR]=$0} $0=="refuse_stray_zips"{last=NR} END{for(i=1;i<=NR;i++) if(i!=last) print a[i]}' \
    "$REPO/scripts/build-release.sh" > "$REPO/scripts/b.sh" && mv "$REPO/scripts/b.sh" "$REPO/scripts/build-release.sh"
run_script PLANT=name
check "exact paths only (second check removed): exit 0" [ "$RC" -eq 0 ]
if has_sums; then
    check "exact paths only: the planted zip is not in SHA256SUMS" sums_lacks "unconfirmed"
    check "exact paths only: SHA256SUMS lists exactly our zip" sums_only_ours
else
    bad "exact paths only: SHA256SUMS missing"
fi

# --- the zip this run built: content checks (M3, L-marker) --------------------
BSDTAR_DIR=""
if [ -n "${FRAMETRACE_TEST_BSDTAR:-}" ]; then
    case "$("$FRAMETRACE_TEST_BSDTAR" --version 2>/dev/null)" in
        *[Bb][Ss][Dd][Tt][Aa][Rr]*) BSDTAR_DIR="$(stub_dir tar "exec \"$FRAMETRACE_TEST_BSDTAR\" \"\$@\"")" ;;
        *) bad "FRAMETRACE_TEST_BSDTAR=$FRAMETRACE_TEST_BSDTAR is not bsdtar" ;;
    esac
fi
HAVE_TAR_READER=0
if [ -n "$BSDTAR_DIR" ]; then
    HAVE_TAR_READER=1
else
    case "$(tar --version 2>/dev/null)" in *[Bb][Ss][Dd][Tt][Aa][Rr]*) HAVE_TAR_READER=1 ;; esac
fi
# run_reader <reader> [env...]: run_script with FRAMETRACE_ZIP_READER (and
# bsdtar on PATH for the tar reader).
run_reader() {
    local reader="$1"; shift
    if [ "$reader" = tar ]; then
        XPATH="${BSDTAR_DIR}${XPATH:+:$XPATH}" run_script FRAMETRACE_ZIP_READER=tar "$@"
    else
        run_script FRAMETRACE_ZIP_READER="$reader" "$@"
    fi
}
P="$OUR_ZIP_NAME"
if [ -z "$PY" ]; then
    skp "built-zip content cases (no python3/python for the stub zip)"
else
    READERS=("" python)
    if [ "$HAVE_UNZIP" -eq 1 ]; then READERS+=(unzip); else skp "built-zip content cases with the unzip reader (no unzip)"; fi
    if [ "$HAVE_TAR_READER" -eq 1 ]; then READERS+=(tar); else skp "built-zip content cases with the bsdtar reader (no bsdtar; set FRAMETRACE_TEST_BSDTAR=<path>)"; fi
    # Every reader on the main cases.
    for reader in "${READERS[@]}"; do
        label="${reader:-default}"
        fresh_repo; run_reader "$reader" FT_ZIP_ADD="$(jpairs "$P/docs/THIRD_PARTY_NOTICES.md" "$MARKER_TEXT")"
        refused_built "built zip, marker notices, reader $label" "THIRD_PARTY_NOTICES.md: Owner to confirm"
        fresh_repo; run_reader "$reader" FT_ZIP_ADD="$(jpairs "$P/UNCONFIRMED.txt" "pending")"
        refused_built "built zip, UNCONFIRMED.txt, reader $label" "UNCONFIRMED.txt entry '$P/UNCONFIRMED.txt'"
        fresh_repo; run_reader "$reader" FT_ZIP_ADD="$(jpairs "$P/tools/bin/ffmpeg.exe" "MZ")"
        refused_built "built zip, ffmpeg.exe, reader $label" "FFmpeg entry '$P/tools/bin/ffmpeg.exe'"
        fresh_repo; run_reader "$reader" FT_ZIP_ADD="$(jpairs "$P/codecs.zip" "PK")"
        refused_built "built zip, nested codecs.zip, reader $label" "nested archive '$P/codecs.zip'"
        fresh_repo; run_reader "$reader" FT_ZIP_ADD="$(jpairs "$P/docs/notes.txt" "ffmpeg is not included")"
        check "built zip, a clean extra entry, reader $label: exit 0" [ "$RC" -eq 0 ]
        check "built zip, a clean extra entry, reader $label: SHA256SUMS lists exactly our zip" sums_only_ours
    done
    # The name rules, default reader.
    for entry in "unconfirmed.TXT" "a/b/Unconfirmed.txt" "$P/UNCONFIRMED.txt ." ; do
        fresh_repo; run_script FT_ZIP_ADD="$(jpairs "$entry" "pending")"
        refused_built "built zip, UNCONFIRMED.txt as '$entry'" "UNCONFIRMED.txt entry"
    done
    for entry in "$P/FFProbe-Old.EXE" "$P/libavcodec-61.dll" "$P/tools/bin/ffmpeg.EXE " "$P/ffplay.exe." \
        "$P/tools/bin/licenses/ffmpeg-LICENSE.txt" "$P/swscale-8.dll" "$P/lib/avutil.so"; do
        fresh_repo; run_script FT_ZIP_ADD="$(jpairs "$entry" "x")"
        refused_built "built zip, FFmpeg name '$entry'" "FFmpeg entry '$entry'"
    done
    for entry in "$P/codecs.7z" "$P/tools.tar.gz" "$P/a.tar" "$P/b.cab" "$P/C.ZIP" "$P/x.tgz" "$P/y.rar" "$P/FrameTrace-9.9.9-win64-UNCONFIRMED.zip"; do
        fresh_repo; run_script FT_ZIP_ADD="$(jpairs "$entry" "x")"
        refused_built "built zip, nested archive '$entry'" "nested archive '$entry'"
    done
    # several problems: each one named
    fresh_repo; run_script FT_ZIP_ADD="$(jpairs "$P/ffmpeg.exe" "MZ" "$P/UNCONFIRMED.txt" "p" "$P/in.zip" "PK")"
    refused_built "built zip, three problems" "FFmpeg entry '$P/ffmpeg.exe'"
    check "built zip, three problems: names UNCONFIRMED.txt" out_has "UNCONFIRMED.txt entry"
    check "built zip, three problems: names the nested zip" out_has "nested archive '$P/in.zip'"
    # a backslash separator (hand-made PS 5.1 zips): python normalises it,
    # Info-ZIP lists it verbatim — the leaf is taken after either separator.
    for reader in "" unzip; do
        [ "$reader" = unzip ] && [ "$HAVE_UNZIP" -eq 0 ] && continue
        fresh_repo; run_reader "$reader" FT_ZIP_ADD="$(jpairs "$P\\tools\\ffplay.exe" "MZ")"
        refused_built "built zip, backslash ffplay entry, reader ${reader:-default}" "FFmpeg entry"
    done
    # lower-case notices at the zip root, re-wrapped marker
    fresh_repo; run_script FT_ZIP_ADD="$(jpairs "third_party_notices.md" $'x\n(owner TO\n\tconfirm)\n')"
    refused_built "built zip, root lower-case notices marker" "THIRD_PARTY_NOTICES.md: Owner to confirm"
    # duplicated entry name: a marked copy hidden behind the clean staged one
    # (zipfile's read(name) returns only the LAST copy).
    for reader in "" python unzip; do
        [ "$reader" = unzip ] && [ "$HAVE_UNZIP" -eq 0 ] && continue
        fresh_repo; run_reader "$reader" FT_ZIP_ADD="$(jpairs "$P/THIRD_PARTY_NOTICES.md" "$MARKER_TEXT" "$P/THIRD_PARTY_NOTICES.md" "$CLEAN_TEXT")"
        refused_built "built zip, duplicate notices (marker first), reader ${reader:-default}" "THIRD_PARTY_NOTICES.md: Owner to confirm"
    done
    # '[x]' in an entry path: unzip/bsdtar read name arguments as patterns —
    # their notices read fails (cannot verify), python reads it.
    BRACKET="$P[x]/THIRD_PARTY_NOTICES.md"
    fresh_repo; run_script FT_ZIP_ADD="$(jpairs "$BRACKET" "$MARKER_TEXT")"
    refused_built "built zip, '[x]' marker, default reader" "THIRD_PARTY_NOTICES.md: Owner to confirm"
    fresh_repo; run_script FT_ZIP_ADD="$(jpairs "$BRACKET" "$CLEAN_TEXT")"
    check "built zip, '[x]' clean, default reader: exit 0" [ "$RC" -eq 0 ]
    for reader in unzip tar; do
        [ "$reader" = unzip ] && [ "$HAVE_UNZIP" -eq 0 ] && continue
        [ "$reader" = tar ] && [ "$HAVE_TAR_READER" -eq 0 ] && continue
        fresh_repo; run_reader "$reader" FT_ZIP_ADD="$(jpairs "$BRACKET" "$CLEAN_TEXT")"
        refused_built "built zip, '[x]' clean, $reader only (cannot verify, not clean)" "THIRD_PARTY_NOTICES.md: cannot verify — every zip reader failed on it"
    done
    # a zip no reader can read
    for reader in "" tar; do
        [ "$reader" = tar ] && [ "$HAVE_TAR_READER" -eq 0 ] && continue
        fresh_repo; run_reader "$reader" FT_ZIP_BROKEN=1
        refused_built "built zip unreadable, reader ${reader:-default}" "cannot verify — every zip reader failed on it"
    done
    # no reader: fails closed; FRAMETRACE_SKIP_NOTICES_CHECK=1 checks the
    # staged directory instead (with a warning) — still refusing what it finds
    fresh_repo; run_script FRAMETRACE_ZIP_READER=none
    refused_built "built zip, no reader" "cannot verify — no zip reader"
    check "built zip, no reader: names the override" out_has "FRAMETRACE_SKIP_NOTICES_CHECK=1"
    fresh_repo; run_script FRAMETRACE_ZIP_READER=none FRAMETRACE_SKIP_NOTICES_CHECK=1
    check "built zip, no reader + SKIP=1: exit 0" [ "$RC" -eq 0 ]
    check "built zip, no reader + SKIP=1: warns it checked the staged directory" out_has "checked through its staged directory"
    check "built zip, no reader + SKIP=1: SHA256SUMS lists exactly our zip" sums_only_ours
    fresh_repo; run_script FRAMETRACE_ZIP_READER=none FRAMETRACE_SKIP_NOTICES_CHECK=1 FT_STAGE_ADD="tools/bin/ffmpeg.exe"
    refused_built "built zip, no reader + SKIP=1, staged ffmpeg.exe" "FFmpeg entry '$P/tools/bin/ffmpeg.exe'"
    fresh_repo; run_script FRAMETRACE_ZIP_READER=none FRAMETRACE_SKIP_NOTICES_CHECK=1 FT_STAGE_ADD="UNCONFIRMED.txt"
    refused_built "built zip, no reader + SKIP=1, staged UNCONFIRMED.txt" "UNCONFIRMED.txt entry"
    fresh_repo; run_script FRAMETRACE_ZIP_READER=none FRAMETRACE_SKIP_NOTICES_CHECK=1 FT_STAGE_ADD="inner/codecs.7z"
    refused_built "built zip, no reader + SKIP=1, staged codecs.7z" "nested archive '$P/inner/codecs.7z'"
    # staged files reach the zip: the normal readers see them too
    fresh_repo; run_script FT_STAGE_ADD="tools/bin/ffprobe.exe"
    refused_built "built zip, staged ffprobe.exe, default reader" "FFmpeg entry '$P/tools/bin/ffprobe.exe'"

    # failing readers: a stub unzip that exits 1; Store-alias python stubs
    FAILING_UNZIP="$(stub_dir unzip 'echo "unzip: stub failure" >&2; exit 1')"
    STORE_MSG='echo "Python was not found; run without arguments to install from the Microsoft Store" >&2; exit 9009'
    STORE_PY3="$(stub_dir python3 "$STORE_MSG")"
    STORE_PY="$(stub_dir python "$STORE_MSG")"
    REAL_PY="$(stub_dir python "exec \"$PY\" \"\$@\"")"
    HAVE_SYS_BSDTAR=0
    case "$(tar --version 2>/dev/null)" in *[Bb][Ss][Dd][Tt][Aa][Rr]*) HAVE_SYS_BSDTAR=1 ;; esac
    fresh_repo; XPATH="$FAILING_UNZIP" run_script FRAMETRACE_ZIP_READER=unzip
    refused_built "failing unzip, unzip only" "cannot verify — every zip reader failed on it"
    # python3 is the Store stub: probed and skipped, `python` reads the zip
    # (unzip is failing, so a found marker proves python did the reading).
    fresh_repo; XPATH="$STORE_PY3:$REAL_PY:$FAILING_UNZIP" run_script FT_ZIP_ADD="$(jpairs "$P/x/THIRD_PARTY_NOTICES.md" "$MARKER_TEXT")"
    refused_built "Store python3: python found the marker" "THIRD_PARTY_NOTICES.md: Owner to confirm"
    fresh_repo; XPATH="$STORE_PY3:$REAL_PY:$FAILING_UNZIP" run_script
    check "Store python3, clean: exit 0 via python" [ "$RC" -eq 0 ]
    if [ -n "$NODE" ]; then
        # no python at all (manifest via node) + failing unzip: nothing can read it
        if [ "$HAVE_SYS_BSDTAR" -eq 0 ]; then
            fresh_repo; XPATH="$STORE_PY3:$STORE_PY:$FAILING_UNZIP" run_script
            refused_built "Store pythons + failing unzip" "cannot verify — every zip reader failed on it"
        else
            skp "Store pythons + failing unzip (bsdtar present)"
        fi
        if [ "$HAVE_UNZIP" -eq 1 ]; then
            # both pythons are Store stubs: unzip reads the zip, node the manifest
            fresh_repo; XPATH="$STORE_PY3:$STORE_PY" run_script FT_ZIP_ADD="$(jpairs "$P/x/THIRD_PARTY_NOTICES.md" "$MARKER_TEXT")"
            refused_built "Store python3 + python: unzip found the marker" "THIRD_PARTY_NOTICES.md: Owner to confirm"
            fresh_repo; XPATH="$STORE_PY3:$STORE_PY" run_script
            check "Store python3 + python, clean: exit 0 via unzip (manifest and build messages via node)" [ "$RC" -eq 0 ]
            check "Store python3 + python, clean: SHA256SUMS lists exactly our zip" sums_only_ours
        else
            skp "Store python3 + python falling to unzip (no unzip)"
        fi
    else
        skp "no-python reader cases (no node for the manifest)"
    fi
fi

# --- the working tree: tools/manifest.json + THIRD_PARTY_NOTICES.md ---------
if [ -z "$PY" ]; then
    skp "manifest notice_confirmed cases (no python3/python)"
else
    fresh_repo; write_manifest false; run_script
    refused_src "manifest notice_confirmed false"
    check "manifest notice_confirmed false: names ffmpeg" out_has "tools/manifest.json: notice_confirmed is not true for: ffmpeg"

    fresh_repo; write_manifest -; run_script
    refused_src "manifest notice_confirmed missing"
    check "manifest notice_confirmed missing: names ffmpeg" out_has "notice_confirmed is not true for: ffmpeg"

    fresh_repo; write_manifest '"true"'; run_script
    refused_src 'manifest notice_confirmed "true" (string)'
    check 'manifest notice_confirmed "true" (string): names ffmpeg' out_has "notice_confirmed is not true for: ffmpeg"

    fresh_repo; write_manifest 1; run_script
    refused_src "manifest notice_confirmed 1 (number)"

    # L-keycase: only the exact key counts
    fresh_repo; write_manifest - ; sed -i.bak 's/"sha256": "a/"Notice_Confirmed": true, "sha256": "a/' "$REPO/tools/manifest.json"; run_script
    refused_src 'manifest "Notice_Confirmed": true (wrong case)'
    check 'manifest "Notice_Confirmed": true (wrong case): names ffmpeg' out_has "notice_confirmed is not true for: ffmpeg"

    # a second installable tool, unconfirmed: the rule is per tool
    fresh_repo; write_manifest true '{"name": "other", "sha256": "bb", "notice_confirmed": false}'; run_script
    refused_src "manifest, second installable tool unconfirmed"
    check "manifest, second installable tool unconfirmed: names only it" out_has "notice_confirmed is not true for: other"

    # hunch-installable: a pip tool with a requirements file is installable
    fresh_repo; write_manifest true '{"name": "lens-pip", "kind": "pip", "sha256": null, "requirements_file": "tools/lens.requirements.txt"}'; run_script
    refused_src "manifest, pip tool with requirements_file unconfirmed"
    check "manifest, pip tool with requirements_file unconfirmed: names it" out_has "notice_confirmed is not true for: lens-pip"
    fresh_repo; write_manifest true '{"name": "lens-pip", "kind": "pip", "requirements_file": "tools/lens.requirements.txt", "notice_confirmed": true}'; run_script
    check "manifest, pip tool with requirements_file confirmed: exit 0" [ "$RC" -eq 0 ]
    fresh_repo; write_manifest true '{"name": "lens-pip", "kind": "pip", "requirements_file": null}'; run_script
    check "manifest, pip tool without requirements_file (not installable): exit 0" [ "$RC" -eq 0 ]

    # sha256 "" is not installable (make-portable.ps1's rule): no notice needed
    fresh_repo; write_manifest true '{"name": "manual", "sha256": ""}'; run_script
    check 'manifest, sha256 "" tool without notice_confirmed: exit 0' [ "$RC" -eq 0 ]
    # a wrong-case "SHA256" is not the key serde reads: not installable
    fresh_repo; write_manifest true '{"name": "odd", "SHA256": "bb"}'; run_script
    check 'manifest, wrong-case "SHA256" only (not installable): exit 0' [ "$RC" -eq 0 ]

    fresh_repo; rm -f "$REPO/tools/manifest.json"; run_script
    refused_src "manifest missing"
    check "manifest missing: says not found" out_has "tools/manifest.json: not found"

    fresh_repo; echo '{"tools": [' > "$REPO/tools/manifest.json"; run_script
    refused_src "manifest not JSON"
    check "manifest not JSON: says cannot verify" out_has "tools/manifest.json: cannot verify"

    fresh_repo; echo '{"tools": [1]}' > "$REPO/tools/manifest.json"; run_script
    refused_src "manifest tool entry not an object"

    # the repository's own manifest
    fresh_repo; cp "$ROOT/tools/manifest.json" "$REPO/tools/manifest.json"; run_script
    if "$PY" -I -c '
import json, sys
m = json.load(open(sys.argv[1], encoding="utf-8-sig"))
def inst(t):
    s, r = t.get("sha256"), t.get("requirements_file")
    return s not in (None, "") or (t.get("kind") == "pip" and r not in (None, ""))
sys.exit(0 if all(t.get("notice_confirmed") is True for t in m["tools"] if inst(t)) else 1)' "$ROOT/tools/manifest.json"; then
        check "real tools/manifest.json (confirmed): exit 0" [ "$RC" -eq 0 ]
    else
        refused_src "real tools/manifest.json (unconfirmed)"
        check "real tools/manifest.json (unconfirmed): names ffmpeg" out_has "notice_confirmed is not true for: ffmpeg"
    fi

    # Fix round 13 L3: python's own error is printed with the refusal
    fresh_repo; echo '{"tools": [' > "$REPO/tools/manifest.json"; run_script
    check "manifest not JSON (python): prints the cause" out_has "python: JSONDecodeError"
    fresh_repo; printf '\357\273\277' > "$REPO/tools/manifest.json"; printf '{"tools": [{"name": "ffmpeg", "sha256": "%s", "notice_confirmed": true}]}\n' "$SHA64" >> "$REPO/tools/manifest.json"; run_script
    check "manifest with a UTF-8 BOM (python): exit 0" [ "$RC" -eq 0 ]
    # ... and the override never skips a verdict python could give
    fresh_repo; write_manifest false; run_script FRAMETRACE_SKIP_NOTICES_CHECK=1
    refused_src "manifest notice_confirmed false + SKIP=1"
fi

# --- the manifest without python: node, else refused with no override --------
NOPY3="$(stub_dir python3 'exit 9009')"
NOPY="$(stub_dir python 'exit 9009')"
NONODE="$(stub_dir node 'exit 127')"
if [ -n "$NODE" ]; then
    NOPYS="$NOPY3:$NOPY"
    fresh_repo; XPATH="$NOPYS" run_script FRAMETRACE_SKIP_NOTICES_CHECK=1
    check "manifest via node (no python), confirmed: exit 0" [ "$RC" -eq 0 ]
    check "manifest via node (no python), confirmed: cargo ran once" [ "$CARGO_CALLS" -eq 1 ]
    check "manifest via node (no python), confirmed: no 'NOT checked' warning" bash -c '! printf "%s" "$1" | grep -qF "notice_confirmed NOT checked"' _ "$OUT"
    fresh_repo; write_manifest false; XPATH="$NOPYS" run_script
    refused_src "manifest via node, notice_confirmed false"
    check "manifest via node, notice_confirmed false: names ffmpeg" out_has "tools/manifest.json: notice_confirmed is not true for: ffmpeg"
    fresh_repo; write_manifest -; XPATH="$NOPYS" run_script
    refused_src "manifest via node, notice_confirmed missing"
    fresh_repo; write_manifest '"true"'; XPATH="$NOPYS" run_script
    refused_src 'manifest via node, notice_confirmed "true" (string)'
    fresh_repo; write_manifest 1; XPATH="$NOPYS" run_script
    refused_src "manifest via node, notice_confirmed 1 (number)"
    fresh_repo; write_manifest - ; sed -i.bak 's/"sha256": "a/"Notice_Confirmed": true, "sha256": "a/' "$REPO/tools/manifest.json"; XPATH="$NOPYS" run_script
    refused_src 'manifest via node, "Notice_Confirmed": true (wrong case)'
    fresh_repo; write_manifest true '{"name": "other", "sha256": "bb", "notice_confirmed": false}'; XPATH="$NOPYS" run_script
    refused_src "manifest via node, second installable tool unconfirmed"
    check "manifest via node, second installable tool unconfirmed: names only it" out_has "notice_confirmed is not true for: other"
    fresh_repo; write_manifest true '{"name": "lens-pip", "kind": "pip", "requirements_file": "tools/lens.requirements.txt"}'; XPATH="$NOPYS" run_script
    refused_src "manifest via node, pip tool with requirements_file unconfirmed"
    check "manifest via node, pip tool unconfirmed: names it" out_has "notice_confirmed is not true for: lens-pip"
    fresh_repo; write_manifest true '{"name": "lens-pip", "kind": "pip", "requirements_file": null}'; XPATH="$NOPYS" run_script FRAMETRACE_SKIP_NOTICES_CHECK=1
    check "manifest via node, pip tool without requirements_file: exit 0" [ "$RC" -eq 0 ]
    fresh_repo; write_manifest true '{"name": "manual", "sha256": ""}'; XPATH="$NOPYS" run_script FRAMETRACE_SKIP_NOTICES_CHECK=1
    check 'manifest via node, sha256 "" tool without notice_confirmed: exit 0' [ "$RC" -eq 0 ]
    fresh_repo; printf '\357\273\277' > "$REPO/tools/manifest.json"; printf '{"tools": [{"name": "ffmpeg", "sha256": "%s", "notice_confirmed": true}]}\n' "$SHA64" >> "$REPO/tools/manifest.json"; XPATH="$NOPYS" run_script FRAMETRACE_SKIP_NOTICES_CHECK=1
    check "manifest via node with a UTF-8 BOM: exit 0" [ "$RC" -eq 0 ]
    fresh_repo; echo '{"tools": [' > "$REPO/tools/manifest.json"; XPATH="$NOPYS" run_script
    refused_src "manifest via node, not JSON"
    check "manifest via node, not JSON: says cannot verify" out_has "tools/manifest.json: cannot verify — not readable as a tool manifest"
    check "manifest via node, not JSON: prints node's cause" out_has "(node: "
    fresh_repo; echo '{"schema": 1}' > "$REPO/tools/manifest.json"; XPATH="$NOPYS" run_script
    refused_src "manifest via node, no tools array"
    check "manifest via node, no tools array: prints the cause" out_has 'no "tools" array'
    fresh_repo; write_manifest false; XPATH="$NOPYS" run_script FRAMETRACE_SKIP_NOTICES_CHECK=1
    refused_src "manifest via node, false + SKIP=1"
    # the repository's own manifest: the same verdict as the reference rule
    fresh_repo; cp "$ROOT/tools/manifest.json" "$REPO/tools/manifest.json"; XPATH="$NOPYS" run_script FRAMETRACE_SKIP_NOTICES_CHECK=1
    if node -e '
const m = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8").replace(/^\uFEFF/, ""));
const given = (v) => v !== undefined && v !== null && v !== "";
process.exit(m.tools.every((t) => !(given(t.sha256) || (t.kind === "pip" && given(t.requirements_file))) || t.notice_confirmed === true) ? 0 : 1)' "$ROOT/tools/manifest.json"; then
        check "real tools/manifest.json via node (confirmed): exit 0" [ "$RC" -eq 0 ]
    else
        refused_src "real tools/manifest.json via node (unconfirmed)"
        check "real tools/manifest.json via node (unconfirmed): names ffmpeg" out_has "notice_confirmed is not true for: ffmpeg"
    fi
else
    skp "manifest via node cases (no node)"
fi
# neither python nor node: fails closed, and FRAMETRACE_SKIP_NOTICES_CHECK=1
# does not change that (Fix round 13 L3)
fresh_repo; XPATH="$NOPY3:$NOPY:$NONODE" run_script
refused_src "manifest, no python and no node"
check "manifest, no python and no node: says cannot verify" out_has "tools/manifest.json: cannot verify — no python3/python or node"
fresh_repo; XPATH="$NOPY3:$NOPY:$NONODE" run_script FRAMETRACE_SKIP_NOTICES_CHECK=1
refused_src "manifest, no python and no node + SKIP=1 (no override)"
check "manifest, no python and no node + SKIP=1: no 'NOT checked' warning" bash -c '! printf "%s" "$1" | grep -qF "notice_confirmed NOT checked"' _ "$OUT"

# THIRD_PARTY_NOTICES.md in the working tree (no python needed)
fresh_repo; printf '%s' "$MARKER_TEXT" > "$REPO/THIRD_PARTY_NOTICES.md"; run_script
refused_src "working-tree notices marker (re-wrapped, CRLF)"
check "working-tree notices marker: names it" out_has "THIRD_PARTY_NOTICES.md: still says 'Owner to confirm'"
fresh_repo; printf '%s' "$MARKER_TEXT" > "$REPO/THIRD_PARTY_NOTICES.md"; run_script FRAMETRACE_SKIP_NOTICES_CHECK=1
refused_src "working-tree notices marker + SKIP=1"
fresh_repo; rm -f "$REPO/THIRD_PARTY_NOTICES.md"; run_script
refused_src "working-tree notices missing"
# Hunch-unreadable-notices: an existing but unreadable file fails closed.
fresh_repo; chmod 000 "$REPO/THIRD_PARTY_NOTICES.md"
if [ ! -r "$REPO/THIRD_PARTY_NOTICES.md" ]; then
    run_script
    refused_src "working-tree notices unreadable (chmod 000)"
    check "working-tree notices unreadable (chmod 000): says it cannot be read" out_has "THIRD_PARTY_NOTICES.md: cannot be read"
else
    skp "working-tree notices chmod 000 (still readable here: Windows or root)"
fi
chmod 644 "$REPO/THIRD_PARTY_NOTICES.md"
# ... and a read that fails although the file looks readable (I/O error)
FAILING_CAT="$(stub_dir cat "case \"\$*\" in *THIRD_PARTY_NOTICES.md*) echo 'cat: THIRD_PARTY_NOTICES.md: Input/output error' >&2; exit 1 ;; esac; exec \"$REAL_CAT\" \"\$@\"")"
fresh_repo; XPATH="$FAILING_CAT" run_script
refused_src "working-tree notices read fails"
check "working-tree notices read fails: says it cannot be read" out_has "THIRD_PARTY_NOTICES.md: cannot be read — cannot verify"
if [ -n "$PY" ]; then
    fresh_repo; write_manifest false; printf '%s' "$MARKER_TEXT" > "$REPO/THIRD_PARTY_NOTICES.md"; run_script
    refused_src "manifest + notices both unconfirmed"
    check "manifest + notices both unconfirmed: names the manifest" out_has "notice_confirmed is not true for: ffmpeg"
    check "manifest + notices both unconfirmed: names the notices" out_has "still says 'Owner to confirm'"
fi

# confirmed + clean: the zip carries THIRD_PARTY_NOTICES.md and LICENSE
fresh_repo; run_script
check "confirmed + clean notices: exit 0" [ "$RC" -eq 0 ]
check "confirmed + clean notices: cargo ran once" [ "$CARGO_CALLS" -eq 1 ]
check "confirmed + clean notices: cargo asked for JSON messages" grep -q -- '--message-format=json-render-diagnostics' "$WORK/log$n"
check "confirmed + clean notices: SHA256SUMS written" has_sums
check "confirmed + clean notices: staged THIRD_PARTY_NOTICES.md" [ -f "$REPO/dist/$OUR_ZIP_NAME/THIRD_PARTY_NOTICES.md" ]
check "confirmed + clean notices: staged LICENSE" [ -f "$REPO/dist/$OUR_ZIP_NAME/LICENSE" ]
if [ -n "$PY" ]; then
    ENTRIES="$("$PY" -I -c 'import sys, zipfile; print("\n".join(zipfile.ZipFile(sys.argv[1]).namelist()))' "$REPO/dist/$OUR_ZIP_NAME.zip" 2>&1 | tr -d '\r')"
    for e in README.md THIRD_PARTY_NOTICES.md LICENSE frametrace; do
        check "confirmed + clean notices: zip has $e" bash -c 'printf "%s\n" "$1" | grep -qxF -- "$2"' _ "$ENTRIES" "$OUR_ZIP_NAME/$e"
    done
else
    skp "zip entry listing (no python3/python)"
fi

# --- M4: the binaries cargo reports (Fix round 13 H1, round 16 M4) -------------
# fresh_repo leaves a STALE target/release/frametrace ("bin"); with a target
# directory configured, the binary cargo built there is packaged, never the
# stale one.
fresh_repo; run_script CARGO_TARGET_DIR="$WORK/ctd$n"
check "CARGO_TARGET_DIR absolute: exit 0" [ "$RC" -eq 0 ]
check "CARGO_TARGET_DIR absolute: packages the fresh binary" grep -qx fresh-from-cargo "$REPO/dist/$OUR_ZIP_NAME/frametrace"
fresh_repo; run_script CARGO_TARGET_DIR=out/ct
check "CARGO_TARGET_DIR relative: exit 0" [ "$RC" -eq 0 ]
check "CARGO_TARGET_DIR relative: cargo built under the repo root" [ -f "$REPO/out/ct/release/frametrace" ]
check "CARGO_TARGET_DIR relative: packages the fresh binary" grep -qx fresh-from-cargo "$REPO/dist/$OUR_ZIP_NAME/frametrace"
fresh_repo; run_script CARGO_BUILD_TARGET_DIR="$WORK/cbtd$n"
check "CARGO_BUILD_TARGET_DIR: exit 0" [ "$RC" -eq 0 ]
check "CARGO_BUILD_TARGET_DIR: packages the fresh binary" grep -qx fresh-from-cargo "$REPO/dist/$OUR_ZIP_NAME/frametrace"
fresh_repo; mkdir -p "$REPO/.cargo"; printf '[build]\ntarget-dir = "cfg-target"\n' > "$REPO/.cargo/config.toml"; run_script
check ".cargo/config.toml build.target-dir: exit 0" [ "$RC" -eq 0 ]
check ".cargo/config.toml build.target-dir: packages the fresh binary" grep -qx fresh-from-cargo "$REPO/dist/$OUR_ZIP_NAME/frametrace"
# cargo wrote nothing there: the stale target/release one is not taken
for var in CARGO_TARGET_DIR CARGO_BUILD_TARGET_DIR; do
    fresh_repo; run_script "$var=$WORK/empty-$var$n" FT_CARGO_NOWRITE=1
    check "$var without a binary: fails" [ "$RC" -ne 0 ]
    check "$var without a binary: says it does not exist" out_has "but it does not exist"
    check "$var without a binary: no zip" no_zip
    check "$var without a binary: no SHA256SUMS" no_sums
done
# cargo reported no frametrace executable
fresh_repo; run_script FT_CARGO_OMIT=frametrace
check "no frametrace executable reported: fails" [ "$RC" -eq 1 ]
check "no frametrace executable reported: says so" out_has "cargo build reported no executable for the frametrace binary"
check "no frametrace executable reported: no SHA256SUMS" no_sums
# unset: target/release as before; frametrace-app packaged when cargo reports it
fresh_repo; echo app > "$REPO/target/release/frametrace-app"; run_script
check "no target dir set: packages target/release/frametrace" grep -qx bin "$REPO/dist/$OUR_ZIP_NAME/frametrace"
check "no target dir set: packages target/release/frametrace-app" grep -qx app "$REPO/dist/$OUR_ZIP_NAME/frametrace-app"

# --- --help prints the header only -------------------------------------------
fresh_repo
OUT="$(bash "$REPO/scripts/build-release.sh" --help 2>&1)"
check "--help: names make-portable.ps1" out_has "scripts/make-portable.ps1, which writes the same SHA256SUMS."
check "--help: stops before code" bash -c '! printf "%s" "$1" | grep -q "set -euo"' _ "$OUT"

echo
echo "build-release fixture: $pass ok, $fail failed, $skip skipped"
[ "$skip" -eq 0 ] || echo "SKIPPED $skip (see the 'skip' lines above)"
[ "$fail" -eq 0 ] || exit 1
if [ "$skip" -eq 0 ]; then echo "all checks passed"; else echo "all run checks passed ($skip SKIPPED)"; fi
