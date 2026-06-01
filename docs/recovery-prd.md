# Recovery PRD

Status: approved for the current FrameTrace production-readiness slice.

## Objective

Provide report-defensible video recovery workflows for local workstation cases without modifying original evidence. Required recovery outputs must be indexed, hashable, auditable, viewable, and separately validated before final reporting.

## Scope

In scope:

- E01/Ex01/S01/L01 inspection and raw export through libewf tools.
- Raw forensic-image filesystem inspection through Sleuth Kit `mmls` and `fls`.
- Inode-level recovery through Sleuth Kit `icat`.
- Signature-based contiguous video carving for common video/DVR containers.
- Recovered-artifact hashing and chained JSONL audit records.
- Recovery visibility in case report and evidence viewer.
- QA checks for accuracy, reproducibility, report defensibility, and SQLite scale.

Out of scope until a feature-specific PRD and corpus exist:

- Browser artifact parsing.
- Windows Event Log parsing.
- Proprietary DVR database decoding beyond file/video recovery.
- Claims that a recovery is complete when the source filesystem metadata is absent or unvalidated.

## Functional Requirements

1. The tool must never write recovered outputs over existing files.
2. Explicit recovery/export output paths must remain inside the case directory.
3. User-configurable external tools must be limited to the intended tool family.
4. Every recovered output must include size, SHA-256, source path/image path, recovery method, timestamp, and validation status.
5. Recovery outputs must remain `candidate-unvalidated` until validation confirms a playable video stream or an examiner records a manual/vendor-player validation.
6. Evidence viewer must show indexed videos, carving candidates, and filesystem recovery outputs in one searchable review surface.
7. Report output must distinguish original evidence references from derived clips, proxies, thumbnails, carved files, and recovered inodes.

## Acceptance Criteria

- `cargo test` passes with recovery, package, migration, QA, and CLI smoke coverage.
- `qa accuracy` passes against a ground-truth TSV manifest with precision and recall >= 0.98.
- `qa reproducibility` passes for deterministic reruns against the same corpus.
- `qa report-defense` passes after `init-case`, `scan-folder`, `make-report`.
- `qa performance` passes at the configured row target.
- Recovered artifacts are visible in `review/evidence-viewer.html` and `reports/case-report.html`.

## Release Blockers

- Any recovery output path escaping the case directory.
- Any unapproved external binary execution path.
- Any missing SHA-256 for a produced recovered artifact.
- Any required corpus false negative marked P0.
- Any migration failure without a generated rollback backup.
- Any SQLite job still marked `running` during report-defense or release-readiness review.
