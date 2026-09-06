# Glint Accessibility

Scope: accessibility of the existing menu-bar panel and the diff pop-out. The
compact design, feature set, and themes are unchanged; this is additive
semantics, focus, announcements, and OS-preference handling.

## What the Tauri webview actually exposes

Glint's UI runs in the macOS system WebView (WKWebView). This machine reports
macOS 26.6.2 with Safari/WebKit 26.6.2, so the WebView reads these system
Accessibility preferences into CSS media features:

| System Settings (Accessibility > Display) | CSS media feature | WebKit support |
|---|---|---|
| Reduce motion | `prefers-reduced-motion: reduce` | Safari 10.1+ |
| Reduce transparency | `prefers-reduced-transparency: reduce` | Safari 17+ |
| Increase contrast | `prefers-contrast: more` | Safari 14.1+ |
| Appearance (light/dark) | `prefers-color-scheme` | Safari 12.1+ (themes here are explicit) |

All four are recognized by the engine (confirmed with `matchMedia(...).media`
returning a valid feature, not `not all`). macOS has no `forced-colors` mode
(that is a Windows concept), so no `forced-colors` handling is added.

Native window effect: the panel window is `transparent: true` with an AppKit
`NSVisualEffectView` (HudWindow) behind the WebView. Under Reduce transparency,
AppKit renders that view opaque on its own; additionally the CSS paints an
opaque themed panel background (`--panel-solid`) that fully occludes the
vibrancy layer, so the composited panel is opaque and themed regardless. No
native code change is required for this and none was made.

## Implemented

- Staging control and diff opener are native `<button>`s (checkbox is
  `role="checkbox"` + `aria-checked`; the path opener names its action and file).
  They replace the old click-only `<span>`s, so both work with Space/Enter and
  take focus.
- `role="application"` removed from the panel so VoiceOver uses normal document
  navigation; file list keeps list semantics with explicit `role="list"` /
  `listitem` (needed because `list-style: none`).
- Visible keyboard focus ring (`:focus-visible`, `2px solid var(--ink)`,
  offset) on every control in every theme; thickened under `prefers-contrast`.
- Off-screen live regions: results announce politely, errors assertively
  (`#sr-status` / `#sr-alert`); the visible toast is no longer the live region,
  so each message is spoken once at the right priority. Commit and diff-load
  errors now announce.
- Sync buttons carry live counts and disabled state in their accessible name;
  the PR pill and diff counts have names; decorative icons are `aria-hidden`.
- Focus management: opening Settings moves focus in, marks the background
  `inert`, and Escape / Done restores focus to the opener. Onboarding focuses
  its primary action; connecting a repo lands focus on the first file.
- Reduced motion: transitions/animations and the press micro-scales are
  suppressed. Reduced transparency: opaque themed panel and overlays, no blur.
  Increased contrast: full-contrast secondary text, visible surface borders,
  thicker focus ring.
- Selectable/copyable repo name, branch, paths, status/error text, and diff
  contents (the diff window was already selectable; line numbers and +/- signs
  stay unselectable so a copy is clean).
- Diff scroll area is keyboard-focusable (`tabindex="0"`, labelled by the file
  heading) so arrow/Page keys scroll it.

## Verified in this environment

- `npm test` (17 tests, jsdom): semantic file controls with names/states;
  checkbox toggle updates `aria-checked`; path opens diff over IPC; sync
  names/disabled; polite vs assertive announcements; Settings focus-in / inert /
  Escape focus-restore; onboarding connect lands focus on a file; Cmd/Ctrl+Enter
  commits only staged files; i18n substitution and per-locale key coverage; CSS
  and markup affordance guards.
- Rendered in the WebKit-family in-app browser at 360x600:
  - File rows render as real checkbox/diff buttons; compact layout unchanged.
  - Focus ring visible on the header selector (Aurora) and on a file checkbox,
    keyboard `:focus-visible` confirmed in all five themes; computed outline is
    `2px solid var(--ink)` per theme: Aurora/Graphite `#1b1d22`, Midnight
    `#f2f3f8`, Sunset `#2a1c18`, Forest `#17251d`.
  - Diff body: keyboard Tab sets `:focus-visible`, computed outline
    `2px solid rgb(242,243,248)` (Midnight foreground); file name is an `<h1>`;
    SR counts read "3 additions, 2 deletions"; +/- glyphs are `aria-hidden`.

No Rust changed, so `cargo test` / `cargo build` (CI) are unaffected.

## Pending manual checks (not verified here)

These need a running signed build on a Mac with VoiceOver and the ability to
toggle System Settings, which this environment cannot do. Mark each as it is
confirmed.

Keyboard only (no mouse), menu-bar build:
- [ ] Tray opens the panel; Tab reaches selector, settings, pull, push, fetch,
      each file checkbox and path, commit summary/description, commit button.
- [ ] Space/Enter toggles a file checkbox; Enter/Space opens a diff.
- [ ] Type a summary, Cmd+Enter commits; focus returns to the summary field.
- [ ] Fetch, pull, push reachable and operable; disabled when count is 0.
- [ ] Settings: opens with focus inside, Tab stays within it, Escape closes and
      returns focus to the gear.
- [ ] Diff window: Tab focuses the diff, arrow/Page keys scroll.

VoiceOver:
- [ ] Each file row reads path + "checkbox, checked/unchecked" and a separate
      "Open diff" button.
- [ ] Fetch/pull/push read their name with the live count and disabled state.
- [ ] A failed pull/push/commit is announced (assertive); a success is
      announced (polite), once each.
- [ ] Diff window: file name as heading, counts spoken, load error announced.

Themes (Aurora, Midnight, Sunset, Forest, Graphite):
- [x] Focus ring clearly visible on a control in each of the five themes
      (checkbox spot-check, keyboard `:focus-visible`, verified here). A full
      per-control sweep with VoiceOver on device is still worthwhile.

OS preferences (toggle in System Settings, then observe):
- [ ] Reduce motion: no transitions, animations, or press-scale.
- [ ] Reduce transparency: panel and overlays fully opaque and themed; no
      desktop bleed; native vibrancy opaque.
- [ ] Increase contrast: stronger borders/text and a thicker focus ring.

Text selection:
- [ ] Repo name, branch, a file path, a toast/error message, and diff lines can
      be selected and copied; selecting a path does not trigger the diff.

## Running the frontend tests

```bash
npm install
npm test
```
