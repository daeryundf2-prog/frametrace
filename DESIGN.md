# FrameTrace Design System (`DESIGN.md`)

## 1. Visual Direction & Domain Persona
- **Domain**: Offline Forensic Video Recovery & Examiner Workstation (`frametrace-app`, `frametrace` CLI, standalone `viewer/index.html`).
- **Aesthetic Direction**: **Archival Paper Document + Dark Media Stage** (inspired by Linear, Sony Ci Media, and DaVinci Resolve).
- **Core Philosophy ("딱 필요한 설명과 버튼만")**:
  - Zero decorative bloat, zero marketing chrome, zero telemetry.
  - Paper-warm low-glare surfaces (`#ecebe7` / `#fbfaf7`) for long-session document & metadata reading in light mode; neutral carbon slate (`#121416` / `#1b1e21`) in dark mode.
  - Dedicated neutral dark video stage (`#0e1012` / `#14171a`) so video frames, thumbnails, and waveform/telemetry canvases are never polluted by surrounding chrome tint.
  - Strict elimination of legacy swamp-teal (`#232c28`, `#3a4f48`, `#16231f`, `#eef6f3`) and coloured accent left-borders on rounded containers.

## 2. Color & Atmospheric Polarity (`assets/tokens.css`)

### Surface Palette
| Token | Light Mode (Warm Archival Paper) | Dark Mode (Carbon Slate) | Role |
| :--- | :--- | :--- | :--- |
| `--bg` | `#ecebe7` | `#121416` | Workbench & grid canvas background |
| `--surface` | `#fbfaf7` | `#1b1e21` | Primary cards, rail, sheets, filter bar, details |
| `--surface-2` | `#e2e0da` | `#24282c` | Secondary wells, hover fills, segmented tracks |
| `--surface-3` | `#d4d1c9` | `#2e3338` | Active pressed fills, subtle badges |
| `--media-bg` | `#0e1012` | `#0a0b0d` | 16:9 video stage & thumbnail backdrop |
| `--media-surface` | `#171a1e` | `#14171a` | Media player header & transport bar |
| `--media-border` | `#2a2f36` | `#242930` | Media stage internal hairlines |
| `--media-fg` | `#eef0f2` | `#eef0f2` | Media stage primary text |
| `--media-muted` | `#9ca3af` | `#9ca3af` | Media stage secondary metadata |
| `--border` | `#c9c6be` | `#32373d` | Hairline structural dividers (`1px solid`) |
| `--border-strong` | `#7d7a72` | `#747b84` | Control outlines (`>= 3:1` non-text contrast) |

### Typography & Ink Hierarchy
| Token | Light Mode | Dark Mode | Min Contrast |
| :--- | :--- | :--- | :--- |
| `--text` | `#17181a` | `#eef0f2` | `>= 13:1` on `--bg` / `--surface` |
| `--muted` | `#52555a` | `#a4abb3` | `>= 4.5:1` on `--bg` / `--surface` / `--surface-2` |
| `--disabled-fg` | `#66696e` | `#7c838c` | `>= 3.0:1` on `--disabled-bg` & surfaces |
| `--disabled-bg` | `#e4e2dc` | `#1f2327` | Disabled control fill |

### Accent & Semantic Tokens (Single Source of Truth)
| Category | Token Prefix | Light (FG / BG / Ink) | Dark (FG / BG / Ink) | Usage |
| :--- | :--- | :--- | :--- | :--- |
| **Primary Accent** | `--accent*` | `#234e8c` / `#dfe7f3` / `#1c3f73` | `#6ea3eb` / `#1d2b3f` / `#8cb7f0` | 1 primary CTA per region, active selection rings, focus outlines |
| **Verified (`ok`)** | `--verified*` | `#15603a` / `#dff0e6` | `#6fd09c` / `#162b21` | Confirmed video stream (`ffprobe-video-stream-confirmed`) |
| **Candidate** | `--candidate*` | `#58427c` / `#ebe6f4` | `#bfa9e6` / `#262033` | Unverified carved/filesystem candidate |
| **Anomaly / Warn** | `--anomaly*` / `--warn*` | `#7c4d08` / `#f6ead4` | `#e5b567` / `#2f2414` | Timestamp/structure anomaly, non-fatal warnings |
| **Failed / Danger** | `--failed*` / `--danger*` | `#96231c` / `#f8e2e0` | `#f28b82` / `#331b1a` | Corrupt stream (`validation-failed`), destructive confirmation |
| **Mark: Reviewed** | `--mark-reviewed*` | `#15603a` / `#dff0e6` / `#1e7a4c` | `#6fd09c` / `#162b21` / `#52b782` | Examiner mark: Reviewed (`검토됨`) |
| **Mark: Important** | `--mark-important*` | `#8f2d14` / `#f9e4dc` / `#b83b1a` | `#f59e84` / `#331e19` / `#e87a5a` | Examiner mark: Important (`중요`) |
| **Mark: Verify** | `--mark-needs-verification*` | `#7c4d08` / `#f6ead4` / `#9e630a` | `#e5b567` / `#2f2414` / `#d49b38` | Examiner mark: Needs Verification (`확인 필요`) |

## 3. Typography & Scannable Hierarchy
- **UI Sans (`--font-sans`)**: `"Pretendard Variable", "Pretendard", "Malgun Gothic", "Apple SD Gothic Neo", "Segoe UI", system-ui, sans-serif`
- **Tabular Mono (`--font-mono`)**: `ui-monospace, "Cascadia Mono", "Consolas", monospace`
- **Scale & Line Heights**:
  - `--fs-xs`: `13px` (minimum text size across the entire application; sub-13px is forbidden)
  - `--fs-sm`: `14px` (default body, buttons, inputs, table cells)
  - `--fs-md`: `16px` (sheet/dialog headers, empty-state card titles)
  - `--fs-lg`: `18px` (major modal titles)
  - `--lh-ctl`: `1.25`, `--lh-body`: `1.5`
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
  - `secondary` (default `<button>`): `--surface` fill, `--border-strong` border, `height: 32px` (`--ctl-h`).
  - `quiet`: transparent fill, `--text` ink, subtle `--surface-2` hover.
  - `destructive`: `--danger` fill, `--on-danger` text, `min-height: 36px`, reserved exclusively for confirmation dialogs.
- **5 Mandatory Interactive States**: Every button and interactive control has visually and computationally distinct `default`, `hover`, `active` (`translateY(0.5px)`), `focus-visible` (`2px solid var(--accent)`, `outline-offset: 2px`), and `disabled` states.
- **Card & List Selection**:
  - Active card in grid uses a crisp `2px` `--accent` border + outer ring (`0 0 0 2px var(--accent)`) + subtle play badge in thumbnail corner—never a thick inset left border on `.meta`.
  - Active tree/rail items use `--accent-soft` tonal fill + `1px solid var(--accent)` border + bold `--accent-ink` typography—never a 3px/4px left border stripe on rounded corners.
  - Group headers use `--surface-2` fill + `1px solid var(--border)` + semantic status dot/badge instead of 4px coloured left borders.
  - Toasts use elevated `--surface` with `1px solid var(--border-strong)`, subtle shadow, and semantic status badge/pill rather than a 4px left border stripe.

## 6. Accessibility & Agent Experience (AX) Guardrails
- **WCAG 2.2 AA Contrast**: `>= 4.5:1` for all text/badge/status pairs; `>= 3.0:1` for control borders, focus rings, disabled foregrounds, and mark icons across both Light and Dark themes.
- **Target Size (WCAG 2.5.8)**: Every interactive element has a minimum `24×24px` bounding box (`32px` standard controls, `36px` primary CTAs, `28×28px` icon buttons).
- **Keyboard & Screen Reader**: Roving `tabindex` in virtual grid and tablists, `aria-pressed` on toggles, `aria-selected` on cards/tabs, `Escape` stack closing popovers -> modals -> sheets -> selection.
