# Security Review

Phase 2 review focused on local file handling, external command boundaries, report/viewer serialization, privacy leakage, and packaging.

## Findings

| Severity | Finding | Status | Owner |
| --- | --- | --- | --- |
| High | User-configurable external binary names can execute arbitrary binaries through `Command::new`. | Fixed for ffprobe, libewf, and Sleuth Kit user-configurable binaries. | Security Owner |
| High | Output paths can be directed outside the case workspace for some export/proxy/package/recovery operations. | Fixed for E01 raw export, video export, proxy, thumbnail, inode recovery, and recursive package traversal. | Security Owner |
| High | Reports and viewer payloads expose full source paths by default. | Pending | Security Owner |
| Medium | Generated HTML/JS serialization is manual and should move toward typed JSON serialization. | Pending | Engineering Lead |
| Medium | Selector-to-path resolution may trust poisoned logs or free-form paths. | Pending | Security Owner |
| Medium | Recursive packaging could follow symlinked inputs outside the intended tree. | Fixed for package inputs. | Engineering Lead |
| Medium | Manual JSON-like parsing increases malformed input risk. | Partially mitigated for `ffprobe`; broader migration pending. | Engineering Lead |

## Implemented Security Fixes

1. Recursive package inputs now reject symlinks instead of following them.
2. Required package files are validated before package generation.
3. Invalid `ffprobe` JSON output now fails closed instead of corrupting JSON index output.
4. Scan now rejects using the case directory as the source and skips nested case output directories.
5. `src/tool_policy.rs` allowlists external tool binaries and rejects unapproved bare names.
6. Explicit derived-output paths must resolve under the case directory for recovery/export artifacts.

## Remaining Security Work

1. Add report privacy/redaction mode before distributable report release.
2. Harden selector-to-path resolution against poisoned logs.
3. Replace manual JSON extraction with typed parsing where feasible.
4. Add release-time privacy leakage QA once redaction policy is approved.

## Validation

- Symlink package regression test added.
- Missing required package regression test added.
- Scan exclusion regression test added.
- `ffprobe` JSON structural helper test added.
- Tool binary allowlist regression tests added.
- Case-contained output path regression tests added.
- CLI smoke test keeps missing libewf guidance while enforcing the allowlist.
