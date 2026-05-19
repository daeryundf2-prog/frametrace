# FrameTrace Forensic Workstation DESIGN.md

## 1. Color

FrameTrace uses a restrained workstation palette. The base is a low-glare warm gray, with white panels for evidence surfaces and muted borders for structure. Teal is the primary action color because it reads as operational without feeling like a consumer dashboard. Blue is reserved for secondary navigation and analytical context. Amber, red, green, and violet are state colors only.

Core tokens:

- Background: `#eef1f0`
- Panel: `#fbfcfb`
- Strong panel: `#ffffff`
- Primary ink: `#1f2724`
- Muted ink: `#68736f`
- Border: `#d8dedb`
- Strong border: `#bdc9c4`
- Primary action: `#0f7c71`
- Secondary action: `#1c5d8f`
- Warning: `#b4802a`
- Danger: `#b14d42`
- Verified/ok: `#2f7a48`
- Candidate: `#6c5d99`

State colors must carry text labels. Do not rely on color alone.

## 2. Typography

Use a system sans stack for Korean/English UI and a monospace stack for hashes, paths, IDs, command names, and timecodes.

- UI font: `Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`
- Mono font: `"SFMono-Regular", Consolas, "Liberation Mono", monospace`
- Letter spacing: `0`
- Avoid viewport-scaled type. Use fixed responsive sizes.
- Compact panels use 12-14px labels and 13-15px body text.
- Hero-scale type is not appropriate for the workstation surface.

## 3. Spacing

The shell uses a dense 8-10px rhythm so thousands of files can remain scannable without feeling cramped.

- App padding: `10px`
- Pane gap: `10px`
- Toolbar gap: `8px`
- Card/panel radius: `8px` maximum
- Button height: `34px`
- Inventory rows should keep stable height and never resize on hover.

## 4. Layout

The default Evidence Viewer is a four-pane workstation:

- Left: evidence sources and review queues.
- Center-left: searchable inventory with counts, thumbnails, parser lane, time, size, and status.
- Center: video/photo viewer, timeline, range selection, and derived-output controls.
- Right: forensic inspector, source path, hash state, parser, validation state, notes, and audit trail.

The current media viewer owns the visual center. Supporting panes are context, not the main event.

## 5. Components

Use familiar workstation controls:

- Segmented controls for media type, validation state, channel mode, and locale.
- Icon or short command buttons for playback, frame step, export, capture, zoom, and report selection.
- Search input and compact filter controls for large inventories.
- Tables/lists with stable row heights for file review.
- Inspector sections for immutable evidence metadata, validation, derived artifacts, and audit events.
- Status badges with explicit text labels.

All controls that imply forensic mutation must be wired to an audit-producing engine command in production.

## 6. Motion

Motion should be functional and minimal:

- Playback and timeline movement are allowed.
- Hover/focus states should be fast and subtle.
- Avoid decorative animation.
- Loading states must not shift layout or obscure evidence values.

## 7. Voice

Default Korean labels should be direct, short, and professional. English labels should match the same operational tone.

Preferred wording:

- "검증 대기" / "candidate-unvalidated"
- "검증됨" / "verified playable"
- "파생 산출물" / "derived artifact"
- "보고서 선택" / "report selected"
- "내보내기" / "export"

Never soften uncertain forensic status. If the engine has not validated a clip, the UI must say so.

## 8. Brand

FrameTrace should feel like a specialist forensic instrument:

- Precise, quiet, and evidence-led.
- Korean-first, Windows-first, local-first.
- Built for repeated review sessions, not a one-time demo.
- The brand mark can be compact and technical; it must not compete with the evidence viewer.

## 9. Anti-Patterns

- Marketing landing pages for examiner workflows.
- Large decorative cards, nested cards, gradient blobs, or purely atmospheric visuals.
- Ambiguous labels such as "good", "done", or "fixed" for forensic state.
- Bulk-rendering thousands of thumbnails at once.
- Hiding original source path, parser lane, validation status, or hash state.
- Treating E01-derived raw images as ordinary folders without source/audit context.
- Switching locale by mutating evidence values.
