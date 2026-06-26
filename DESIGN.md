# FrameTrace Design System

## 1. Atmosphere & Identity

FrameTrace feels like a local-first forensic workstation: quiet, dense, precise, Korean-first, Windows-first, and evidence-led. The signature is a viewer-centered matte workstation where source path, parser lane, validation state, hash state, and audit context stay near the media without competing with it. The interface must never soften uncertainty: carved or recovered media remains `candidate-unvalidated` until the engine records validation, and any item that still needs engine confirmation must carry an explicit `verification-needed` label.

## 2. Color

### Palette

The current CSS source of truth is `gui/evidence-viewer/styles.css`. OpenDesign mirrors these tokens in `opendesign/design-systems/frametrace-forensic-workstation/tokens/colors_and_type.css` with the `--frametrace-*` prefix.

| Role | Current token | OpenDesign token | Value | Usage |
|------|---------------|------------------|-------|-------|
| Surface/app | `--bg` | `--frametrace-bg` | `#eef1f0` | Low-glare workstation background |
| Surface/panel | `--panel` | `--frametrace-panel` | `#fbfcfb` | Panes, sections, tool surfaces |
| Surface/strong | `--panel-strong` | `--frametrace-panel-strong` | `#ffffff` | Topbar, controls, elevated evidence fields |
| Text/primary | `--ink` | `--frametrace-ink` | `#1f2724` | Primary UI text and labels |
| Text/muted | `--muted` | `--frametrace-muted` | `#68736f` | Captions, hints, secondary metadata |
| Border/default | `--line` | `--frametrace-line` | `#d8dedb` | Panel edges, row dividers |
| Border/strong | `--line-strong` | `--frametrace-line-strong` | `#bdc9c4` | Controls, selected boundaries |
| Action/primary | `--accent` | `--frametrace-accent` | `#0f7c71` | Primary action, active row marker, focus emphasis |
| Action/secondary | `--accent-2` | `--frametrace-accent-secondary` | `#1c5d8f` | Secondary navigation and analytical context |
| Status/danger | `--danger` | `--frametrace-danger` | `#b14d42` | Important, destructive, or risk states |
| Status/warning | `--warn` | `--frametrace-warning` | `#b4802a` | `verification-needed`, timeline events |
| Status/ok | `--ok` | `--frametrace-ok` | `#2f7a48` | Reviewed or verified states |
| Status/candidate | `--candidate` | `--frametrace-candidate` | `#6c5d99` | `candidate-unvalidated`, recovered/carved candidates |
| Depth/shadow | `--shadow` | `--frametrace-shadow` | `0 14px 38px rgba(33, 43, 39, 0.14)` | Workstation elevation when needed |

### Rules

- State colors always carry text labels. Never rely on color alone.
- `--accent` is operational, not decorative. Use it for active, selected, hover, and command states.
- Use amber only for `verification-needed` or event warnings, violet only for `candidate-unvalidated`, green only for reviewed or engine-verified states, and red only for important risk or destructive states.
- Preserve evidence values verbatim across Korean and English locale changes; do not recolor or relabel evidence data in ways that change meaning.

## 3. Typography

### Font Stack

| Token | Value | Usage |
|-------|-------|-------|
| `--sans` | `-apple-system, BlinkMacSystemFont, "Apple SD Gothic Neo", "Malgun Gothic", "Noto Sans KR", "Segoe UI", Inter, ui-sans-serif, system-ui, sans-serif` | Korean-first Windows workstation UI |
| `--mono` | `"SFMono-Regular", Consolas, "Liberation Mono", monospace` | Hashes, paths, IDs, command names, timecodes |

OpenDesign currently mirrors `--frametrace-sans` with `Inter` first. The root project follows the live Evidence Viewer CSS stack above unless the tokens are explicitly consolidated.

### Scale

| Level | Size | Weight | Line Height | Tracking | Usage |
|-------|------|--------|-------------|----------|-------|
| Workstation title | `18px` | `700` | `20px` | `0` | Compact brand/case title |
| Viewer title | `14px` | `700` | normal | `0` | Current media name |
| Body | `13px` | `400-700` | normal | `0` | File names, source lines, queue rows |
| Metadata | `12px` | `400-800` | `16-17px` where needed | `0` | Source subtitles, controls, review cells, inspector values |
| Table header | `11px` | `800` | normal | `0` | Uppercase file table headers and metadata keys |
| Dense code | `10.5-11px` | `400` | normal | `0` | Hash cells, IDs, inline evidence code |

### Rules

- Body text in dense panels stays in the 12-14px range; hero-scale type is not part of FrameTrace workstation UI.
- Letter spacing is `0`.
- Use the monospace stack for evidence identifiers, hashes, file paths, timestamps, parser IDs, and audit event times.
- Korean labels should be short, direct, and professional. English labels should use the same operational tone.

## 4. Spacing & Layout

### Base Rhythm

FrameTrace uses a dense 4px-compatible workstation rhythm with common steps at 6px, 8px, 10px, 12px, and 18px because the live CSS optimizes for stable review panes and large evidence inventories.

| Token or pattern | Value | Usage |
|------------------|-------|-------|
| `--radius` / `--frametrace-radius` | `8px` | Maximum panel and card radius |
| App padding | `10px` | Workspace outer padding |
| Pane gap | `10px` | Grid gap between source, inventory, viewer, and inspector panes |
| Toolbar/control gap | `6-8px` | Segmented controls, transport, viewer toggles |
| Topbar height | `64px` | Case identity and global actions |
| Button height | `34px` | Default buttons and selects |
| Compact button height | `30px` | Segmented controls, toolbar buttons, selection actions |
| Round transport button | `36px` square | Playback and frame-step controls |
| File row height | `44px` via `--row-height` | Stable inventory rows |
| File header height | `34px` | Stable table header |
| Status pill | `24px` high, `64px` min-width | Default status badges |
| Focused status pill | `20px` high, `56px` min-width | Inventory-focused density mode |

### Grid

- Body has no fixed minimum width; responsive rules must keep horizontal overflow at zero for 375px, 768px, and desktop QA widths.
- App shell: `64px minmax(720px, 1fr)`.
- Topbar: `240px 1fr auto`, `18px` gap, `18px` horizontal padding.
- Default review workspace: media viewer first, candidate inventory second, forensic inspector third.
- Narrow workstation workspace below `1320px`: keep the viewer first and reduce only secondary inventory/inspector widths.
- Inventory-focused workspace: `172px minmax(0, 1fr) 270px`, with the viewer pane hidden.
- Viewer stage rows: `46px minmax(0, 1fr) 82px 48px`.
- Media canvas: `16 / 9`, contained within the viewer and never cropped for decoration.

### Rules

- The review workstation is viewer-first: media/recovered-candidate preview, candidate inventory, and forensic decision/export/report controls. Source context is compact summary, not a competing first-screen pane.
- The current media viewer owns the visual center; supporting panes are context.
- Inventory rows, toolbar heights, and viewer dimensions must not resize on hover, selection, loading, or empty states.
- Large cases must support lazy thumbnails and virtualized rows; do not bulk-render thousands of thumbnails at once.

## 5. Components

### Workstation Shell

- **Structure**: topbar above a grid workspace with source pane, inventory/browser pane, viewer pane, and inspector pane.
- **Spacing**: `64px` topbar, `10px` workspace padding and pane gap.
- **States**: default four-pane mode and `inventory-focused` mode.
- **Rules**: no marketing hero treatment, decorative cards, nested cards, gradient blobs, or atmospheric filler.

### Source and Queue Panels

- **Structure**: `summary-panel` and `queue-panel` blocks with `panel-title`, list rows, title line, and muted subtext.
- **Spacing**: `8px` list padding, `9px 8px` row padding, `7px` row radius.
- **States**: active rows use a teal-tinted background and `rgba(15, 124, 113, 0.34)` border.
- **Content**: distinguish E01, raw image, mounted folder, SD card, exported media, and report queues.

### Inventory Browser

- **Structure**: metrics, filters, search, virtualized table, stable rows, optional media thumbnail column, and selection bar.
- **Spacing**: `44px` rows, `34px` header, `6px` column gaps, `10px` horizontal row padding.
- **States**: hover `#f7faf8`, active `#e8f5f2` with inset `3px` accent marker, selected `#f0f7f5`, empty state.
- **Content**: counts, queue states, parser lane, source path, time, size, hash state, report state, and validation state must remain visible.

### Status Badges

- **Structure**: text badge with pill shape.
- **Variants**: `unreviewed`, `reviewed`, `important`, `verification-needed`, `candidate-unvalidated`, `verified playable`, `derived artifact`, `report selected`, `exported`.
- **Spacing**: `24px` height, `64px` minimum width, `999px` radius.
- **States**: color-coded by semantic token plus explicit text label.
- **Rules**: carved or recovered media uses `candidate-unvalidated` until validation exists; engine-pending items use `verification-needed`.

### Viewer Stage

- **Structure**: toolbar, media frame, timeline, and transport controls.
- **Spacing**: toolbar `46px`, timeline `82px`, transport `48px`, media padding `6px`.
- **States**: playback, frame step, zoom, capture, range selection, export, synchronized channel review.
- **Content**: timecode and current source context stay visible while viewing.

### Inspector

- **Structure**: status card, review actions, derived-output grid, metadata list, and activity/audit list.
- **Spacing**: `12px` section padding, `76px minmax(0, 1fr)` metadata grid, `7px` action-grid gaps.
- **States**: default, reviewed, important, `verification-needed`, `candidate-unvalidated`, exported, report selected.
- **Content**: immutable evidence metadata, source path, hash state, parser, validation state, derived artifacts, notes, and audit events.

## 6. Motion & Interaction

### Timing

| Type | Duration | Easing | Usage |
|------|----------|--------|-------|
| Micro | `100-150ms` | `ease-out` | Hover, active, focus, and control feedback |
| Standard | `200-300ms` | `ease-in-out` | Panel, filter, and selection state changes if motion is needed |
| Evidence movement | live media timing | native playback/timeline behavior | Playback, timeline head, frame stepping |

### Rules

- Motion is functional and minimal. Playback and timeline movement are the primary allowed motion.
- Avoid decorative animation.
- Loading states must not shift layout or obscure evidence values.
- Interactive elements need visible hover, active, and focus states.
- Animate only `transform` and `opacity` when animation is necessary; do not animate layout properties.
- Respect reduced-motion preferences by disabling non-essential animation.
- Any UI command that changes case state must map to a Rust engine command and an audit event in production.

## 7. Depth & Surface

### Strategy

FrameTrace uses a mixed but restrained workstation strategy: tonal matte surfaces plus thin borders, with shadows reserved for true elevation such as the media canvas and bulk preview.

| Surface | Treatment | Usage |
|---------|-----------|-------|
| App background | `--bg` | Low-glare workstation base |
| Panels | `--panel`, `1px solid var(--line)`, `8px` radius | Source, browser, viewer, inspector sections |
| Strong panels | `--panel-strong` | Topbar, controls, stat cards |
| Panel inset | `0 1px 0 rgba(255, 255, 255, 0.75) inset` | Subtle workstation separation |
| Media frame | `#151d1a` / `#151c1a` with canvas shadow | Evidence viewer contrast |
| Elevated media | `0 18px 42px rgba(6, 12, 10, 0.32)` | Active canvas only |
| Bulk preview | `0 -8px 24px rgba(27, 35, 32, 0.08)` | Temporary selected-evidence preview |
| General elevation token | `--shadow` | Reserved elevation, not default cards |

### Rules

- Depth must clarify evidence hierarchy, not decorate.
- Do not nest cards inside cards.
- Do not hide original source path, parser lane, validation state, or hash state behind elevation or collapsed panels.
- Keep the media legible: inspector data and overlays must not obscure evidence.
