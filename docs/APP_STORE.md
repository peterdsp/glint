# App Store Connect - ready-to-paste metadata

Everything to fill the Mac App Store listing. Copy each field in. Assets are in
`docs/assets/store/`. The App Store build is `cargo tauri build
--no-default-features --features appstore` (no trial, no license gate, no
self-updater - Apple gates the purchase and pushes updates).

## Prerequisites (yours - one-time)

- **Apple Developer Program** membership, signed in to App Store Connect.
- **Paid Apps Agreement** active (App Store Connect -> Business -> Agreements) plus tax + banking. Required for any paid app.
- **Bundle ID** `dev.peterdsp.glint` registered at developer.apple.com -> Identifiers (macOS App). Use this exact id - it matches `tauri.conf.json`.
- **App name** must be unique on the store. First choice "Glint"; if taken, use "Glint: Menu Bar Git".

## New app (App Store Connect -> Apps -> +)

| Field | Value |
|---|---|
| Platform | macOS |
| Name | `Glint: Menu Bar Git` (the plain `Glint` was already taken on the store; app ID 6790709729) |
| Primary language | English (U.S.) |
| Bundle ID | `dev.peterdsp.glint` |
| SKU | `glint-macos` |
| User access | Full |

## App information

- **Subtitle** (30 char max): `Git in your menu bar`
- **Category:** Primary `Developer Tools`, Secondary `Productivity`
- **Support URL:** `https://github.com/peterdsp/glint`
- **Marketing URL:** `https://glint.peterdsp.dev`

## Pricing

- **Price:** `$2.99` (the USD 2.99 tier). Availability: all countries.

## Version information (1.0)

**Promotional text** (170 char max):
```
A native, liquid-glass Git client that lives in your menu bar. Commit, pull, push, and read diffs in one click. Tiny, themeable, and out of your way.
```

**Description** (up to 4000 chars):
```
Glint is an ultralightweight Git client that lives in your menu bar. Click the icon and a small glass panel drops down; stage, commit, pull, push, and read diffs, then it is gone.

Built on Tauri and Rust rather than Electron, Glint is megabytes, not hundreds of them, and it uses the system webview with real macOS vibrancy for genuine liquid glass.

FEATURES
- Native menu-bar app with real vibrancy and five built-in themes, plus your own from a small token file.
- One-click commit, fetch, pull, and push, using your existing SSH keys or Keychain credentials.
- A diff pop-out window that renders changes with clear add and remove coloring.
- Live pull request and CI status next to your branch.
- Available in English, Greek, and Albanian.
- Change themes and language from a Settings screen; the base panel stays focused on your repo.

Glint stays out of your way: no bundled browser, no account, no telemetry, nothing phoning home. Point it at any local repository and it just works.

Requires macOS 12 or later.
```

**Keywords** (100 char max, comma-separated, no spaces after commas):
```
git,menu bar,commit,push,pull,diff,version control,developer,github,repo,vcs,liquid glass
```

**What's New in This Version** (1.0):
```
First release of Glint. A native menu-bar Git client with liquid-glass themes, one-click commit and sync, a diff pop-out, live PR and CI status, and English, Greek and Albanian.
```

**Copyright:** `2026 peterdsp`

## Screenshots (macOS, 2560 x 1600)

Upload from `docs/assets/store/` in this order:
1. `appstore-1-hero.png` - "Your repo, one click from the menu bar."
2. `appstore-2-themes.png` - "Five themes. Or your own."
3. `appstore-3-features.png` - "Tiny. Native. Yours."

App icon (1024 x 1024) is `src-tauri/icons/icon.png`, already in the build.

## App privacy

- **Data collection:** None. Glint does not collect or transmit user data. PR/CI status uses the user's own local GitHub CLI token and talks directly to GitHub; Glint stores nothing.
- Answer Apple's privacy questionnaire as **Data Not Collected**.
- **Privacy policy URL:** `https://glint.peterdsp.dev/privacy.html` (written and deployed - see `site/privacy.html`). Answers "Data Not Collected".

## Already done in App Store Connect (app ID 6790709729)

The listing is fully populated:
- App record created: `Glint: Menu Bar Git`, macOS, bundle `dev.peterdsp.glint`, SKU `glint-macos`.
- Version 1.0 metadata: promotional text, description, keywords, support + marketing URLs, copyright, and the 3 screenshots.
- App Information: subtitle `Git in your menu bar`, category Developer Tools / Productivity.
- Pricing: `$2.99` USD base, applied to all 175 regions (Apple auto-adjusts the rest).
- App Privacy: `Data Not Collected`, privacy policy URL set, and published.

## What still needs you

These are native-tool and legal steps that cannot be done from a browser:
- **Paid Apps Agreement + tax + banking** (App Store Connect -> Business). Required before the price can go live.
- **Build, sign, and upload** the `appstore` build (Apple Distribution + Mac Installer certs) via Xcode or Transporter.
- Once the build is attached, **submit for review** from the version page.
