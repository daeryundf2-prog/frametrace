# Security Review

Phase 2 review focused on local file handling, external command boundaries, report/viewer serialization, privacy leakage, and packaging.

## Findings

| Severity | Finding | Status | Owner |
| --- | --- | --- | --- |
| High | User-configurable external binary names can execute arbitrary binaries through `Command::new`. | Fixed for ffprobe, ffmpeg, libewf, and Sleuth Kit user-configurable binaries. | Security Owner |
| High | Bare tool names resolved by `Command::new` let the Windows loader search the current directory (binary planting from evidence media). | Fixed: bare names are resolved manually against PATH only, returning canonical paths or failing. | Security Owner |
| High | Output paths can be directed outside the case workspace for some export/proxy/package/recovery operations. | Fixed for E01 raw export, video export, proxy, thumbnail, inode recovery, marks export, and recursive package traversal. | Security Owner |
| High | Reports and viewer payloads expose full source paths by default. | Mitigated: `make-report`/`make-review --redact-paths` rewrites absolute paths as `<case>/`-relative or `<redacted:hash>` tokens and marks the output; default output is still unredacted. | Security Owner |
| Medium | Generated HTML/JS serialization is manual and should move toward typed JSON serialization. | Pending | Engineering Lead |
| Medium | Selector-to-path resolution may trust poisoned logs or free-form paths. | Pending | Security Owner |
| Medium | Recursive packaging could follow symlinked inputs outside the intended tree. | Fixed for package inputs. | Engineering Lead |
| Medium | Manual JSON-like parsing increases malformed input risk. | Partially mitigated for `ffprobe`; broader migration pending. | Engineering Lead |

## Implemented Security Fixes

1. `ffmpeg` invocations (export, proxy, thumbnail, version logging) now go through the same tool-policy allowlist as ffprobe/libewf/Sleuth Kit, and the unvalidated `audit::command_version` helper was removed.
2. Bare tool names are resolved manually against PATH (current directory excluded), removing the Windows binary-planting window for planted executables.
3. Recursive package inputs now reject symlinks instead of following them.
4. Required package files are validated before package generation.
5. Invalid `ffprobe` JSON output now fails closed instead of corrupting JSON index output.
6. Scan now rejects using the case directory as the source and skips nested case output directories.
7. `src/tool_policy.rs` allowlists external tool binaries and rejects unapproved bare names.
8. Explicit derived-output paths must resolve under the case directory for recovery/export artifacts — now including `export-marks --output`.
9. The workstation Origin gate now parses the `http://` authority host exactly (`127.0.0.1`, `localhost`, `[::1]`, optional `:port`), so suffix/userinfo spoofs such as `http://127.0.0.1.evil.com` no longer pass a prefix check.
10. Parallel batch workers (validate-batch, thumbnail generation) recover poisoned mutexes via `into_inner()` instead of cascading one worker panic into a whole-batch abort.

## Workstation Loopback Threat Model

The examiner workstation (running `frametrace` with no arguments, `src/serve.rs`) binds `127.0.0.1` only and ships **no auth token by design choice**: it is a local-only tool, and any process already running as the examiner holds equivalent privileges over the case directory.

Consequences and the controls that exist:

- **Any local process can invoke every API endpoint.** That includes `POST /api/open-folder`, which spawns the OS file explorer (`explorer.exe`/`xdg-open`) on an arbitrary path from the request body. This is an accepted trade-off for single-user local use.
- **Browser-originated requests are gated.** Every route (including `/media`) requires a loopback `Host` header (the DNS-rebinding gate) and, when present, an `Origin` whose authority parses to an exact loopback host (the cross-site form-POST gate). Requests with **no** `Origin` — curl, local scripts, the page the workstation itself opened — are trusted, because a local caller needs no browser to reach the port.
- **Recommended mitigations for examiners:** run the workstation on a single-user analysis machine, avoid browsing untrusted sites in the same browser session while a case is loaded, and close the window (which stops the server) when review is done. Multi-user or remote access would require an auth token and a changed bind policy — loopback trust does not extend to other interfaces.

## Remaining Security Work

1. Report privacy/redaction mode exists as an opt-in flag (`--redact-paths`); decide whether redaction becomes the default before distributable report release.
2. Harden selector-to-path resolution against poisoned logs.
3. Replace manual JSON extraction with typed parsing where feasible.
4. Add release-time privacy leakage QA once redaction policy is approved.

## Validation

- Symlink package regression test added.
- Missing required package regression test added.
- Scan exclusion regression test added.
- `ffprobe` JSON structural helper test added.
- Tool binary allowlist regression tests added.
- Case-contained output path regression tests added (including the `export-marks` sibling-prefix rejection).
- Origin authority exact-match regression test added.
- Mutex poison recovery regression test added.
- CLI smoke test keeps missing libewf guidance while enforcing the allowlist.
