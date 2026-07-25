# Ductus Web — Design System
*v2 · 2026-07-19 · General-purpose visual system for every Ductus web surface (landing, docs, blog, playground). This document contains NO language semantics and NO page content — those live in per-surface specs (e.g. CLAUDE-DESIGN-PROMPT.md for the landing). If a rule here only makes sense on one page, it doesn't belong here.*

---

## 1. Principles

1. **Dark-first.** The primary theme is dark. A light reading variant exists only for long-form documentation surfaces.
2. **Purposeful motion only.** Nothing animates ambiently; motion communicates a state change and then stops. No loops, no idle particles, no decorative easing.
3. **Content is the imagery.** No stock art, no illustration, no hero videos. Real code, real data, real typography.
4. **Restraint in chrome, boldness in accents.** Neutral surfaces carry the page; the accent palette is vivid and used sparingly, always meaningfully.

## 2. Color tokens

### Neutral ramp (structure — never vivid)
| Token | Hex | Use |
|---|---|---|
| `ground` | `#0A0E17` | page background |
| `surface` | `#161D2C` | cards, panels, containers |
| `surface-2` | `#1C2536` | raised/hover surfaces |
| `border` | `#2A3550` | outlines, dividers |
| `border-dim` | `#1E2740` | subtle separators |
| `ink` | `#E7ECF5` | primary text |
| `ink-dim` | `#93A0B8` | secondary text |
| `ink-faint` | `#5C6A85` | tertiary/disabled text |

### Accent set (vivid — semantic assignment is per-surface, defined in each surface's spec)
| Token | Hex |
|---|---|
| `accent-cyan` | `#38D9F5` |
| `accent-violet` | `#9D7BFF` |
| `accent-magenta` | `#FF6EC7` |
| `accent-amber` | `#FFC53D` |
| `accent-ember` | `#FF6B57` |
| `accent-copper` | `#7A5C3E` (muted; the one non-vivid accent, for heavy structural strokes) |

### Status
| Token | Hex | Use |
|---|---|---|
| `positive` | `#4ADE80` | success states |
| `warn` | `#FFC53D` | warnings (shares accent-amber) |
| `negative` | `#FF4747` | errors, destructive actions |
| `highlight` | `#F4F7FF` | flashes, active sweeps, focus moments |

Rules: accents never tint large areas — they color small elements (dots, chips, keywords, underlines, gauges). Neutral ramp does all layout work. One overlay is sanctioned: `#8FA3BF @ 35%` + desaturation for "muted/inactive" regions.

### Light reading variant (docs only)
`ground #FAFBFD · surface #FFFFFF · border #E2E8F2 · ink #16202F · ink-dim #5C6A85`. Accents unchanged. Code blocks stay dark even in light mode.

## 3. Typography

- **Body — Inter.** Sizes: 13 / 15 / 17. Line-height 1.6. Body copy is sparse by voice rule.
- **Display — Space Grotesk.** Sizes: 22 / 28 / 40 / 64 / 96. Tight leading (1.05–1.15), used for headlines and full-bleed typographic blocks. Set wide, confident, never italic.
- **Code — JetBrains Mono.** Sizes: 13 / 15. No ligatures. Code keywords may take accent colors per the surface's syntax spec.
- Weights: Inter 400/500/600; Grotesk 500/700; Mono 400/700. No other families anywhere.

## 4. Spacing, radii, strokes

- **Spacing scale:** 4 · 8 · 12 · 16 · 24 · 32 · 48 · 64 · 96.
- **Radii:** 4 (chips, badges) · 10 (buttons, inputs, code blocks) · 14 (cards, panels) · 999 (dots, pills).
- **Strokes:** 1px standard border · 2px emphasis rule · 8px heavy structural stroke (rare, uses `accent-copper`).
- Cards: `surface` + 1px `border` + radius 14. **No drop shadows anywhere** — depth comes from the surface ramp.

## 5. Motion tokens

- Durations: `fast 120ms · base 200ms · slow 350ms · sweep 500ms`.
- Easing: `ease-out` for entrances/steps, `ease-in-out` for sweeps; no bounce, no spring, no overshoot.
- Every animation is triggered by a state change (user action, data update, navigation) and runs once.
- `prefers-reduced-motion`: all movement collapses to opacity/step changes at the same trigger moments; information conveyed by motion must survive its removal.

## 6. Components

- **Top nav** — slim bar on `ground`, wordmark left, text links right, 1px `border-dim` bottom rule. No mega-menus.
- **Footer** — `ink-faint` single-row, minimal.
- **Buttons** — primary (`ink` on `highlight`… inverse: dark text on near-white), ghost (1px border, `ink`), both radius 10; no gradients.
- **Link** — `ink` with accent underline on hover; accent chosen per surface.
- **Card** — per §4; header row optional; no shadow.
- **Code block** — always-dark panel, Mono 13/15, radius 10, optional line numbers in `ink-faint`, syntax accents per surface spec.
- **Callout / note** — card with 2px left rule in an accent or status color; body in Inter 15.
- **Table** — 1px `border-dim` row rules only; no zebra striping; numeric columns in Mono.
- **Chip / badge** — Mono 13, radius 4, 1px border; tinted text + border, never filled.
- **Stat card** — big Grotesk number + Inter caption; optional live-updating counter.
- **Display block** — full-bleed typographic interlude: one Grotesk line (40–96px) on `ground`, with the **§ margin tick** brand motif (small Mono `§` + optional reference in `ink-faint`, left margin). Content per surface.
- **Inline error/annotation** — accent-colored wavy or straight underline + Mono caption below; used by editors and forms alike.
- **Gauge / meter** — thin 4px track (`border`), accent fill, no gradient.

## 7. Iconography & brand

- **No icon font, no icon set, no emoji.** Where a glyph is needed, use a JetBrains Mono unicode character (`§ → ␣ ·`). Anything richer must be escalated as a design decision.
- **No logo mark exists.** The brand renders as the wordmark "DUCTUS" in Space Grotesk 700. Do not invent a mark. (The name is provisional; the wordmark must be trivially replaceable.)
- The **§ tick** is the one brand motif: a small Mono `§` set in the left margin of display typography.

## 8. Voice

- Sparse, declarative, specification-flavored. Short sentences. No exclamation points, no superlatives ("blazingly fast"), no competitor comparisons, no apology for project status.
- Technical claims are checkable or absent.

## 9. Hard bans (all surfaces)

Feature grids as hero content · testimonial walls · stock imagery/illustration · looping or ambient animation · particle backgrounds · drop shadows · gradients on interactive elements · countdown timers · "coming soon" pages · light-mode code blocks · icon fonts · invented logo marks.
