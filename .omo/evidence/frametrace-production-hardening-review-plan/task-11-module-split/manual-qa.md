# T11 Manual QA

## Happy Path CLI Smoke

Surface: real local CLI binary `target/debug/frametrace`.

Before and after commands:

```bash
target/debug/frametrace init-case <tmp>/case --title T11-... --operator qa-t11
target/debug/frametrace scan-folder <tmp>/case <tmp>/source --no-ffprobe
target/debug/frametrace make-report <tmp>/case
target/debug/frametrace make-review <tmp>/case
```

Artifacts:

- `baseline-cli-smoke-transcript.txt`
- `post-cli-smoke-transcript.txt`
- `baseline-cli-smoke-contract.json`
- `post-cli-smoke-contract.json`
- `behavior-snapshot-diff.txt`

Pass criterion: commands exit 0 and normalized contract fields match exactly. Result: PASS; `behavior-snapshot-diff.txt` reports no normalized CLI smoke contract drift.

## Failure / Drift Rejection

Surface: normalized before/after contract diff.

Command:

```bash
diff -u baseline-cli-smoke-contract.json post-cli-smoke-contract.json
```

Pass criterion: diff exit 0. Result: PASS; no unexpected CLI/report/JSON contract drift found.
