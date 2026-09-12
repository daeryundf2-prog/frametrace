# Third-Party Validation Plan

A CFTT-style (NIST Computer Forensic Tool Testing, federated method)
protocol for independently validating FrameTrace's recovery, carving, and
indexing claims against reference tools. The goal is not to prove the tool
is "correct" in the abstract — it is to record, per test case, what was
asserted, what was measured, and where the tool's output agrees or
disagrees with independent reference implementations on identical input.

## 1. Method

Federated test cases follow the CFTT shape: each assertion lists
**setup → action → expected result → measured result → verdict**. A test
case is one disk image plus one assertion table. The validator (the party
running this plan) need not trust the producer's claims — every measured
result is reproduced by running the shipped binary, and every reference
result is produced by a tool the producer does not control.

Ground-truth hierarchy when tools disagree:

1. **Known-planted data wins.** On synthetic corpora the planted file's
   pre-deletion SHA-256 and offsets are the truth; a tool that recovers a
   byte-identical payload is right regardless of what another tool found.
2. **The Sleuth Kit `fls`/`icat` are the filesystem truth** on
   field-acquired images (deleted-entry enumeration, inode→bytes).
3. **ffprobe/ffmpeg decode is the playability truth** for any carved or
   recovered candidate — a "recovered video" that no decoder opens is a
   candidate, and FrameTrace's `candidate-*`/`validation-failed` labels
   are the claim being tested, not the file itself.
4. **Unresolvable disagreement is recorded, not resolved by vote.** Both
   outputs go in the receipt; adjudication notes explain the divergence.

## 2. Tool under test

- Build per `docs/repro-build.md`: pinned toolchain
  (`rust-toolchain.toml`), `cargo build --release --locked`, packaged by
  `scripts/build-release.sh` into `frametrace-<ver>-<target>.zip` +
  `SHA256SUMS`. The receipt MUST quote the artifact's SHA-256 so a later
  reader can prove they tested the same bytes.
- Gates that must pass before comparison testing starts:
  `cargo fmt --check`, `cargo clippy --locked --all-targets --
  -D warnings`, `cargo test --locked` (the receipt's gate table mirrors
  `docs/MACOS_FULL_RUN_RECEIPT.md`).

## 3. Reference tools

| Reference | Role in comparison |
| --- | --- |
| The Sleuth Kit (`fls`, `icat`, `mmls`, `blkls`) | Filesystem ground truth: deleted entries, inode byte extraction, partition map |
| Autopsy (ingest modules, file carving/keyword search) | GUI-tool cross-check of deleted-file enumeration and carve hit counts |
| X-Ways Forensics (evaluation copy acceptable) | Commercial cross-check of carved file headers and recovered-file integrity |
| ffmpeg/ffprobe | Decodability ground truth for every recovered/carved candidate |

All tools run against **the same byte image** — never re-acquired per
tool, or acquisition variance masquerades as tool variance.

## 4. Test assertions

Per test case (image + corpus), assert and measure:

| # | Assertion | Reference | Pass measure |
| --- | --- | --- | --- |
| A1 | Deleted-file enumeration: `inspect-image` reports at least the deleted entries `fls -d` reports on the same image | TSK `fls` | FrameTrace set ⊇ TSK set; extra entries must be labelled candidates, never asserted as live files |
| A2 | Inode recovery: `recover-inode` output is byte-identical to `icat` output (or to the planted file's recorded SHA-256) | TSK `icat` / planted hash | SHA-256 equality |
| A3 | Signature carving: `carve-file` finds every planted signature offset and reports each as a `candidate-*` artifact | planted manifest | Every planted offset present in `db/carve_results.json`; no claimed-strictly-more assertions — candidates are labelled candidates |
| A4 | Carved-content fidelity: each carved artifact's SHA-256 equals the planted payload hash | planted manifest | SHA-256 equality where the format defines a deterministic span; otherwise offset + signature agreement and a note |
| A5 | Index determinism: two `scan-folder` runs on identical input produce byte-identical `db/video_index.json`/`videos.jsonl` | self-consistency | `diff` empty |
| A6 | Audit integrity: every produced `.jsonl` audit log passes `verify-audit`; a deliberately flipped byte in a copy fails verification | self + negative control | 100% pass on originals; `entry hash mismatch` on the tampered copy |
| A7 | Package manifest: `package-case` output verifies against its `manifest.sha256` | self-consistency | all entries hash-match |
| A8 | Candidate honesty: no output path labels unverified data as confirmed evidence — carve/index output keeps `candidate-*` semantics; only `ffprobe-video-stream-confirmed` may claim decodability | receipt review | manual review of labels in the receipt |

## 5. Federated comparison protocol

1. **Normalize outputs** to per-item tuples before comparing:
   `(path-or-offset, size_bytes, sha256, disposition)`. Write a small
   normalizer per tool (TSK `fls -r` output, Autopsy report CSV, X-Ways
   export, FrameTrace `db/video_index.json` + `db/carve_results.json`).
2. **Join on identity**, not name: match recovered bytes by SHA-256;
   match carve hits by `(offset, signature)`.
3. **Classify every disagreement** as one of: `missing-in-tool`
   (reference found, FrameTrace did not), `extra-in-tool` (FrameTrace
   found, reference did not — check the candidate label), or
   `divergent-content` (same identity, different bytes — adjudicate per
   §1 hierarchy).
4. **Record, don't fix, during measurement.** A failed assertion is
   recorded with the exact binary SHA-256 and input manifest so the
   producer can reproduce; fixes happen after the measurement pass.

## 6. Pass criteria

- A1–A2: zero `missing-in-tool` for planted data; zero
  `divergent-content`.
- A3–A4: all planted signatures found; planted-payload hash equality
  where deterministic.
- A5–A7: deterministic outputs, clean audit chains, verified manifest.
- A8: zero label violations.
- Any `extra-in-tool` must carry `candidate` labelling; an extra asserted
  as confirmed evidence is a P0 defect.

## 7. Corpus requirements

| Corpus | Contents | Ground truth |
| --- | --- | --- |
| C-FS | FAT/ext4 images with planted kept+deleted videos (mtools/mke2fs-generated and field-imaged variants) | planted manifest: path, inode, pre-deletion SHA-256 |
| C-CARVE | Raw images with MP4/MOV/DHAV signatures at recorded offsets, including a deliberately truncated tail | planted manifest: offset, signature, payload SHA-256 |
| C-MIXED | The existing mixed real-world case set (validation-corpus.md Corpus F) | examiner TSV manifest |
| C-DAV | **Real Dahua recorder exports** — still absent | see §8 |
| C-HIK | **Real Hikvision recorder exports** — still absent | see §8 |

## 8. Real DAV/Hikvision samples — what is needed and why

Current DAV and Hikvision lanes are validated only against **synthetic**
samples (hand-built DHAV/IMKH headers wrapping real H.264/MPEG-PS
elementary streams — see `docs/DAV_VALIDATION.md`,
`docs/HIKVISION_VALIDATION.md`, and the receipts). That proves the remux
pipeline handles the documented container shape; it does **not** prove
field correctness, because real recorders vary in:

- header/journal layouts across firmware generations and model lines;
- timestamp and event-metadata fields the synthetic fixtures don't carry;
- edge conditions (power-loss tails, circular-buffer wraps) that only
  appear in field media.

**Required samples** (all kept outside git, per the corpus storage rule;
only SHA-256 manifests are committed):

- Dahua: ≥3 `.dav` exports — one continuous-recording, one
  event/motion-triggered, one parking/low-bitrate — plus the recorder's
  model/firmware string for each.
- Hikvision: ≥3 exports covering the same three recording classes, with
  model/firmware noted.
- For each sample, a ground-truth playable reference: either the
  vendor-player-exported MP4 or an ffprobe-confirmed decode of the same
  span, so remux output can be compared against vendor truth, not just
  against "ffprobe accepts it".

Until these exist, the DAV/Hik lanes stay marked **BLOCKED for field
validation** in every receipt — the synthetic pass is a gate, not
closure.

## 9. Receipt convention

Every validation run produces a receipt doc at
`docs/<SCOPE>_VALIDATION_RECEIPT_<YYYY-MM-DD>.md` following
`docs/MACOS_FULL_RUN_RECEIPT.md`:

- header: target version/commit, machine + OS + tool versions, and the
  tested binary's SHA-256;
- a gate table (`fmt`/`clippy`/`test` results with counts);
- per-assertion measured table (assertion → expected → measured →
  verdict) including the comparison-tool output summaries;
- disagreement register with adjudication notes;
- corpus manifest rows (paths + SHA-256, no evidence committed);
- a "caveats" section that records what was *not* covered.

Dual-verifier rule: for third-party runs, the producing operator and the
verifying operator are different people; the receipt names both and the
verifier reruns at minimum the `verify-audit` + `sha256sum -c` +
one-spot-check assertion from their own copy of the image.
