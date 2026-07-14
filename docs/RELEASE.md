# Releasing & selling Glint

Glint ships through two channels with different rules.

| Channel | Price | Trial | Updates | Build |
|---|---|---|---|---|
| **Ko-fi** (direct download, macOS/Windows/Linux) | **$4.99** | 7-day free, then license key | Self-updates via GitHub Releases | default features |
| **Mac App Store** | **$2.99** | none (Apple gates purchase) | Apple pushes updates | `--no-default-features --features appstore` |

The two builds come from the same code. A Cargo feature flips the behavior:

- **default** (`updater` feature on): trial + license gate + self-updater. This is the Ko-fi build.
- **`appstore`**: no trial, no license gate, no self-updater. Apple forbids self-updating apps, and the store handles payment, so all of that is compiled out.

---

## One-time setup

### 1. Update signing key (already generated)

`cargo tauri signer generate` created `.secrets/glint-updater.key` (private, gitignored) and its `.pub`. The **public** key is already in `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`. Add the private key to CI so releases are signed:

- GitHub repo -> Settings -> Secrets and variables -> Actions -> New secret
  - `TAURI_SIGNING_PRIVATE_KEY` = the full contents of `.secrets/glint-updater.key`
  - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` = empty (the key has no password)

Losing this key means existing installs can no longer verify updates. Keep a backup in your password manager.

### 2. License signing key

Generate the keypair that mints license keys:

```sh
cd src-tauri
cargo run --features keygen --bin glint-keygen -- new
```

It prints a PUBLIC key and a PRIVATE key.

- Put the **PUBLIC** key in CI as secret `GLINT_LICENSE_PUBKEY` (release builds embed it so the app can verify keys). For local testing, export it before building: `GLINT_LICENSE_PUBKEY=... cargo tauri dev`.
- Keep the **PRIVATE** key secret (password manager). It signs license keys; anyone with it can mint them.

Until you set a real `GLINT_LICENSE_PUBKEY`, the app uses a placeholder that rejects every key (trial still works).

### 3. Ko-fi

Create a $4.99 shop item / digital product. Delivery: link buyers to the latest GitHub Release assets (or upload the installers to Ko-fi). After a purchase, mint and email their key (see below). Update `KOFI_URL` in `src/app.js` to your Ko-fi page.

---

## Cutting a Ko-fi / direct release

1. Bump `version` in `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`.
2. Tag and push:
   ```sh
   git tag v0.1.0
   git push origin v0.1.0
   ```
3. `release.yml` builds signed installers for macOS (Intel + Apple silicon), Windows, and Linux, generates `latest.json`, and attaches everything to a **draft** GitHub Release.
4. Review the draft, then publish it. Existing installs check `releases/latest/download/latest.json` and update themselves.

## License delivery

**Automatic (recommended):** deploy the Cloudflare Worker in `licensing/`. Ko-fi calls it on each purchase and it mints + emails the key to the buyer, no manual step. See [licensing/README.md](../licensing/README.md).

**Manual (fallback / testing):**

```sh
cd src-tauri
# perpetual license
cargo run --features keygen --bin glint-keygen -- sign buyer@example.com <PRIVATE_KEY_B64>
# or time-limited (e.g. 365 days)
cargo run --features keygen --bin glint-keygen -- sign buyer@example.com <PRIVATE_KEY_B64> 365
```

Send the printed key to the buyer. They paste it into Glint when the trial ends (or anytime) to unlock it. Helpers: `glint-keygen pubkey <PRIVATE_KEY_B64>` prints the public key to embed; `glint-keygen verify <KEY> <PUBLIC_KEY_B64>` checks a key with the app's exact rules.

---

## App Store build ($2.99, no self-updater)

```sh
cd src-tauri
cargo tauri build --no-default-features --features appstore
```

This compiles out the trial, license gate, and updater. From there it's the standard Apple path: an Apple Developer account, App Store Connect app record, provisioning profile, code signing, and setting the price to the **$2.99** tier. Apple delivers updates, so no `latest.json` is involved.

---

## How the trial + license work

- First launch records a timestamp. For 7 days the app is fully usable, with a "Trial - N days left" banner and a Theme/License/Updates screen behind the gear icon (theme changes live in Settings, not on the base screen).
- After 7 days, a blocking gate asks for a license key or a Ko-fi purchase.
- A license key is an Ed25519-signed token (`payload.signature`, base64url). The app verifies it against the embedded public key. Perpetual keys have no expiry; time-limited keys carry an `exp`. Everything is offline; nothing phones home.

## Anti-piracy

- The trial start and the license are pinned in the **OS Keychain** (Keychain on macOS, Credential Manager on Windows, secret-service on Linux) **and** the config file. The trial start used is the earliest found in either store, so deleting the config file does not reset the trial - both stores would have to be wiped, and the Keychain entry is hard to find.
- License keys are Ed25519-signed, so they cannot be forged without the private key.
- What this does NOT stop: one buyer **sharing** their key with others (offline keys are inherently copyable). To prevent that you need online activation - e.g. a tiny Cloudflare Worker + KV that records device activations per key and caps them at N. That's the recommended next step if key-sharing becomes a problem; the client already sends nothing, so it's an additive change.

## Silent auto-update (no prompts, no passwords)

- On launch the direct build checks GitHub Releases and, if a newer signed build exists, downloads and installs it and relaunches - no dialogs. There's also a "Check now" button in Settings.
- **Password-free requires signing + notarization.** macOS only allows a silent in-place update when the app is signed with a Developer ID and notarized; otherwise Gatekeeper warns. Add these repo secrets so `release.yml` signs and notarizes:
  - `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`
  - (optional, Windows) `WINDOWS_CERTIFICATE`, `WINDOWS_CERTIFICATE_PASSWORD`
- Until those are set, builds are unsigned: updates still work but the OS shows a warning on first open.
