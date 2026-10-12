---
name: Obsidian Violet Media Core
colors:
  surface: '#10141a'
  surface-dim: '#10141a'
  surface-bright: '#353940'
  surface-container-lowest: '#0a0e14'
  surface-container-low: '#181c22'
  surface-container: '#1c2026'
  surface-container-high: '#262a31'
  surface-container-highest: '#31353c'
  on-surface: '#dfe2eb'
  on-surface-variant: '#cbc3d7'
  inverse-surface: '#dfe2eb'
  inverse-on-surface: '#2d3137'
  outline: '#958ea0'
  outline-variant: '#494454'
  surface-tint: '#d0bcff'
  primary: '#d0bcff'
  on-primary: '#3c0091'
  primary-container: '#a078ff'
  on-primary-container: '#340080'
  inverse-primary: '#6d3bd7'
  secondary: '#d2bbff'
  on-secondary: '#3f008e'
  secondary-container: '#6001d1'
  on-secondary-container: '#c9aeff'
  tertiary: '#7bd0ff'
  on-tertiary: '#00354a'
  tertiary-container: '#009bd1'
  on-tertiary-container: '#002d40'
  error: '#ffb4ab'
  on-error: '#690005'
  error-container: '#93000a'
  on-error-container: '#ffdad6'
  primary-fixed: '#e9ddff'
  primary-fixed-dim: '#d0bcff'
  on-primary-fixed: '#23005c'
  on-primary-fixed-variant: '#5516be'
  secondary-fixed: '#eaddff'
  secondary-fixed-dim: '#d2bbff'
  on-secondary-fixed: '#25005a'
  on-secondary-fixed-variant: '#5a00c6'
  tertiary-fixed: '#c4e7ff'
  tertiary-fixed-dim: '#7bd0ff'
  on-tertiary-fixed: '#001e2c'
  on-tertiary-fixed-variant: '#004c69'
  background: '#10141a'
  on-background: '#dfe2eb'
  surface-variant: '#31353c'
typography:
  headline-xl:
    fontFamily: Inter
    fontSize: 2rem
    fontWeight: '700'
    lineHeight: 2.5rem
    letterSpacing: -0.02em
  headline-lg:
    fontFamily: Inter
    fontSize: 1.5rem
    fontWeight: '600'
    lineHeight: 2rem
    letterSpacing: -0.015em
  headline-md:
    fontFamily: Inter
    fontSize: 1.25rem
    fontWeight: '600'
    lineHeight: 1.75rem
    letterSpacing: -0.01em
  title-sm:
    fontFamily: Inter
    fontSize: 0.9375rem
    fontWeight: '600'
    lineHeight: 1.375rem
    letterSpacing: '0'
  body-md:
    fontFamily: Inter
    fontSize: 0.875rem
    fontWeight: '400'
    lineHeight: 1.25rem
    letterSpacing: '0'
  body-sm:
    fontFamily: Inter
    fontSize: 0.8125rem
    fontWeight: '400'
    lineHeight: 1.125rem
    letterSpacing: '0'
  mono-metric:
    fontFamily: JetBrains Mono
    fontSize: 0.8125rem
    fontWeight: '500'
    lineHeight: 1rem
    letterSpacing: -0.02em
  mono-code:
    fontFamily: JetBrains Mono
    fontSize: 0.75rem
    fontWeight: '400'
    lineHeight: 1rem
    letterSpacing: '0'
  label-badge:
    fontFamily: JetBrains Mono
    fontSize: 0.6875rem
    fontWeight: '600'
    lineHeight: 0.875rem
    letterSpacing: 0.04em
rounded:
  sm: 0.25rem
  DEFAULT: 0.5rem
  md: 0.75rem
  lg: 1rem
  xl: 1.5rem
  full: 9999px
spacing:
  gutter: 0.75rem
  margin: 1rem
  space-2xs: 0.125rem
  space-xs: 0.25rem
  space-sm: 0.5rem
  space-md: 0.75rem
  space-lg: 1rem
  space-xl: 1.5rem
  space-2xl: 2rem
---

## Brand & Style

The design system is engineered for a high-performance Linux desktop anime library manager. The personality is precise, cybernetic, utilitarian, and deeply focused—reminiscent of bespoke developer environments and power-user automation suites (such as Sonarr and Taiga), while delivering the visual refinement expected of modern anime media tooling.

The aesthetic fuses **Utilitarian Minimalism** with **Subtle Neon-Tinged Glass and Dark Terminal accents**. High information density dominates the layout without visual noise: crisp monospaced telemetry, quiet borders, dark slate containers, and luminous violet highlights. It evokes total operational control, speed, and quiet reliability during prolonged low-light desktop sessions.

## Colors

The palette employs a multi-tiered system of cold, near-black blue-greys to anchor heavy media artwork and continuous data streams.

- **Background Canvas (`#0d1117` / `#11151c`)**: The deep void canvas prevents eye fatigue and lets bright anime poster art command attention.
- **Surface Elevation Tiers (`#161b22`, `#1a202c`, `#21262d`)**: Deep slate tones providing structural separation without high-contrast jumps.
- **Borders & Dividers (`#30363d`)**: Ultra-refined structural lines maintaining strict grid rhythm across dense toolbars and metadata matrices.
- **Primary & Secondary Violet (`#8b5cf6`, `#7c3aed`)**: Reserved strictly for actionable highlights, focused states, batch selection counters, active queue states, and neon pulse glows.
- **Tertiary Cyan (`#38bdf8`)**: Reserved for torrent health, network telemetry, and indexing markers.
- **Functional Semantics**:
  - Success/Seeding: `#10b981` (Emerald)
  - Warning/Missing: `#f59e0b` (Amber)
  - Error/Stalled: `#ef4444` (Rose)

## Typography

The type system balances quick scannability with machine readability:

- **Inter** handles high-legibility application chrome, navigation, modal dialogues, series titles, and narrative synopsis copy. Tightened letter-spacing on headline levels preserves compact density across multi-column views.
- **JetBrains Mono** powers all operational telemetry: download throughput, hash digests, file sizes, CRC32 checks, season/episode numbering (`S02E08`), video codecs (`HEVC 10-bit`), and release group strings. Its tabular numbers maintain strict vertical alignment across dense tables and grid overlays.

## Layout & Spacing

Designed for compact, information-dense desktop power users:

- **Structure**: A fixed collapsible sidebar navigation (220px expanded, 56px collapsed icon-only) combined with a fluid main workspace supporting responsive column reflow.
- **Poster Grid Rhythm**: Media posters adhere strictly to a **2:3 aspect ratio** (`aspect-ratio: 2 / 3`). The auto-fill grid maintains a minimum item width of 150px and a maximum of 210px, with `0.75rem` gutters.
- **Dense Data Tables**: Alternative view mode utilizes 32px height rows with compressed cell padding (`space-xs` vertical, `space-sm` horizontal) for rapid bulk triage and episode tracking.
- **Desktop Breakpoints**:
  - Compact Desktop (`1024px`): 4-column poster grid; secondary telemetry panels collapse to flyout drawers.
  - Standard Desktop (`1440px`): 6-column poster grid; side-by-side inspector view available.
  - Ultra-wide Workstation (`1920px+`): 8-to-10-column poster grid with persistent queue inspector dock.

## Elevation & Depth

Visual hierarchy is constructed via **Tonal Surface Stacking** coupled with **Violet Luminescence**, rejecting heavy, muddy drop shadows in favor of crisp technical layering:

- **Level 0 (Canvas)**: `#0d1117` base.
- **Level 1 (Panels, Sidebars, Toolbars)**: `#161b22` bounded by a 1px solid border of `#30363d`.
- **Level 2 (Cards, Floating Lists, Row Insets)**: `#1a202c` with `#30363d` outline.
- **Level 3 (Modals, Overlays, Command Palettes)**: `#21262d` with high-clarity edge highlight `rgba(255, 255, 255, 0.08)`.
- **Glow & Interaction Physics**:
  - Focused interactive controls and active download cards eliminate dark diffuse drop shadows. Instead, they project a focused, soft violet edge glow: `box-shadow: 0 0 0 1px #8b5cf6, 0 0 16px -2px rgba(139, 92, 246, 0.35)`.
  - Hover states on 2:3 poster cards smoothly lift via border transitions to `#8b5cf6` alongside an ambient `0 4px 20px rgba(124, 58, 237, 0.2)` under-glow.

## Shapes

The interface embraces a structured 12px (`0.75rem`) border-radius for primary containment surfaces, balancing modern softness with desktop efficiency:

- **Media Posters & Cards**: Standardized to `12px` (`0.75rem`), preserving card corner clarity without cropping key artwork details.
- **Input Fields & Buttons**: Softened to `6px` (`0.375rem`) to remain sharp, mechanical, and compact within data-dense toolbars.
- **Status Pills & Code Badges**: Styled with `4px` (`0.25rem`) to fit monospaced metadata snugly without excess pill padding.
- **Modal Dialogue Boxes**: Scaled up to `12px` (`0.75rem`) to match outer card envelopes.

## Components

### 2:3 Media Poster Cards
- Built using a continuous `2:3` portrait container with overflow hidden and a 12px border radius.
- Background defaults to `#161b22` with a 1px `#30363d` border.
- Bottom overlay features an integrated dark gradient (`rgba(13, 17, 23, 0.95)` to transparent) displaying the localized title, seasonal tracker (`12/12 EPS`), and air status.
- Top edges contain floating compact badges: release quality (`1080p`) anchored top-left, library monitor flag (monitored/unmonitored bookmark icon) top-right.
- Hover reveals an immediate quick-action micro-dock: Manual Search, Refresh Metadata, and Open Folder buttons.

### Action Buttons
- **Primary**: Solid `#8b5cf6` with `#ffffff` text, transitioning to `#7c3aed` on hover. Box shadow applies a subtle violet luminescence (`0 0 12px rgba(139, 92, 246, 0.4)`).
- **Secondary / Ghost**: `#161b22` background, 1px `#30363d` border, text `#c9d1d9`. On hover, border turns `#8b5cf6` and text shifts to `#ffffff`.
- **Terminal Action (Micro)**: JetBrains Mono uppercase labels, height 24px, padding 0 8px, font size 11px, ideal for queue pausing, re-checking hashes, and tag manipulation.

### Status Indicators & Badges
- Monospaced micro-chips (`JetBrains Mono`, 11px uppercase) with a 4px corner radius and 1px border.
- **Queued / Downloading**: Cyan text (`#38bdf8`), background `rgba(56, 189, 248, 0.1)`, border `rgba(56, 189, 248, 0.25)`. Includes an animated 6px circular pulse dot.
- **Completed**: Emerald text (`#10b981`), background `rgba(16, 185, 129, 0.1)`, border `rgba(16, 185, 129, 0.25)`.
- **Missing / Cutoff Unmet**: Amber text (`#f59e0b`), background `rgba(245, 158, 11, 0.1)`, border `rgba(245, 158, 11, 0.25)`.

### Inputs & Search Bars
- Background `#11151c` inset within `#161b22` panels.
- 1px border `#30363d`.
- Active state replaces border with `#8b5cf6` and applies a crisp `0 0 0 1px #8b5cf6` halo.
- Global library search includes an inline keyboard shortcut badge (`Ctrl + K` / `/`) set in monospaced muted grey (`#8b949e`).

### Progress Bars (Telemetry & Bandwidth)
- Thin 4px height background track of `#21262d` with a 2px radius.
- Active fill rendered in `#8b5cf6` or `#38bdf8` depending on state (importing vs downloading).
- Finished episodes feature segmented progress markers showing individual file presence along the bar.

### Lists & Episode Grids
- Alternating subtle stripe highlights on table rows (`transparent` vs `rgba(255, 255, 255, 0.015)`).
- Checkbox selections feature custom violet tick marks with immediate row perimeter lighting.