# FrameTrace Forensic Workstation Design System

Use this design system for FrameTrace evidence-review screens, report-review screens, and Windows workstation prototypes.

FrameTrace is not a marketing site. It is a local-first forensic workstation for Korean examiners reviewing blackbox, CCTV, SD card, hard disk, and E01-derived evidence. Every design decision must keep source evidence, validation boundaries, and review throughput visible.

## Required Context

- Primary user: forensic examiner on a Windows 10/11 workstation.
- Default language: Korean UI labels, with English available as a secondary locale.
- Evidence values stay verbatim: paths, hashes, parser IDs, vendor names, timestamps, raw metadata, and file names are never translated or case-mutated.
- Primary object: the currently reviewed video/photo, not decorative dashboard chrome.
- Scale target: thousands of files from terabyte-scale media, with lazy thumbnails and fast filtering.
- Trust boundary: recovered/carved files remain candidates until the engine records validation.

## Design Priorities

1. Viewer-first layout with inventory, evidence sources, inspector, and audit context always near the media.
2. Dense but calm workstation styling: matte neutrals, teal/blue action accents, restrained warnings, no hero-page treatment.
3. High scanability for large evidence sets: counts, states, queue filters, parser lane, source path, time, and review status.
4. Explicit forensic status language: candidate, verified, duplicate, derived, exported, report-selected.
5. Durable handoff to the Rust engine: UI actions should map to source registration, scan, carve, validation, export, report, or package commands.

## Anti-Patterns

- Do not hide validation status behind icons alone.
- Do not present carved candidates as verified playable files.
- Do not overwrite or edit original evidence.
- Do not make decorative cards inside cards.
- Do not use landing-page hero sections, gradients, or large illustrative filler.
- Do not translate evidence data values when switching locale.
- Do not assume all thumbnails or metadata can be loaded at once.

## Files

- `DESIGN.md` contains the portable 9-section design system.
- `tokens/colors_and_type.css` mirrors the Evidence Viewer CSS tokens.
- `review_checklist.md` is the forensic/UI QC gate for OpenDesign or human review.
