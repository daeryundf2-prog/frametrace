# Audit Chain HMAC Design

**Status: design only — not implemented.** Tracked as roadmap M5 candidate
(`docs/ROADMAP-v2.md` §4, open decision #1); adoption requires legal/practice
review before any code lands. This document records the current chain format,
the residual threat, and the proposed keyed format so the decision can be made
without re-deriving the analysis.

## Current chain structure

`audit::append_chained_jsonl` (`src/audit.rs`) appends one JSON object per line
to each `*.jsonl` audit log. Every entry carries two chain fields, appended in
fixed order at the end of the object:

```json
{"kind":"scan-folder", "...": "...", "previous_entry_sha256":"<hex>", "entry_sha256":"<hex>"}
```

- `previous_entry_sha256` — SHA-256 of the **raw previous line bytes**
  (including its own `entry_sha256` field and trailing content, excluding the
  newline). The first entry uses the literal `GENESIS`.
- `entry_sha256` — SHA-256 of the entry as serialized up to and including
  `previous_entry_sha256`, i.e. the line prefix `{...,"previous_entry_sha256":"X"}`
  before the `entry_sha256` field itself is appended.

`verify-audit` re-walks the chain: it recomputes each `entry_sha256` from the
recorded bytes and each `previous_entry_sha256` from the preceding raw line,
and reports torn final lines (a crash mid-append) distinctly from mid-chain
mismatches.

## What the unkeyed chain does and does not prove

The chain is **tamper-evident against accidental damage**: bit rot, torn
writes, truncated copies, and naive single-line edits all break verification.
That is already valuable — it proves the log was not corrupted in storage or
transit.

It is **not tamper-evident against a rewriting adversary.** Anyone with write
access to the case directory can modify or drop entries and recompute both
hash fields from scratch, producing a chain that verifies cleanly. The chain
commits to nothing the attacker cannot recompute, because it is keyed by
nothing.

## Threat a keyed chain mitigates

| Threat | Unkeyed chain | Keyed (HMAC) chain |
| --- | --- | --- |
| Torn write / corruption | Detected | Detected |
| Single-line edit | Detected | Detected |
| Full rewrite with recomputed hashes by anyone holding the case directory | **Undetectable** | Detected (attacker lacks the key) |
| Rewrite by an attacker who also holds the key | Undetectable | Undetectable — see non-goals |

The practical scenario: evidence copied to media, case directory shared with a
second workstation, or a package inspected by opposing counsel. Today a
recipient who distrusts the sender gains nothing from `verify-audit`; with a
keyed chain, a log produced under a key the sender never possessed (or that
lives only in the producing machine's OS keystore) cannot be silently
re-written.

## Proposed format

Opt-in, versioned by field presence — no schema flag file needed:

```json
{"kind":"scan-folder", "...": "...", "previous_entry_sha256":"<hex>", "entry_sha256":"<hex>", "entry_hmac_sha256":"<hex>", "entry_hmac_key_id":"<id>"}
```

- `entry_hmac_sha256` — HMAC-SHA-256 over **exactly the same bytes** that
  `entry_sha256` covers (the line through `previous_entry_sha256` + closing
  `}`). Hashing the same signed region means the HMAC is a pure additive
  field; the unkeyed verification rules are untouched.
- `entry_hmac_key_id` — short opaque identifier (`"default"`, `"2026-Q1"`,
  a key fingerprint prefix). It names the key, never contains key material,
  and enables rotation lookups.

Field order stays append-only at the tail of the object so
`entry_without_recorded_hash` style byte-surgery on the signed region keeps
working.

## Key storage

In order of preference; the first available source wins:

1. **OS-protected store.** Windows DPAPI (`CryptProtectData`, per-user), macOS
   Keychain (generic password item), Linux a `0600` file under the user's
   config dir (`~/.config/frametrace/audit-key` or platform equivalent). The
   key never lives inside the case directory — storing it next to the log
   would defeat the control entirely.
2. **Environment variable.** `FRAMETRACE_AUDIT_KEY` (hex- or base64-encoded
   32-byte key) for CI, portable installs, and multi-machine cases where the
   examiner deliberately manages the key. Documented as the weakest option:
   any process running as the examiner can read it, matching the existing
   loopback threat model (`docs/security-review.md`).

When no key is configured, append behavior is unchanged: entries are written
without the HMAC fields and no warning is emitted (keyed mode is opt-in).

## Backward-compatible verification

`verify-audit` must handle three log shapes:

| Log contents | Verification result |
| --- | --- |
| Entries without `entry_hmac_sha256` (all logs written before this feature, or written with no key configured) | Structural chain verified; report marks the log **"integrity-structural-only"** — the current documented limitation, made explicit in output. |
| Entries with `entry_hmac_sha256`, key available | Structural + HMAC verified; report marks **"integrity-keyed"**. |
| Entries with `entry_hmac_sha256`, key missing | Structural chain verified; report warns **"keyed entries present but key unavailable — authenticity unverified"** (still a pass with a loud caveat, so a log moved off its producing machine remains inspectable). |
| Mixed keyed/unkeyed entries | Per-entry result; the log-level mark degrades to structural-only with a mixed-chain warning. |

The unkeyed fields are never removed: keyed logs still verify structurally on
any machine, and the HMAC layer adds rather than replaces a check.

## Key rotation

- New appends always use the **current** key and stamp its
  `entry_hmac_key_id`.
- Verification accepts any configured key: the verifier holds an ordered
  key set `{key_id: key}` and tries the entry's declared `key_id` first,
  falling back to trying all known keys so a renamed id cannot brick old
  entries.
- Rotation is itself audited: rotating appends a signed marker entry
  (`{"kind":"audit-key-rotate","from":"<old id>","to":"<new id>"}`) keyed
  under the **new** key, so a gap between last-old-key and first-new-key
  entries is explainable in-log.
- Old keys are retained (in the OS store or env config) for as long as the
  case lives; deleting a retired key downgrades verification of its entries
  to structural-only, never to failure.

## Non-goals

- **Not a signature.** HMAC is symmetric: anyone holding the key can forge
  entries. This raises the bar from "anyone with the case directory" to
  "anyone with the key", nothing more. Asymmetric signing is a larger legal
  and operational commitment and out of scope for M5.
- **Not a write-once guarantee.** Truncating the log back to an earlier
  prefix remains possible regardless of keying; package-level manifests
  (`manifest.sha256`) and external timestamping are the controls for that.
- **Not a replacement for chain-of-custody documentation.** The legal review
  that gates this feature decides how much weight a keyed log carries in
  testimony; the design deliberately exposes the "integrity-structural-only"
  wording so unkeyed logs are never oversold.
