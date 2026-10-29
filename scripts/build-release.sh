#!/usr/bin/env bash
# Builds the FrameTrace release binaries and packages them as
#   dist/frametrace-<version>-<target-triple>.zip
# (binaries, README.md, THIRD_PARTY_NOTICES.md, LICENSE) plus a
# dist/SHA256SUMS manifest listing exactly the zip(s) this run built.
# Refuses to run while the working tree's notices are unconfirmed
# (tools/manifest.json notice_confirmed, THIRD_PARTY_NOTICES.md "Owner to
# confirm"), while dist/ holds any *.zip this run did not build, or when the
# built zip holds an FFmpeg file, an UNCONFIRMED.txt, a nested archive or
# unconfirmed notices.
#
# Options:
#   --sign   also runs `cosign sign-blob` on the zip and SHA256SUMS
#            (requires cosign on PATH; keyless OIDC or -key, see
#            docs/repro-build.md)
#   --sbom   also emits dist/frametrace-<ver>-<target>.sbom.cyclonedx.json
#            via `cargo sbom` (requires cargo-sbom; see docs/repro-build.md)
#
# The script deliberately adds no build infrastructure: it is the
# documented release procedure made repeatable. Windows packaging lives
# in scripts/make-portable.ps1, which writes the same SHA256SUMS.
set -euo pipefail

cd "$(dirname "$0")/.."

SIGN=0
SBOM=0
for arg in "$@"; do
    case "$arg" in
        --sign) SIGN=1 ;;
        --sbom) SBOM=1 ;;
        -h|--help)
            sed -n '2,21p' "$0"
            exit 0
            ;;
        *) echo "unknown option: $arg" >&2; exit 2 ;;
    esac
done

# What gets listed and signed (§Fix round 16 M3): only the zips this run
# built, tracked by exact path in PRODUCED. Any other *.zip in dist/ (an
# UNCONFIRMED Windows test package from make-portable.ps1
# -AllowUnconfirmedNotices, an old *-win64-bundled.zip with FFmpeg, an outer
# zip wrapping one of those, a zip from another target) stops the run, before
# the build and again before SHA256SUMS — the releaser cleans dist/.
#
# The zips this run built are checked by content before SHA256SUMS: every
# entry against the FFmpeg name rule of make-portable.ps1 (leaf, trimmed of
# spaces and trailing dots, any case, containing ffmpeg/ffprobe/ffplay or an
# FFmpeg library name), no UNCONFIRMED.txt (make-portable.ps1's marker for an
# unconfirmed package, any case, any folder), no nested archive (*.zip *.7z
# *.gz *.tgz *.tar *.cab *.rar *.xz *.bz2 *.zst), and no THIRD_PARTY_NOTICES.md
# (any case, any folder) still matching "Owner to confirm". A zip that fails
# is deleted.
#
# The content check fails closed: a zip it cannot read is refused
# ("cannot verify"), never treated as clean. Readers, in order: Python's
# zipfile (python3, then python — each probed with `import zipfile` first,
# so the Microsoft Store alias stub that exits 9009 is skipped), then
# Info-ZIP `unzip` (-Z1 to list, -p to read), then bsdtar. A reader that
# fails on a zip (non-zero exit: BusyBox unzip without -Z1, encrypted or
# unsupported compression, an entry name unzip would read as a wildcard)
# hands the zip to the next one; when every available reader failed, the
# zip is refused. With no reader at all the run stops too, unless
# FRAMETRACE_SKIP_NOTICES_CHECK=1: then (with a warning) the same checks run
# on the staged directory the zip was made from instead. That is all the
# override does: it never affects the working tree's manifest /
# THIRD_PARTY_NOTICES.md checks (source_problems).
#
# FRAMETRACE_ZIP_READER (python|unzip|tar|none) forces a single zip reader
# (no fallback); it exists for scripts/test-build-release.sh.

# Prints a Python interpreter that can `import zipfile`; fails if none.
find_python() {
    local cand
    for cand in python3 python; do
        command -v "$cand" >/dev/null 2>&1 || continue
        if "$cand" -I -c 'import zipfile' >/dev/null 2>&1; then
            printf '%s\n' "$cand"
            return 0
        fi
    done
    return 1
}

# Whether an entry name cannot be passed to `unzip -p` / `bsdtar -x` as a
# literal (they read it as a wildcard pattern).
pattern_unsafe() {
    case "$1" in
        *'['*|*']'*|*'*'*|*'?'*|*'\'*) return 0 ;;
    esac
    return 1
}

# list_with <reader> <zip>: prints every entry name, one per line. Returns
# 0 on success, 1 when the reader failed on this zip, 3 when unavailable.
list_with() {
    local reader="$1" zip="$2" py
    case "$reader" in
        python)
            py="$(find_python)" || return 3
            "$py" -I -c '
import sys, zipfile
try:
    with zipfile.ZipFile(sys.argv[1]) as z:
        for info in z.infolist():
            # bytes with LF only (a text-mode stdout on Windows writes CRLF)
            name = info.filename.replace("\\", "/").replace("\r", " ").replace("\n", " ")
            sys.stdout.buffer.write(name.encode("utf-8") + b"\n")
    sys.stdout.flush()
except BaseException as e:
    sys.stderr.write("zipfile: %s: %r\n" % (sys.argv[1], e))
    sys.exit(1)
' "$zip" 2>/dev/null || return 1
            ;;
        unzip)
            command -v unzip >/dev/null 2>&1 || return 3
            unzip -Z1 "$zip" 2>/dev/null | tr -d '\r' || return 1
            ;;
        tar)
            # bsdtar (macOS/BSD) reads zip archives; GNU tar does not.
            case "$(tar --version 2>/dev/null)" in
                *[Bb][Ss][Dd][Tt][Aa][Rr]*) ;;
                *) return 3 ;;
            esac
            tar -tf "$zip" 2>/dev/null | tr -d '\r' || return 1
            ;;
        *)
            return 3
            ;;
    esac
}

# read_notices_with <reader> <zip>: prints the THIRD_PARTY_NOTICES.md
# entries (any folder, any case; nothing if there are none). Returns 0 on
# success, 1 when the reader failed on this zip, 3 when it is unavailable.
read_notices_with() {
    local reader="$1" zip="$2" py list entry base norm
    case "$reader" in
        python)
            py="$(find_python)" || return 3
            "$py" -I -c '
import sys, zipfile
try:
    with zipfile.ZipFile(sys.argv[1]) as z:
        # infolist(), not namelist(): z.read(name) returns only the LAST
        # entry of a duplicated name, which would hide a marked copy
        # behind a clean one. Reading by ZipInfo emits every copy.
        for info in z.infolist():
            if info.filename.replace("\\", "/").rsplit("/", 1)[-1].lower() == "third_party_notices.md":
                sys.stdout.buffer.write(z.read(info) + b"\n")
    sys.stdout.flush()
except BaseException as e:
    sys.stderr.write("zipfile: %s: %r\n" % (sys.argv[1], e))
    sys.exit(1)
' "$zip" 2>/dev/null || return 1
            ;;
        unzip)
            command -v unzip >/dev/null 2>&1 || return 3
            # List the entries and pick the notices by basename — unzip's
            # own wildcards do not reliably cross '/'.
            list="$(unzip -Z1 "$zip" 2>/dev/null)" || return 1
            while IFS= read -r entry; do
                entry="${entry%$'\r'}" # a CRLF-listing unzip (native Win32 builds)
                [ -n "$entry" ] || continue
                # Info-ZIP lists backslash separators verbatim (`a\x.md`);
                # compare the basename on a '/'-normalised copy so such an
                # entry reaches the wildcard refusal below instead of being
                # skipped as "not the notices file".
                norm="${entry//\\//}"
                base="$(printf '%s' "${norm##*/}" | tr '[:upper:]' '[:lower:]')"
                [ "$base" = "third_party_notices.md" ] || continue
                # `unzip -p` treats its name argument as a wildcard pattern:
                # `a[x]/…` would match `ax/…` (or nothing). Not addressable
                # reliably → this reader fails, the next one gets the zip.
                pattern_unsafe "$entry" && return 1
                unzip -p -P '' "$zip" "$entry" 2>/dev/null || return 1
                echo
            done <<< "$list"
            ;;
        tar)
            case "$(tar --version 2>/dev/null)" in
                *[Bb][Ss][Dd][Tt][Aa][Rr]*) ;;
                *) return 3 ;;
            esac
            # Same as the unzip reader: list, pick the notices by
            # case-insensitive basename, read only that entry.
            list="$(tar -tf "$zip" 2>/dev/null)" || return 1
            while IFS= read -r entry; do
                entry="${entry%$'\r'}"
                [ -n "$entry" ] || continue
                base="$(printf '%s' "${entry##*/}" | tr '[:upper:]' '[:lower:]')"
                [ "$base" = "third_party_notices.md" ] || continue
                # bsdtar matches its name arguments as patterns, like unzip.
                pattern_unsafe "$entry" && return 1
                tar -xOf "$zip" -- "$entry" 2>/dev/null || return 1
                echo
            done <<< "$list"
            ;;
        *)
            return 3
            ;;
    esac
}

# zip_read <list|notices> <zip>: the entry names / the notices of zip $2 from
# the first reader that succeeds (with both). Returns 3 when no reader is
# available, 4 when every available reader failed on this zip.
zip_read() {
    local what="$1" zip="$2" readers reader rc out tried=0
    readers="${FRAMETRACE_ZIP_READER:-python unzip tar}"
    for reader in $readers; do
        rc=0
        if [ "$what" = list ]; then
            out="$(list_with "$reader" "$zip")" || rc=$?
        else
            out="$(read_notices_with "$reader" "$zip")" || rc=$?
        fi
        case "$rc" in
            0) printf '%s\n' "$out"; return 0 ;;
            3) ;;
            *) tried=1 ;;
        esac
    done
    if [ "$tried" -eq 1 ]; then
        return 4
    fi
    return 3
}

# The leaf of an entry name: '\' read as '/', trailing '/' dropped, leading
# whitespace and trailing whitespace/dots trimmed, lower case — the
# normalisation of make-portable.ps1's Get-NormalLeaf.
leaf_of() {
    local n="${1//\\//}"
    while [ "${n%/}" != "$n" ]; do n="${n%/}"; done
    n="${n##*/}"
    printf '%s' "$n" | sed -e 's/^[[:space:]]*//' -e 's/[.[:space:]]*$//' | tr '[:upper:]' '[:lower:]'
}
FFMPEG_NAME_RE='ffmpeg|ffprobe|ffplay|av(codec|format|util|device|filter)|swscale|swresample|postproc'
ARCHIVE_NAME_RE='\.(zip|7z|gz|tgz|tar|cab|rar|xz|bz2|zst)$'

# entry_problems <entry names (one per line)>: one problem per bad entry.
entry_problems() {
    local entry leaf
    while IFS= read -r entry; do
        entry="${entry%$'\r'}" # a CRLF listing (native Win32 tools)
        [ -n "$entry" ] || continue
        leaf="$(leaf_of "$entry")"
        [ -n "$leaf" ] || continue
        if [[ "$leaf" =~ $FFMPEG_NAME_RE ]]; then
            echo "FFmpeg entry '$entry' — ffmpeg/ffprobe are never packaged"
        fi
        if [ "$leaf" = unconfirmed.txt ]; then
            echo "UNCONFIRMED.txt entry '$entry' — an unconfirmed test package (make-portable.ps1 -AllowUnconfirmedNotices)"
        fi
        if [[ "$leaf" =~ $ARCHIVE_NAME_RE ]]; then
            echo "nested archive '$entry' — its contents cannot be checked"
        fi
    done <<< "$1"
}

# Whether text holds the "Owner to confirm" marker (re-wrapped across lines
# too): fold whitespace and case first, then match in bash (no `grep -q` +
# pipefail SIGPIPE).
has_marker() {
    local text
    text="$(printf '%s' "$1" | tr '\r\n\t' '   ' | tr '[:upper:]' '[:lower:]')"
    [[ "$text" =~ owner[[:space:]]+to[[:space:]]+confirm ]]
}

# Prints a working `node` (probed with `node -e 0`); fails if none.
find_node() {
    command -v node >/dev/null 2>&1 || return 1
    node -e 0 >/dev/null 2>&1 || return 1
    printf '%s\n' node
}

# Release guard on the working tree (§Fix round 12 M): the tools' notices in
# tools/manifest.json are compiled into EVERY platform's binary
# (include_str!) and `frametrace tools` prints them on Linux/macOS too, and
# this script ships THIRD_PARTY_NOTICES.md in its zips. Same rule as
# scripts/make-portable.ps1: every tool the app can install needs
# "notice_confirmed": true as a JSON boolean (the string "true" does not
# count; keys are exact — "Notice_Confirmed" is another key, as for serde).
# Installable (§Fix round 16, src/tool_install/mod.rs): a "sha256" that is
# present, not null and not "", or "kind": "pip" with a "requirements_file"
# that is present, not null and not "". THIRD_PARTY_NOTICES.md must exist,
# be readable (§Fix round 16: an unreadable file fails closed) and not match
# Owner\s+to\s+confirm. The manifest is parsed with Python's json (same probe
# as the zip reader), else with node's JSON.parse (§Fix round 13 L3: the same
# rule); with neither the check fails closed — there is no override
# (FRAMETRACE_SKIP_NOTICES_CHECK=1 only concerns the zip reader, never this
# check). A reader's own error is printed with the refusal.
# Prints one problem per line.
source_problems() {
    local py node out rc text err
    if [ ! -f THIRD_PARTY_NOTICES.md ]; then
        echo "THIRD_PARTY_NOTICES.md: not found"
    elif [ ! -r THIRD_PARTY_NOTICES.md ]; then
        echo "THIRD_PARTY_NOTICES.md: cannot be read (permissions) — cannot verify"
    elif ! text="$(cat -- THIRD_PARTY_NOTICES.md)"; then
        echo "THIRD_PARTY_NOTICES.md: cannot be read — cannot verify"
    elif has_marker "$text"; then
        echo "THIRD_PARTY_NOTICES.md: still says 'Owner to confirm'"
    fi
    if [ ! -f tools/manifest.json ]; then
        echo "tools/manifest.json: not found — cannot check the tools' distribution notices"
        return 0
    fi
    err="$(mktemp 2>/dev/null || mktemp -t ft-manifest)"
    rc=0
    if py="$(find_python)"; then
        out="$("$py" -I -c '
import json, sys
try:
    with open(sys.argv[1], encoding="utf-8-sig") as f:
        m = json.load(f)
    pending = []
    for t in m["tools"]:
        if not isinstance(t, dict):
            raise ValueError("a tool entry is not an object")
        s = t.get("sha256")
        r = t.get("requirements_file")
        installable = not (s is None or s == "") or (t.get("kind") == "pip" and not (r is None or r == ""))
        if not installable:
            continue  # manual only
        if t.get("notice_confirmed") is not True:
            pending.append(str(t.get("name")))
    print(", ".join(pending))
except BaseException as e:
    sys.stderr.write("python: %r\n" % (e,))
    sys.exit(1)
' tools/manifest.json 2>"$err")" || rc=$?
    elif node="$(find_node)"; then
        out="$("$node" -e '
const fs = require("fs");
const own = (o, k) => Object.prototype.hasOwnProperty.call(o, k);
const given = (o, k) => own(o, k) && o[k] !== null && o[k] !== "";
try {
  let text = fs.readFileSync(process.argv[1], "utf8");
  if (text.charCodeAt(0) === 0xfeff) text = text.slice(1);
  const m = JSON.parse(text);
  if (!m || typeof m !== "object" || !own(m, "tools") || !Array.isArray(m.tools)) throw new Error("no \"tools\" array");
  const pending = [];
  for (const t of m.tools) {
    if (!t || typeof t !== "object" || Array.isArray(t)) throw new Error("a tool entry is not an object");
    const installable = given(t, "sha256") || (own(t, "kind") && t.kind === "pip" && given(t, "requirements_file"));
    if (!installable) continue; // manual only
    if (!own(t, "notice_confirmed") || t.notice_confirmed !== true) pending.push(own(t, "name") && t.name !== null ? String(t.name) : "None");
  }
  console.log(pending.join(", "));
} catch (e) {
  process.stderr.write("node: " + (e && e.message ? e.message : String(e)) + "\n");
  process.exit(1);
}
' tools/manifest.json 2>"$err")" || rc=$?
    else
        rm -f "$err"
        echo "tools/manifest.json: cannot verify — no python3/python or node to parse it (install one; FRAMETRACE_SKIP_NOTICES_CHECK does not skip this check)"
        return 0
    fi
    out="$(printf '%s' "$out" | tr -d '\r')"
    if [ "$rc" -ne 0 ]; then
        echo "tools/manifest.json: cannot verify — not readable as a tool manifest ($(tr -d '\r' < "$err" | tr '\n' ' ' | sed 's/ *$//'))"
    elif [ -n "$out" ]; then
        echo "tools/manifest.json: notice_confirmed is not true for: $out"
    fi
    rm -f "$err"
}

# Exact paths of the zips this run built (M3).
PRODUCED=()

# Prints every dist/*.zip (any case; hidden ones too) that is not in PRODUCED.
stray_zips() {
    local zip lower p mine
    [ -d dist ] || return 0
    for zip in dist/* dist/.[!.]*; do
        [ -f "$zip" ] || continue
        lower="$(printf '%s' "${zip##*/}" | tr '[:upper:]' '[:lower:]')"
        case "$lower" in
            *.zip) ;;
            *) continue ;;
        esac
        mine=0
        for p in "${PRODUCED[@]+"${PRODUCED[@]}"}"; do
            [ "$p" = "$zip" ] && mine=1
        done
        [ "$mine" -eq 1 ] && continue
        case "$lower" in
            *unconfirmed*) echo "$zip (an UNCONFIRMED test package by its name)" ;;
            *) echo "$zip" ;;
        esac
    done
}

# Refuses (exit 1) while dist/ holds a zip this run did not build.
refuse_stray_zips() {
    local stray
    stray="$(stray_zips)"
    [ -n "$stray" ] || return 0
    {
        echo "error: dist/ holds zips this run did not build — NOT FOR RELEASE:"
        printf '%s\n' "$stray" | sed 's/^/  /'
        echo "only the zips a run builds are listed in SHA256SUMS and signed: clean dist/ (move or delete every *.zip there, and SHA256SUMS.unconfirmed) and run again; no SHA256SUMS was written and nothing was signed."
    } >&2
    exit 1
}

# produced_problems <zip> <staged dir>: the content problems of a zip this
# run built (see the header), one per line.
produced_problems() {
    local zip="$1" stage="$2" rc entries notices f
    rc=0
    entries="$(zip_read list "$zip")" || rc=$?
    if [ "$rc" -eq 3 ] && [ "${FRAMETRACE_SKIP_NOTICES_CHECK:-}" = 1 ]; then
        echo "warning: no zip reader (python3/python with zipfile, unzip, bsdtar) and FRAMETRACE_SKIP_NOTICES_CHECK=1 — $zip checked through its staged directory $stage instead of its entries" >&2
        entries="$(cd "$(dirname "$stage")" && find "$(basename "$stage")" -mindepth 1)"
        entry_problems "$entries"
        while IFS= read -r f; do
            [ -n "$f" ] || continue
            [ "$(leaf_of "$f")" = third_party_notices.md ] || continue
            if [ ! -r "$(dirname "$stage")/$f" ] || ! notices="$(cat -- "$(dirname "$stage")/$f")"; then
                echo "$f: cannot be read — cannot verify"
            elif has_marker "$notices"; then
                echo "THIRD_PARTY_NOTICES.md: Owner to confirm ('$f')"
            fi
        done <<< "$entries"
        return 0
    elif [ "$rc" -eq 3 ]; then
        echo "cannot verify — no zip reader (python3/python with zipfile, unzip, bsdtar); install one, or set FRAMETRACE_SKIP_NOTICES_CHECK=1 to check the staged directory instead"
        return 0
    elif [ "$rc" -ne 0 ]; then
        echo "cannot verify — every zip reader failed on it"
        return 0
    fi
    entry_problems "$entries"
    rc=0
    notices="$(zip_read notices "$zip")" || rc=$?
    if [ "$rc" -ne 0 ]; then
        echo "THIRD_PARTY_NOTICES.md: cannot verify — every zip reader failed on it"
    elif has_marker "$notices"; then
        echo "THIRD_PARTY_NOTICES.md: Owner to confirm"
    fi
}

# --- before the build ---------------------------------------------------------
SOURCE="$(source_problems)"
if [ -n "$SOURCE" ]; then
    {
        echo "error: the working tree is not releasable — NOT FOR RELEASE:"
        printf '%s\n' "$SOURCE" | sed 's/^/  /'
        echo "the owner must confirm the notices (\"notice_confirmed\": true in tools/manifest.json, the '(Owner to confirm ...)' marker removed from THIRD_PARTY_NOTICES.md); nothing was built."
    } >&2
fi
if [ -n "$(stray_zips)" ]; then
    refuse_stray_zips
fi
[ -z "$SOURCE" ] || exit 1
# Nothing an earlier run wrote may look like this run's manifest.
rm -f dist/SHA256SUMS dist/SHA256SUMS.sigstore.json

# Pinned toolchain + locked dependencies: the two inputs that decide the
# build (see docs/repro-build.md). §Fix round 16 M4: the binaries are taken
# from the compiler-artifact messages (`executable`), so CARGO_TARGET_DIR,
# CARGO_BUILD_TARGET_DIR, .cargo/config.toml build.target-dir and
# --target layouts are all honoured — never a stale target/release.
mkdir -p dist
BUILD_JSON="$(mktemp 2>/dev/null || mktemp -t ft-build-json)"
trap 'rm -f "$BUILD_JSON"' EXIT
cargo build --release --locked --message-format=json-render-diagnostics > "$BUILD_JSON"

# Prints "<bin>\t<executable>" for frametrace and frametrace-app (empty when
# cargo reported none).
artifact_paths() {
    local py node
    if py="$(find_python)"; then
        "$py" -I -c '
import json, sys
found = {}
with open(sys.argv[1], encoding="utf-8", errors="replace") as f:
    for line in f:
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            m = json.loads(line)
        except ValueError:
            continue
        if not isinstance(m, dict) or m.get("reason") != "compiler-artifact":
            continue
        t = m.get("target") or {}
        if t.get("name") in ("frametrace", "frametrace-app") and "bin" in (t.get("kind") or []) and m.get("executable"):
            found[t["name"]] = m["executable"]
for name in ("frametrace", "frametrace-app"):
    print("%s\t%s" % (name, found.get(name, "")))
' "$BUILD_JSON"
    elif node="$(find_node)"; then
        "$node" -e '
const found = {};
for (const raw of require("fs").readFileSync(process.argv[1], "utf8").split("\n")) {
  const line = raw.trim();
  if (!line.startsWith("{")) continue;
  let m; try { m = JSON.parse(line); } catch { continue; }
  if (!m || m.reason !== "compiler-artifact") continue;
  const t = m.target || {};
  if ((t.name === "frametrace" || t.name === "frametrace-app") && Array.isArray(t.kind) && t.kind.includes("bin") && m.executable) found[t.name] = m.executable;
}
for (const n of ["frametrace", "frametrace-app"]) console.log(n + "\t" + (found[n] || ""));
' "$BUILD_JSON"
    else
        echo "error: no python3/python or node to read cargo's build messages" >&2
        return 1
    fi
}
PATHS="$(artifact_paths | tr -d '\r')"
BIN_CLI=""
BIN_APP=""
while IFS=$'\t' read -r bin path; do
    case "$bin" in
        frametrace) BIN_CLI="$path" ;;
        frametrace-app) BIN_APP="$path" ;;
    esac
done <<< "$PATHS"
if [ -z "$BIN_CLI" ]; then
    echo "error: cargo build reported no executable for the frametrace binary (--message-format=json-render-diagnostics); nothing was packaged" >&2
    exit 1
fi
for bin in "$BIN_CLI" ${BIN_APP:+"$BIN_APP"}; do
    if [ ! -f "$bin" ]; then
        echo "error: cargo reported $bin but it does not exist; nothing was packaged" >&2
        exit 1
    fi
done

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
TARGET="$(rustc -vV | sed -n 's/^host: //p')"
ARTIFACT="frametrace-${VERSION}-${TARGET}"
STAGE="dist/${ARTIFACT}"

rm -rf "$STAGE"
mkdir -p "$STAGE"
cp "$BIN_CLI" "$STAGE/"
if [ -n "$BIN_APP" ]; then
    cp "$BIN_APP" "$STAGE/"
fi
cp README.md THIRD_PARTY_NOTICES.md LICENSE "$STAGE/"

rm -f "dist/${ARTIFACT}.zip"
(cd dist && zip -qr "${ARTIFACT}.zip" "$ARTIFACT")
PRODUCED+=("dist/${ARTIFACT}.zip")

# --- before SHA256SUMS: nothing else in dist/, and the built zip is clean -------
refuse_stray_zips
FOUND=""
for zip in "${PRODUCED[@]}"; do
    problems="$(produced_problems "$zip" "$STAGE")"
    if [ -n "$problems" ]; then
        FOUND="${FOUND}$(printf '%s\n' "$problems" | sed "s|^|${zip}: |")"$'\n'
    fi
done
if [ -n "$FOUND" ]; then
    rm -f "${PRODUCED[@]}"
    {
        echo "error: the zip this run built is not releasable — NOT FOR RELEASE:"
        printf '%s' "$FOUND" | sed 's/^/  /'
        echo "the zip was deleted; no SHA256SUMS was written and nothing was signed."
    } >&2
    exit 1
fi

# SHA256SUMS lists exactly the zips this run built (by path), never anything
# else that sits in dist/.
SUM_NAMES=()
for zip in "${PRODUCED[@]}"; do
    SUM_NAMES+=("${zip#dist/}")
done
(cd dist && shasum -a 256 -- "${SUM_NAMES[@]}") > dist/SHA256SUMS

if [ "$SBOM" -eq 1 ]; then
    command -v cargo-sbom >/dev/null 2>&1 || {
        echo "cargo-sbom not found; install with: cargo install cargo-sbom --locked" >&2
        exit 1
    }
    cargo sbom > "dist/${ARTIFACT}.sbom.cyclonedx.json"
fi

if [ "$SIGN" -eq 1 ]; then
    command -v cosign >/dev/null 2>&1 || {
        echo "cosign not found; see docs/repro-build.md#signing" >&2
        exit 1
    }
    if [ -n "${COSIGN_KEY:-}" ]; then
        # Local-key mode (offline): COSIGN_KEY points at a cosign private key,
        # COSIGN_PASSWORD unlocks it. Uses a signing config without a
        # transparency-log service so no Rekor upload is attempted; the bundle
        # still verifies offline via `cosign verify-blob --insecure-ignore-tlog`.
        CONFIG="${COSIGN_SIGNING_CONFIG:-}"
        if [ -z "$CONFIG" ]; then
            CONFIG="$(mktemp -t ft-signing-config).json"
            curl -fsSL https://raw.githubusercontent.com/sigstore/root-signing/refs/heads/main/targets/signing_config.v0.2.json \
                | python3 -c 'import json,sys; c=json.load(sys.stdin); c.pop("rekorTlogUrls",None); c.pop("rekorTlogConfig",None); json.dump(c,sys.stdout)' \
                > "$CONFIG"
        fi
        for zip in "${PRODUCED[@]}"; do
            cosign sign-blob --yes --key "$COSIGN_KEY" --signing-config "$CONFIG" \
                --bundle "${zip}.sigstore.json" "$zip"
        done
        cosign sign-blob --yes --key "$COSIGN_KEY" --signing-config "$CONFIG" \
            --bundle "dist/SHA256SUMS.sigstore.json" \
            "dist/SHA256SUMS"
    else
        # Keyless mode: OIDC identity + Rekor transparency log (needs network).
        for zip in "${PRODUCED[@]}"; do
            cosign sign-blob --yes --bundle "${zip}.sigstore.json" "$zip"
        done
        cosign sign-blob --yes \
            --bundle "dist/SHA256SUMS.sigstore.json" \
            "dist/SHA256SUMS"
    fi
fi

echo "package: dist/${ARTIFACT}.zip"
echo "sums:    dist/SHA256SUMS"
[ "$SBOM" -eq 1 ] && echo "sbom:    dist/${ARTIFACT}.sbom.cyclonedx.json"
[ "$SIGN" -eq 1 ] && echo "sigs:    dist/${ARTIFACT}.zip.sigstore.json, dist/SHA256SUMS.sigstore.json"
exit 0
