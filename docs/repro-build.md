# Reproducible Build & Release Procedure

This document is the release checklist for FrameTrace binaries: how the
released artifact is built, how its integrity is anchored (SHA256SUMS +
optional signature), how to emit an SBOM, and how to build from a vendored
source tree. It is procedure, not new infrastructure — the only code added
is `scripts/build-release.sh` (Unix) and the SHA256SUMS step in
`scripts/make-portable.ps1` (Windows).

## Inputs that decide the build

| Input | Where pinned |
| --- | --- |
| Rust toolchain | `rust-toolchain.toml` — channel `1.94.0`, components `clippy`, `rustfmt` |
| Dependency graph | `Cargo.lock` — every build command below passes `--locked` |
| Compiler/target | `rustc -vV` host triple, baked into the artifact name |

Rebuilding a release means: same pinned toolchain, same lockfile, same
target. FrameTrace does **not** currently assert bit-for-bit
reproducibility across machines or toolchain builds — the published
SHA-256 manifest plus a signature is the integrity anchor, not a
rebuild-everywhere guarantee. If byte-identical rebuilds become a
requirement, the first gaps to close are documented below under
"Caveats".

## Building a release

```sh
scripts/build-release.sh            # zip + SHA256SUMS
scripts/build-release.sh --sbom     # + CycloneDX SBOM (needs cargo-sbom)
scripts/build-release.sh --sign     # + cosign sign-blob bundles
```

produces:

```
dist/frametrace-<version>-<target>.zip
dist/SHA256SUMS                     # lists exactly the zip this run built
```

Each zip holds `frametrace`, `frametrace-app`, `README.md`,
`THIRD_PARTY_NOTICES.md` and `LICENSE`. The binaries are the ones cargo
reports in its `compiler-artifact` messages
(`cargo build --release --locked --message-format=json-render-diagnostics`,
the `executable` of the `frametrace` / `frametrace-app` bins), so
`CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET_DIR`, `.cargo/config.toml`
`build.target-dir` and `--target` layouts are honoured and a stale
`target/release` binary is never packaged (§Fix round 16 M4).

Before anything else (cargo is not run) `build-release.sh` checks the
working tree, with the same rule as `make-portable.ps1`: every tool the app
can install — an archive tool with a `sha256` that is present, not `null`
and not `""`, or a `"kind": "pip"` tool with such a `requirements_file`
(`src/tool_install/mod.rs`; manual-only tools are exempt) — must carry
`"notice_confirmed": true` as a JSON boolean under that exact key (the
string `"true"`, `1`, a missing key or a wrong-case `"Notice_Confirmed"` are
refused), because that manifest and its notices are compiled into every
platform's binary and `frametrace tools` prints them on Linux/macOS too; and
`THIRD_PARTY_NOTICES.md` must exist, be readable (a file that exists but
cannot be read fails closed) and must not match `Owner\s+to\s+confirm` (it
ships in the zip). Otherwise it exits 1 with "the working tree is not
releasable — NOT FOR RELEASE" and names each problem. The manifest is parsed
with Python's `json` (`python3`, else `python`, run with `-I`), else with
`node -e` (`JSON.parse`, the same rule); with neither the check fails closed
("cannot verify") and there is **no override** —
`FRAMETRACE_SKIP_NOTICES_CHECK=1` never affects the manifest or the
working-tree THIRD_PARTY_NOTICES.md check, only a missing zip reader for the
zip content check below. A reader that fails on the manifest is a refusal
too, and its own error is printed with it. Unlike `make-portable.ps1` there
is no switch for an unconfirmed test package: Unix zips are release zips
only.

`dist/` must hold no other `*.zip` (any case, hidden ones included): SHA256SUMS
lists — and `--sign` signs — only the zips the run built, tracked by exact
path, so the script refuses (exit 1, before building and again before
writing SHA256SUMS) while any other zip is there — an UNCONFIRMED test
package from `make-portable.ps1 -AllowUnconfirmedNotices` (whatever it is
called), an old `*-win64-bundled.zip` with FFmpeg, an outer zip wrapping
one, or a zip of another target. Clean `dist/` (move or delete every
`*.zip`, and `SHA256SUMS.unconfirmed`) and run again; one release run per
target. A `SHA256SUMS` from an earlier run is removed before the build.

The zip the run built is then checked by content before SHA256SUMS is
written, and deleted when it fails: every entry against the FFmpeg name rule
of `make-portable.ps1` (leaf, trimmed of spaces and trailing dots, any case,
containing ffmpeg/ffprobe/ffplay or an FFmpeg library name), no
`UNCONFIRMED.txt` in any case or folder (the marker `make-portable.ps1` puts
into an unconfirmed package), no nested archive (`*.zip`, `*.7z`, `*.gz`,
`*.tgz`, `*.tar`, `*.cab`, `*.rar`, `*.xz`, `*.bz2`, `*.zst`), and no
`THIRD_PARTY_NOTICES.md` (any case, any folder) matching
`Owner\s+to\s+confirm`. The zip is read with Python's `zipfile` (`python3`,
else `python`, each probed with `import zipfile` so the Microsoft Store
alias stub is skipped), else `unzip -Z1`/`-p`, else bsdtar `-tf`/`-xOf`; the
check fails closed: a reader that exits non-zero on the zip hands it to the
next one, and a zip that no available reader can read is refused as "cannot
verify", never passed as clean. With no reader at all the run also stops
(exit 1); `FRAMETRACE_SKIP_NOTICES_CHECK=1` lets it run the same checks on
the staged directory the zip was made from instead, with a warning (it
affects nothing else). Fixture test (stub cargo/rustc/zip, no build):
`bash scripts/test-build-release.sh` (also run by the Linux CI job
`release-scripts`; the bsdtar cases run when `tar` is bsdtar or with
`FRAMETRACE_TEST_BSDTAR=<path>`, e.g. `C:/Windows/System32/tar.exe`; the
summary prints `SKIPPED n` when cases were skipped).

On Windows, `scripts/make-portable.ps1` stages the portable layout and
writes `dist/SHA256SUMS` with the same `sha256sum -c`-compatible format —
listing only the zip that run wrote (it fails when another zip in `dist/`
contains FFmpeg). The package bundles **no** third-party tool (§Fix round 16
M2): the run fails while the repository's `tools/bin` or the release
directory's `tools/bin` holds any file, and every staged file is refused
when it is an FFmpeg file (by name, PE version information or signature
strings) or an archive / packed executable (by extension or magic bytes:
zip, gzip, 7z, cab, rar, xz, tar, UPX). Its binaries come from cargo's
real target directory (build messages, or `cargo metadata`
`target_directory` with `-SkipBuild`), and both exes must embed this working
tree's `tools/manifest.json` — the manifest the notice guard checks is the
one compiled in. It refuses to package while `THIRD_PARTY_NOTICES.md` still
contains "Owner to confirm" or an installable tool's notice is unconfirmed
(same rule as above); `-AllowUnconfirmedNotices` overrides that for CI test
packages only, never for a release: such a package is named
`FrameTrace-<ver>-win64-UNCONFIRMED.zip`, carries `UNCONFIRMED.txt` at its
root (what is pending; it survives renaming the zip), gets
`dist/SHA256SUMS.unconfirmed` instead of `dist/SHA256SUMS` (a stale
`SHA256SUMS` is removed), and the CI artifact is
`FrameTrace-win64-unconfirmed`. Fixture test: `node scripts/test-make-portable.mjs`
(Windows CI).

## Verifying a release

```sh
cd dist
sha256sum -c SHA256SUMS             # or: shasum -a 256 -c SHA256SUMS
```

With a cosign bundle produced by `--sign`:

```sh
cosign verify-blob --bundle frametrace-<ver>-<target>.zip.sigstore.json \
    --certificate-identity <release-identity> \
    --certificate-oidc-issuer https://token.actions.githubusercontent.com \
    frametrace-<ver>-<target>.zip
```

For releases cut on GitHub Actions, prefer artifact attestation over raw
cosign: add `actions/attest-build-provenance@v2` to the release workflow
and consumers verify with `gh attestation verify frametrace-<ver>-<target>.zip
--repo <org>/<repo>`. Both options are documented alternatives; neither is
required to produce the zip + SHA256SUMS baseline.

## SBOM

`cargo-sbom` is the reasonable choice: maintained, emits CycloneDX JSON,
walks `Cargo.lock` so it documents exactly what `--locked` built:

```sh
cargo install cargo-sbom --locked
cargo sbom > dist/frametrace-<ver>-<target>.sbom.cyclonedx.json
```

The `--sbom` flag of `build-release.sh` does exactly this. A lighter
fallback when cargo-sbom cannot be installed: `cargo tree --locked
--format "{p}"` enumerates the resolved dependency graph — less
structured than CycloneDX, but sufficient for an examiner to diff two
builds' inputs.

## Vendored (offline) build

For air-gapped or archive-preservation builds:

```sh
cargo vendor --locked vendor/ > vendor-config.toml
# append vendor-config.toml to .cargo/config.toml, then:
cargo build --release --locked --offline
```

Commit or archive `vendor/` + `Cargo.lock` + `rust-toolchain.toml`
alongside the release record; that triplet is the complete build input.

## Publication practice

1. Cut the release from a clean `main` checkout (`git status` empty).
2. Run `scripts/build-release.sh --sbom --sign` per platform (each from a clean `dist/`) (or the
   `.ps1` equivalent on Windows).
3. Publish the zips, `SHA256SUMS`, the `.sigstore.json` bundles (or the
   GitHub attestation), and the SBOMs together — the manifest is only
   trustworthy when its signature travels with it.
4. Record the expected binary SHA-256 in the release notes themselves so
   a consumer who never fetches dist/ can still compare a hash quoted in
   release text against a downloaded binary.

## Caveats

- **Not bit-for-bit reproducible yet.** `rustc` embeds path-dependent
  and occasionally nondeterministic metadata; same toolchain + lockfile
  on the same target *should* converge, but nobody has run the two-machine
  comparison. `qa consistency` (in-repo) is the pragmatic check today:
  same inputs → same outputs at the *product* level.
- **SBOM covers the Rust crate graph only.** Runtime external tools
  (ffmpeg, libewf, Sleuth Kit) are out-of-process and belong in the
  examiner's environment manifest, not the crate SBOM.
- **SHA256SUMS is self-asserted.** It anchors integrity only when the
  signature/attestation over it is verified; an unsigned manifest proves
  nothing about who produced it.

The Windows zip contains no FFmpeg; it is installed by the user from the pinned BtbN release (tools/manifest.json).
