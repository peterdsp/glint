# Glint — Design Language & Pixel Spec

The single source of truth for how Glint looks. Every number here is extracted
from the concept mockup. If an implementation disagrees with this document, the
implementation is wrong. The goal is **pixel-exact parity with the mock**.

> **Reading note — mockup vs. native.** The mockup fakes the desktop behind the
> panel with a per-theme gradient "wallpaper" and a fake macOS menu-bar strip.
> In the shipping Tauri app those two things are *real* (the OS menu bar; the
> live desktop seen through `NSVisualEffectView`). Wherever that changes a value,
> it's called out under **Native calibration**. Everything else must match to the
> pixel.

---

## 1. Design language — the five principles

1. **Liquid glass, not flat translucency.** The panel is a physical pane: a
   blurred, saturation-boosted view of what's behind it, with a bright 1px inner
   top highlight (the "wet edge"), a hairline outer stroke, and a soft drop
   shadow that lifts it off the desktop. Never a solid fill.
2. **One material, layered.** There is exactly one glass recipe. Everything else
   is either *on* the glass (inset tint surfaces) or *is* the accent (solid
   accent fills). No third material.
3. **The accent carries the theme.** Each theme is one accent color plus a
   tint/ink pair. The accent appears on exactly the interactive commit path
   (repo badge, sync button, stage checks, commit button) — nothing decorative.
4. **Glanceable density.** Everything critical fits one 340px-wide dropdown:
   identity → sync state → changes → commit. Type is small and tight; whitespace
   does the separating, not borders.
5. **Themeable by token, not by redesign.** Light (Aurora) and dark (Midnight)
   are the *same* structure with swapped tokens. If a change can't be expressed
   as a token swap, it doesn't belong in a theme.

---

## 2. Canvas & window

| Property | Value |
|---|---|
| Panel width | **340 px** (fixed) |
| Panel height | content-driven in mock; **560 px** window in app |
| Panel corner radius | **18 px** |
| Window background | fully transparent |
| Attach point | top-right, hangs below the menu-bar icon |
| Notch | 14×14 px square rotated 45°, centered under the tray icon |

**Native calibration:** the app window is `360×560` (20px of breathing room
around the 340px panel is fine); transparent, borderless, always-on-top,
`macOSPrivateApi` on. The notch is optional in v1 (macedon tray panels often drop
it); if used, it sits at `top: -6px; right: 14px`.

---

## 3. The glass material (the core recipe)

This is *the* recipe. Reproduce it exactly.

### 3a. Primary glass — the panel

```
background:        var(--tint)                         /* theme translucent tint */
backdrop-filter:   blur(30px) saturate(1.6)
border:            0.5px solid var(--stroke)
box-shadow:        0 20px 50px rgba(20,20,40,0.35),    /* drop — lifts off desktop */
                   inset 0 1px 0 rgba(255,255,255,0.60) /* wet top edge highlight */
border-radius:     18px
```

Layer order, bottom to top:
1. Drop shadow (`0 20px 50px rgba(20,20,40,0.35)`)
2. Backdrop blur+saturate of the desktop
3. Theme tint fill (`--tint`)
4. Hairline stroke (`0.5px --stroke`)
5. Inner top highlight (`inset 0 1px 0 rgba(255,255,255,0.6)`)

### 3b. Secondary glass — the theme card (mock only)

Same material, lighter shadow: `box-shadow: 0 12px 30px rgba(20,20,40,0.25)`,
`border-radius: 16px`, `blur(30px)`. *In the app the theme bar is merged into the
single panel (see §9.7), so this only exists in the mock.*

### 3c. Menu-bar strip glass (mock only — real menu bar in app)

```
height:           30 px
padding:          0 14px
background:        rgba(255,255,255,0.14)
backdrop-filter:   blur(20px)
border-bottom:     0.5px solid rgba(255,255,255,0.25)
color:             #fff
font:              13px / 500
```

### 3d. Inset surfaces (on the glass, no own blur)

Sync stats and the commit box sit on top of the glass. They do **not** blur
again — they're a slightly more opaque tint with a stroke:

| Surface | Fill | Border | Radius |
|---|---|---|---|
| Sync stat | `var(--tint-2)` | `0.5px var(--stroke)` | 11 px |
| Commit box | `rgba(255,255,255,0.50)` | `0.5px var(--stroke)` | 12 px |

**Native calibration:** WebKit `backdrop-filter` composites over the transparent
webview and the `NSVisualEffectView` behind it. Because real vibrancy already
darkens/blurs, the panel `--tint` alpha is retuned **down** from the mock so the
*composited* result matches. Mock uses `0.55` alpha; the app currently uses
`0.42`. Tune against a mid-tone desktop until panel luminance matches the mock.

---

## 4. Spatial system

### 4a. Corner-radius scale (every radius in the design)

| Token | Value | Used by |
|---|---|---|
| r1 | 5 px | stage checkbox |
| r2 | 7 px | icon buttons, menu-bar app icon |
| r3 | 9 px | repo badge, file row, swatch |
| r4 | 11 px | sync stat, sync button |
| r5 | 12 px | commit box, commit button |
| r6 | 16 px | theme card |
| r7 | 18 px | panel |

Rule of thumb: radius grows with the element's size so every corner looks like
the same physical bevel. Never introduce a radius outside this scale.

### 4b. Padding & gaps (literal, per region — do not "round to a scale")

| Region | Padding | Internal gap |
|---|---|---|
| Header | `14px 16px 12px` | 10 px |
| Sync row | `12px 16px` (app: `11px 14px`) | 8 px |
| Sync stat (inner) | `8px 10px` | 7 px |
| Section label | `2px 16px 4px` | — |
| File list | `0 10px` (app: `0 8px`) | — |
| File row | `7px 8px` | 9 px |
| Commit region | `10px 16px 16px` (app: `8px 14px 12px`) | — |
| Commit box (inner) | `9px 11px` (app: `8px 11px`) | — |
| Commit button | `10px` | — |
| Theme bar | `12px 14px` (app: `10px 14px 12px`) | swatches 9 px |

Hairline dividers between regions: `0.5px solid var(--tint-2)`.

---

## 5. Color & theming

### 5a. Token model

Six variables define every theme. Nothing in the UI uses a raw color except the
two semantic diff colors (§5c).

| Token | Meaning |
|---|---|
| `--accent` | interactive/brand color (badges, buttons, checks, arrows) |
| `--tint` | panel glass fill (translucent) |
| `--tint-2` | inset surface fill + hairline dividers + hover |
| `--stroke` | hairline borders and highlights |
| `--ink` | primary text |
| `--ink-2` | secondary text |

### 5b. The five themes (canonical mock values)

Accent, ink, ink-2, and swatch are theme identity. Tints/stroke are the glass.

| Theme | `--accent` | `--ink` | `--ink-2` | Swatch |
|---|---|---|---|---|
| **Aurora** (default) | `#5b8cff` | `#1b1d22` | `#5a5f6b` | `#8ea8ff` |
| **Midnight** | `#7c8cff` | `#f2f3f8` | `#a7adbf` | `#1f2333` |
| **Sunset** | `#ff7a59` | `#2a1c18` | `#7a5a4e` | `#ff9a6a` |
| **Forest** | `#1d9e75` | `#17251d` | `#4e6a5c` | `#5fb8a0` |
| **Graphite** | `#111318`¹ | `#1b1d22` | `#5f636e` | `#c2c5cd` |

Glass tints per theme:

| Theme | `--tint` (mock) | `--tint-2` | `--stroke` |
|---|---|---|---|
| Aurora | `rgba(255,255,255,0.55)` | `rgba(255,255,255,0.35)` | `rgba(255,255,255,0.60)` |
| Midnight | `rgba(38,42,58,0.55)` | `rgba(255,255,255,0.08)` | `rgba(255,255,255,0.18)` |
| Sunset | `rgba(255,255,255,0.50)` | `rgba(255,255,255,0.32)` | `rgba(255,255,255,0.60)` |
| Forest | `rgba(255,255,255,0.50)` | `rgba(255,255,255,0.32)` | `rgba(255,255,255,0.60)` |
| Graphite | `rgba(255,255,255,0.60)` | `rgba(0,0,0,0.05)` | `rgba(255,255,255,0.70)` |

¹ Graphite accent reads near-black on glass in the mock. The app softens it to
`#3a3f4a` so the commit button doesn't look disabled. Decide which is canonical;
this doc recommends the softened `#3a3f4a` for the app.

Midnight is the proof that the material survives dark mode: only the tint flips
to a dark translucent (`rgba(38,42,58,0.55)`) and ink inverts — structure,
radii, blur, and shadow are unchanged.

### 5c. Mockup backdrop gradients (reference only — replaced by real desktop)

Used *only* to demonstrate the glass in the mock. The app shows the live desktop
instead. Kept here so mock and screenshots are reproducible.

| Theme | Backdrop `linear-gradient(145deg, …)` |
|---|---|
| Aurora | `#8ea8ff 0%, #c9a3ff 55%, #ffc4d6 100%` |
| Midnight | `#161a26 0%, #1f2333 55%, #2a2140 100%` |
| Sunset | `#ffb37a 0%, #ff7a9c 55%, #c85adf 100%` |
| Forest | `#8fd6b0 0%, #5fb8a0 55%, #3d9d8f 100%` |
| Graphite | `#d7d9df 0%, #c2c5cd 55%, #aeb2bd 100%` |

### 5d. Semantic diff colors (theme-independent)

| Meaning | Color |
|---|---|
| Additions (`+N`) | `#1d9e75` |
| Deletions (`−N`) | `#d85a30` |

Note the deletion glyph is a true minus `−` (U+2212), not a hyphen.

---

## 6. Typography

System font stack: `-apple-system, BlinkMacSystemFont, "SF Pro Text",
system-ui`. Monospace: `"SF Mono", ui-monospace, "JetBrains Mono"`.

| Role | Size | Weight | Color | Notes |
|---|---|---|---|---|
| Repo name | 14 px | 500 | `--ink` | ellipsis on overflow |
| Branch name | 12 px | 400 | `--ink-2` | with fork icon |
| Sync number | 15 px | 500 | `--ink` | line-height 1 |
| Sync label | 11 px | 400 | `--ink-2` | |
| Section label | 11 px | 500 | `--ink-2` | UPPERCASE, letter-spacing `0.04em` |
| File path | 12 px | 400 | `--ink` | **monospace**, ellipsis |
| Diff delta | 11 px | 500 | semantic | |
| Commit summary | 13 px | 500 | `--ink` | |
| Commit description | 12 px | 400 | `--ink-2` | |
| Commit button | 13 px | 500 | `#fff` | |
| Theme name | 12 px | 400 | `--ink-2` | e.g. "Aurora — translucent, cool" |

Two weights only: **400** and **500**. Never 600/700. Sentence case
everywhere except section labels (uppercase) and proper nouns.

---

## 7. Iconography

Tabler Icons, **outline** set only. Sizes are literal:

| Icon | Where | Size |
|---|---|---|
| `git-branch` | repo badge, tray | 17 px (badge) |
| `git-fork` | branch line | 13 px |
| `selector` | switch-repo button | 18 px |
| `arrow-down` / `arrow-up` | pull / push | 16 px |
| `refresh` | sync button | 18 px |
| `check` | staged checkbox | 11 px |

Icon color inherits: accent on accent surfaces, `--ink-2` on plain, `#fff` on
solid accent fills. **Native:** vendor the Tabler webfont into the bundle (drop
the CDN link) so the app is fully offline.

---

## 8. Accent-fill elements (the commit path)

Every solid-accent element shares one look: `background: var(--accent)`, white
icon/text, and — on tappable ones — an inner top highlight
`inset 0 1px 0 rgba(255,255,255,0.35)` and `active { transform: scale(0.98) }`.

| Element | Size | Radius | Icon/label |
|---|---|---|---|
| Repo badge | 30×30 | 9 | `git-branch` 17px, white |
| Sync button | 40 wide, row-height | 11 | `refresh` 18px, white |
| Stage checkbox (on) | 16×16 | 5 | `check` 11px, white |
| Commit button | full width, `10px` pad | 12 | text 13/500, white |

Unstaged checkbox: `16×16`, `border: 1.5px solid var(--ink-2)`, no fill.

---

## 9. Component specs (top to bottom)

### 9.1 Header
Row, `padding 14px 16px 12px`, `gap 10px`, bottom hairline `0.5px var(--tint-2)`.
Left: repo badge (§8). Center (flex, min-width 0): repo name (14/500, ellipsis)
over branch line (`git-fork` 13px + 12px `--ink-2`, gap 5px). Right: selector
icon button — `padding 4px`, `radius 7px`, `--ink-2`, `hover: bg var(--tint-2)`.

### 9.2 Sync row
Three items, `padding 12px 16px`, `gap 8px`.
- Two stat pills (flex:1): inset surface (§3d), `padding 8px 10px`, `gap 7px`,
  `radius 11px`. Arrow icon 16px accent, then number (15/500, line-height 1)
  over label (11px `--ink-2`). Left = `to pull` (down arrow), right = `to push`
  (up arrow).
- Sync button: `40px` wide, accent fill, `refresh` 18px white, `radius 11px`.

### 9.3 Section label
`padding 2px 16px 4px`, 11/500 uppercase, letter-spacing `0.04em`, `--ink-2`.
Text: `N changed file(s)`.

### 9.4 File list
`padding 0 10px`. Each row: `padding 7px 8px`, `gap 9px`, `radius 9px`,
`hover: bg var(--tint-2)`.
- Checkbox (§8) — staged = accent+check, unstaged = 1.5px outline.
- Path: 12px **monospace** `--ink`, flex:1, ellipsis.
- Delta: 11/500, `+N` green `#1d9e75` / `−N` coral `#d85a30`.

### 9.5 Commit box
`radius 12px`, `background rgba(255,255,255,0.50)`, `border 0.5px var(--stroke)`,
`padding 9px 11px`, `margin-bottom 9px`. Summary input (13/500 `--ink`) over
description input (12px `--ink-2`, `margin-top 2–3px`).

### 9.6 Commit button
Full width, `padding 10px`, `radius 12px`, accent fill, white 13/500, inner top
highlight, `active scale(0.99)`. Label: `Commit to <branch>`.

### 9.7 Theme bar
Section label `Theme`, then swatch row (`gap 9px`), then theme-name line
(12px `--ink-2`).
- Swatch: `28×28` (mock 30), `radius 9px`, `background: <swatch>`, default ring
  `0 0 0 1px rgba(0,0,0,0.15)`.
- Selected swatch ring: `0 0 0 2px <panel tint>, 0 0 0 4px <accent>`.

**Native:** in the mock the theme bar is a *separate* floating glass card below
the panel. In the app it's folded into the bottom of the single panel with a top
hairline `0.5px var(--tint-2)` — one window, one material.

---

## 10. Interaction & states

| Trigger | Behavior |
|---|---|
| Click tray icon | toggle panel; position under icon |
| Panel loses focus | hide panel (menu-bar convention) |
| Hover file row / icon button | `background: var(--tint-2)` |
| Click checkbox | toggle staged; accent fill ⇄ outline |
| Press accent button | `transform: scale(0.98–0.99)` |
| Click swatch | apply theme tokens instantly; persist; move selection ring |
| Click selector | choose repo |

No transitions except the swatch ring (`box-shadow 0.15s`) and button press. No
fades on theme change — tokens swap instantly.

---

## 11. Native implementation mapping (CSS → Tauri / AppKit)

| Mock (CSS) | App (Tauri + macOS) |
|---|---|
| Fake wallpaper gradient | Real desktop via `NSVisualEffectView` |
| `backdrop-filter: blur(30px) saturate(1.6)` | `apply_vibrancy(HudWindow, Active, radius 16)` + WebKit `backdrop-filter` for the tint layer |
| `--tint` @ 0.55 | retuned to ~0.42 so composite over live vibrancy matches |
| Fake menu-bar strip | real macOS menu bar (delete the strip) |
| Panel `border-radius: 18px` | window transparent + rounded webview; vibrancy view mask rounded to match |
| Drop shadow | native window shadow off (`shadow:false`); glass shadow drawn in CSS |
| Notch under icon | `tauri-plugin-positioner` `TrayBottomCenter`; notch optional |
| Tabler via CDN | vendor webfont into bundle |
| Colored app icon in tray | **monochrome template** tray icon (`iconAsTemplate`) |

**Material choice:** `HudWindow` is the closest AppKit vibrancy to the mock's
bright glass. If it reads too gray on dark desktops, evaluate `Popover` /
`UnderWindowBackground`. The chosen material must land the composited panel
luminance on the mock — this is the one thing to A/B against a screenshot.

---

## 12. Fidelity checklist (acceptance criteria)

Ship only when all pass, checked against the mock screenshot at 2× on a mid-tone
desktop:

- [ ] Panel is 340px wide, 18px corners, visible drop shadow + top highlight.
- [ ] Glass shows the desktop blurred and slightly *more saturated* behind it.
- [ ] Aurora and Midnight both render from the same structure via token swap.
- [ ] Accent appears on exactly: repo badge, sync button, staged checks, commit
      button — nowhere else.
- [ ] File paths are monospace; deltas use `#1d9e75` / `#d85a30` with a true `−`.
- [ ] All radii come from the §4a scale; all type is 400/500 only.
- [ ] Section labels are 11px uppercase, letter-spacing 0.04em.
- [ ] Selected swatch shows the double ring (tint + accent).
- [ ] No fade on theme change; only the ring and button-press animate.
- [ ] Tray icon is a monochrome template; font is vendored (no network).
