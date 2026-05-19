# OpenDesign Adaptation

OpenDesign is not installed in this Codex session, so it was not invoked directly. The repository now includes the project-native files OpenDesign expects to discover later:

```text
opendesign/design-systems/frametrace-forensic-workstation/
```

The design system follows OpenDesign's file-first model: a design-system folder contains a `SKILL.md` marker, a portable `DESIGN.md`, and CSS token files. The current OpenDesign repository documents that the front-door skill scans `./opendesign/design-systems/*/` for `SKILL.md` or `tokens/colors_and_type.css` markers, then routes work to the appropriate artifact skill.

## What Was Adapted

- Question-form discipline became a FrameTrace brief: forensic workstation, Korean-first, Windows-first, viewer-first, and large-case scale.
- Design-system tokens mirror the current Evidence Viewer CSS so future generated screens stay visually compatible.
- Anti-patterns are forensic-specific: no decorative hero UI, no hidden validation status, no translated hashes/paths, and no candidate-to-verified promotion without engine validation.
- QC is split into forensic correctness, large-case usability, viewer quality, and production handoff.

## How To Use Later

When the OpenDesign plugin is available in Codex, invoke it against this repository and ask it to use the existing FrameTrace design system. Example:

```text
/opendesign refine the evidence viewer for a Korean Windows forensic workstation using the existing FrameTrace design system
```

Generated artifacts should be reviewed against:

```text
opendesign/design-systems/frametrace-forensic-workstation/review_checklist.md
```

The production GUI boundary remains unchanged: the static Evidence Viewer is a prototype, and the Rust engine is the source of truth for evidence registration, scan, carve, validation, export, report, and package actions.

## References

- OpenDesign project site: https://opendesigner.io/
- OpenDesign repository and Codex installation notes: https://github.com/manalkaff/opendesign
