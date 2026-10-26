# FrameTrace Design System (`DESIGN.md`)

## 1. Visual Direction & Domain Persona
- **Domain**: Offline Forensic Video Recovery & Examiner Workstation (`frametrace-app`, `frametrace` CLI, standalone `viewer/index.html`).
- **Aesthetic Direction**: **Archival Paper Document + Dark Media Stage** (inspired by Linear, Sony Ci Media, and DaVinci Resolve).
- **Core Philosophy ("딱 필요한 설명과 버튼만")**:
  - Zero decorative bloat, zero marketing chrome, zero telemetry.
  - Paper-warm low-glare surfaces (`#ecebe7` / `#fbfaf7`) for long-session document & metadata reading in light mode; neutral carbon slate (`#121416` / `#1b1e21`) in dark mode.
  - Dedicated neutral dark video stage (`#141618` light theme / `#0e1012` dark theme) so video frames, thumbnails, and waveform/telemetry canvases are never polluted by surrounding chrome tint.
  - Strict elimination of legacy swamp-teal (`#232c28`, `#3a4f48`, `#16231f`, `#eef6f3`) and coloured accent left-borders on rounded containers.

## 2. Color Tokens (`assets/tokens.css`)

`assets/tokens.css` is the single source of truth; this table is generated from it and
`scripts/check-css.mjs` fails when a value or token name here drifts from the CSS.
Light = `:root`, Dark = `html[data-theme="dark"]` (a token not redefined in the dark
block keeps its light value).

| Token | Light | Dark | Role |
| :--- | :--- | :--- | :--- |
| `--bg` | `#ecebe7` | `#121416` | Workbench & grid canvas |
| `--surface` | `#fbfaf7` | `#1b1e21` | Cards, rail, sheets, popovers, details |
| `--surface-2` | `#f3f2ee` | `#24282c` | Hover fills, secondary wells, group headers |
| `--border` | `#d6d3cc` | `#33383d` | Hairline dividers (decorative) |
| `--border-strong` | `#858177` | `#767d84` | Control outlines (>= 3:1 non-text) |
| `--text` | `#1a1c1e` | `#e6e4df` | Body text |
| `--muted` | `#55595e` | `#a7acb2` | Secondary text (>= 4.5:1 on bg / surface / surface-2 / accent-soft) |
| `--disabled-fg` / `--disabled-bg` | `#6f7276` / `#e6e4df` | `#848a90` / `#1b1e21` | Disabled controls (fg >= 3:1 on bg) |
| `--accent` | `#234e8c` | `#8db4ea` | Ink-blue accent: primary CTA fill, selection, focus ring |
| `--on-accent` | `#ffffff` | `#0f1a2b` | Text on --accent |
| `--accent-ink` | `#1c3f73` | `#a9c7f2` | Accent text / primary hover |
| `--accent-soft` | `#e4ebf5` | `#22304a` | Selected / active tonal fill |
| `--media-bg` | `#141618` | `#0e1012` | 16:9 video stage & thumbnail backdrop (dark in both themes) |
| `--media-surface` | `#1b1e22` | `#16191d` | Media title bar & transport toolbar |
| `--media-border` | `#2e333a` | `#282d34` | Hairlines inside the media bars (decorative) |
| `--media-control-border` | `#6e747b` | `#6e747b` | Toolbar / stage control outlines (>= 3:1) |
| `--media-fg` | `#eef0f2` | `#eef0f2` | Primary text on media surfaces |
| `--media-muted` | `#b4b9be` | `#b4b9be` | Secondary text on media surfaces |
| `--media-disabled-fg` | `#6a7077` | `#6a7077` | Disabled toolbar text (>= 3:1, dimmer than --media-muted) |
| `--media-disabled-border` | `#4a5057` | `#4a5057` | Dashed disabled toolbar outline (>= 1.5:1, below --media-control-border) |
| `--media-hover` / `--media-active` | `#252a30` / `#2f363e` | `#21262c` / `#2b3139` | Toolbar button hover / pressed fill |
| `--media-ring` | `#8db4ea` | `#8db4ea` | Focus ring & hover outline on media surfaces |
| `--cal-l1-bg` / `--cal-l1-border` | `#eef2f8` / `#c5d3e8` | `#1c2739` / `#283a54` | Calendar heat level 1 |
| `--cal-l2-bg` / `--cal-l2-border` | `#dde6f3` / `#a3bae0` | `#223352` / `#344e73` | Calendar heat level 2 |
| `--cal-l3-bg` / `--cal-l3-border` | `#c6d6ec` / `#7b9fd4` | `#2c4569` / `#426494` | Calendar heat level 3 |
| `--mark-hl-bg` / `--mark-hl-fg` | `#fde68a` / `#1a1c1e` | `#614e14` / `#fef3c7` | Search-hit highlight (<mark>) |
| `--ok` / `--danger` / `--warn` / `--info` | `#2a6b3a` / `#b3261e` / `#7a5200` / `#234e8c` | `#7fcb95` / `#ff8a80` / `#e8b45a` / `#8db4ea` | Semantic status text only |
| `--verified-fg` / `--verified-bg` | `#2a6b3a` / `#e4f1e7` | `#7fcb95` / `#17352a` | Confirmed video stream |
| `--failed-fg` / `--failed-bg` | `#b3261e` / `#fbeae8` | `#ff8a80` / `#3d2422` | Validation failed (red) |
| `--candidate-fg` / `--candidate-bg` | `#5e4f91` / `#eeecf6` | `#b8a9ec` / `#2a2640` | Unverified candidate |
| `--original-fg` / `--original-bg` | `#1c3f73` / `#e4ebf5` | `#a9c7f2` / `#22304a` | Original file |
| `--carved-fg` / `--carved-bg` | `#96470a` / `#fbeadd` | `#f0a060` / `#3a2616` | Carved |
| `--fs-recovered-fg` / `--fs-recovered-bg` | `#1f4fb0` / `#e5ecfa` | `#8fb0f5` / `#1c2a4a` | Filesystem-recovered |
| `--deleted-candidate-fg` / `--deleted-candidate-bg` | `#9b1c1c` / `#fbe6e6` | `#ff8f8f` / `#3d2222` | Deleted candidate (red) |
| `--anomaly-fg` / `--anomaly-bg` | `#7a5200` / `#f8efd9` | `#e8b45a` / `#3a2e1a` | Timestamp / structure anomaly |
| `--warn-fg` / `--warn-bg` | `#7a5200` / `#fbf1dc` | `#e8b45a` / `#3a2e1a` | Non-fatal warning |
| `--info-fg` / `--info-bg` | `#234e8c` / `#e4ebf5` | `#8db4ea` / `#22304a` | Information |
| `--mark-reviewed-fg` / `--mark-reviewed-bg` / `--mark-reviewed-ink` | `#3d4146` / `#e2e1dc` / `#55595e` | `#d6d9dd` / `#3a3f45` / `#a7acb2` | Mark 검토됨 (reviewed) = **grey check** |
| `--mark-important-fg` / `--mark-important-bg` / `--mark-important-ink` | `#5c3a00` / `#f6dfb3` / `#8a5a00` | `#1f1500` / `#e0a94a` / `#e0a94a` | Mark 중요 (important) = **amber/orange flag** |
| `--mark-needs-verification-fg` / `--mark-needs-verification-bg` / `--mark-needs-verification-ink` | `#4a3b7f` / `#e7e2f5` / `#5e4f91` | `#15102a` / `#b8a9ec` / `#b8a9ec` | Mark 확인 필요 (needs verification) = **violet question mark** |

Review marks are distinguished by glyph **and** colour: reviewed = grey check,
important = amber/orange flag, needs verification = violet question mark. Red is
reserved for failed / deleted-candidate / destructive. `*-ink` mark tokens are the
single-colour glyph on a plain surface (non-text, `>= 3:1`).

Contrast contract (enforced by `scripts/check-css.mjs` in both themes): every text
token on `--bg` / `--surface` / `--surface-2` `>= 4.5:1`; status and mark fg on their
own bg `>= 4.5:1`; `--disabled-fg` on `--disabled-bg` `>= 3:1`; `--border-strong`,
`--accent`, `*-ink` on the three surfaces `>= 3:1`; media text on every media fill
`>= 4.5:1`; `--media-ring` and `--media-control-border` on `--media-bg` /
`--media-surface` / `--media-hover` `>= 3:1` (the ring also bounds the pressed `.on` toggle, whose light-theme accent fill is ~2:1 on the bar);
`--media-disabled-border` on `--media-surface` `>= 1.5:1` and below the control outline; calendar ramp steps never shrink;
`--media-disabled-fg` on `--media-surface` `>= 3:1` and
clearly dimmer than `--media-muted`; `--text` on every calendar level and
`--mark-hl-fg` on `--mark-hl-bg` `>= 4.5:1`.

## 3. Typography & Scannable Hierarchy
- **UI Sans (`--font-sans`)**: `"Pretendard Variable", Pretendard, "Apple SD Gothic Neo", "Malgun Gothic", "Noto Sans KR", system-ui, sans-serif` (system fonts only; nothing is downloaded)
- **Tabular Mono (`--font-mono`)**: `ui-monospace, "SFMono-Regular", "JetBrains Mono", Menlo, Consolas, "Liberation Mono", monospace`
- **Scale & Line Heights**:
  - `--fs-xs`: `13px` (minimum text size; new UI must not go below it)
  - Known exceptions still at `12px`, deferred because they sit in fixed widths / chip rows (`assets/evidence_viewer.css`, many overridden later by `controls.css` / `kit.css`): `.date-row input`, `.rail-chip` / `.chip-toggle`, `.active-filters .af-chip`, `.af-chip .af-x`, `.card.active .thumb::after` (▶ marker glyph), `.selection-bar button`, `.menu-list button`, `.tag-add input` / `.tag-add button`, `.timeline-list`, `.dlg .dlg-field`, the inline-styled custom tag input `#customTagInput` (`assets/viewer/60-details.js`, fixed `104px`); plus the summary report table (`12px`, print-only HTML).
  - `--fs-sm`: `14px` (default body, buttons, inputs, table cells)
  - `--fs-md`: `16px` (sheet/dialog headers, weight 600)
  - `--fs-lg`: `20px` (empty-state and dialog titles)
  - `--lh-ctl`: `1.35`, `--lh-body`: `1.5`
- **Shape & motion**: `--radius` `3px`; shadows only on popovers (`--shadow-pop`) and sheets (`--shadow-sheet`); transitions `--dur-fast` 120ms / `--dur` 150ms / `--dur-slow` 180ms, all `0ms` under `prefers-reduced-motion`.
- **Numeric Discipline**: All timestamps, frame counts, byte sizes, SHA-256 hashes, durations, and badge counters enforce `font-variant-numeric: tabular-nums`.
- **CJK Line-Break Discipline**: Interactive labels, buttons, badges, and pills enforce `white-space: nowrap` or `word-break: keep-all`.

## 4. Layout, Spatial Architecture & Viewport Budgets
Target desktop viewports: `1366×768` (minimum workstation budget) through `1920×1080`+.
1. **Top Strip (`#hTopStrip`)**: Fixed `44px` height, `1px` bottom hairline (`--border`), 3 clear clusters:
   - Left: Brand identity + Project selector trigger (`#btnProjects`) + Scan status pill (`#btnScanState`).
   - Center-Right: Examiner credit chip (`#btnExaminer`).
   - Far-Right: Finalize CTA (`#btnOpenFinalize`) + Overflow utility menu (`#btnMore`).
2. **Viewer Filter Topbar (`.ft-viewer .topbar`)**: Single compact row (`min-height: 44px`, `padding: 6px 12px`) with search input, status count pills (`.count-pills`), sort selector, and overflow filter popover (`#btnFilterMore`), plus collapsible active-filter bar (`#activeFilterBar`).
3. **3-Pane Forensic Workbench (`.ft-viewer .main`)**:
   - **Left Facet Rail (`#leftRail`)**: `240px` (`1366×768`) / `280px` (`1920×1080`). Houses quick filters, ink-blue perceptual calendar heatmap, and collapsible source/codec/channel trees.
   - **Center Virtual Thumbnail Grid (`section.browse`)**: `>= 600px` (`1366×768`) / `>= 960px` (`1920×1080`). Fixed card heights (`168px` normal, `128px` compact) for 60fps virtual scrolling.
   - **Right Media & Details Column (`section.media`)**: `480px` (`1366×768`) / `640px` (`1920×1080`). Top 16:9 dark stage (`270px` / `360px`) + transport toolbar (`#mediaToolbar`) + bottom tabbed inspector (`#miniInfo` with Summary, Technical, Integrity, Notes/Tags).
4. **Bottom Verdict & Batch Action Bar (`#selectionBar`)**: Fixed `<= 52px` (`44px` nominal), organized into Selection Count, Examiner Verdict Segmented Group (`검토됨` / `중요` / `확인 필요` / `해제`), Tag Menu, and Primary Export CTA (`선택 항목 내보내기`).
5. **Empty State (`#inputCard`)**: Centered single-column (`max-width: 760px`) with 2 structured archival cards: `#blkEvidence` (Source selection + inline preflight readiness) and `#blkRecent` (Recent projects table + 1-click resume).

## 5. Component & Interaction Specifications (`assets/kit.css`)
- **4 Button Kinds**:
  - `primary`: `--accent` fill, `--on-accent` text, `min-height: 36px` (`--ctl-h-lg`), max 1 visible per `[data-region]`.
  - `secondary` (default `<button>`): `--surface` fill, `--border-strong` border, `min-height: 32px` (`--ctl-h`).
  - `quiet`: transparent fill, `--text` ink, subtle `--surface-2` hover.
  - `destructive`: `--danger` outline and text on `--surface`; `primary destructive` = `--danger` fill with `--on-accent` text, `min-height: 36px`, reserved for confirmation dialogs.
- **5 Mandatory Interactive States**: Every button and interactive control has visually and computationally distinct `default`, `hover`, `active` (`translateY(1px)`), `focus-visible` (`2px solid var(--accent)`, `outline-offset: 2px`), and `disabled` states.
- **Card & List Selection**:
  - Active card in grid: the card's `1px` border turns `--accent` plus a `2px` shadow ring (`box-shadow: 0 0 0 2px var(--accent)`), an `--accent-soft` meta strip and a ▶ badge in the thumbnail corner (shape, not colour alone); under forced colors a `3px double Highlight` outline—never a thick inset left border on `.meta`.
  - Active tree/rail items use `--accent-soft` tonal fill + `1px solid var(--accent)` border + bold `--accent-ink` typography—never a 3px/4px left border stripe on rounded corners.
  - Group headers use `--surface-2` fill + `1px solid var(--border)` + semantic status dot/badge instead of 4px coloured left borders.
  - Toasts use elevated `--surface` with `1px solid var(--border-strong)` and `--shadow-pop`; warn / error (and shell ok) toasts take the status fg/bg pair plus a kind glyph (shell: i / ✓ / ! / ✕, viewer: ! warn / ✕ error) so the kind never depends on colour alone (also under forced colors). No 4px left border stripe.
  - Media toolbar (`#mediaToolbar`): controls outlined with `--media-control-border`; pressed `.on` toggle = `--accent` fill bounded by `--media-ring`; disabled = `--media-disabled-fg` text, transparent fill and a dashed `--media-disabled-border` outline; focus and hover use `--media-ring` (also on the video stage and thumbnail controls).

## 6. Accessibility & Agent Experience (AX) Guardrails
- **WCAG 2.2 AA Contrast**: `>= 4.5:1` for all text/badge/status pairs; `>= 3.0:1` for control borders, focus rings, disabled foregrounds, and mark icons across both Light and Dark themes.
- **Target Size (WCAG 2.5.8)**: Every interactive element has a minimum `24×24px` bounding box (`32px` standard controls, `36px` primary CTAs, `28×28px` icon buttons).
- **Keyboard & Screen Reader**: Roving `tabindex` in virtual grid and tablists, `aria-pressed` on toggles, `aria-selected` on cards/tabs, `Escape` stack closing popovers -> modals -> sheets -> selection.
