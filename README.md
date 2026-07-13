# Glint

> A glint of Git in your menu bar — ultralightweight, liquid glass, themeable.

Glint is a menu-bar Git client: a reimagining of GitHub Desktop's ideas built on
**Tauri + Rust** instead of Electron. It lives in the macOS menu bar, uses native
`NSVisualEffectView` translucency for real liquid glass, and ships swappable
themes as CSS token sets.

- **Tiny.** System webview (WebKit) + a Rust core — no bundled Chromium.
- **Glassy.** Native vibrancy behind a transparent webview; themes tint the glass.
- **Menu-bar native.** Tray-positioned panel that toggles on click and dismisses on blur.

## Architecture

```
glint/
├─ src/                     Frontend (system webview) — HTML/CSS/JS, no framework
│  ├─ index.html            The glass panel
│  ├─ styles.css            Glass + theme variables
│  ├─ themes.js             Theme token sets (add themes here)
│  └─ app.js                Theme switching + Git rendering, calls Rust over IPC
└─ src-tauri/               Rust core
   ├─ src/main.rs           Tray, transparent window, vibrancy, IPC commands
   ├─ src/git.rs            Git status (via `git` CLI — see note below)
   ├─ tauri.conf.json       Window: transparent, borderless, always-on-top
   └─ capabilities/         Permissions
```

### Git backend note

`src/git.rs` currently shells out to the `git` CLI (`status --porcelain=2`) so the
first build is fast and dependency-free. Planned upgrade: migrate to
[`git2`](https://docs.rs/git2) (libgit2 bindings) for in-process reads, then native
`push`/`pull` with credential handling, and `octocrab` for PR status.

## Run it

Prerequisites: Rust, Node (for the Tauri CLI convenience scripts), and
`cargo-tauri` (`cargo install tauri-cli --version '^2'`).

```sh
# from glint/
cargo tauri dev        # or: npm run dev
```

Look for the Glint icon in your menu bar; click it to toggle the panel. Click the
selector in the header to point it at a real repo, or it shows sample data.

## Themes

Each theme is an entry in `src/themes.js` — an accent color, a swatch, and the
glass tint/ink variables. Drop in another object and it appears as a swatch
automatically. A future build will read user-authored themes from disk.

## Roadmap

- [ ] Migrate `git.rs` to `git2`
- [ ] Native push/pull + credentials
- [ ] PR status via `octocrab`
- [ ] Diff / merge-conflict "pop-out" window
- [ ] Monochrome template tray icon
- [ ] Vendor the icon font (drop the CDN link) for full offline use
- [ ] User-authored themes from disk
