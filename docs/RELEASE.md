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
cargo run --bin glint-keygen -- new
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

## Minting a license key for a buyer

```sh
cd src-tauri
# perpetual license
cargo run --bin glint-keygen -- sign buyer@example.com <PRIVATE_KEY_B64>
# or time-limited (e.g. 365 days)
cargo run --bin glint-keygen -- sign buyer@example.com <PRIVATE_KEY_B64> 365
```

Send the printed key to the buyer. They paste it into Glint when the trial ends (or anytime) to unlock it.

---

## App Store build ($2.99, no self-updater)

```sh
cd src-tauri
cargo tauri build --no-default-features --features appstore
```

This compiles out the trial, license gate, and updater. From there it's the standard Apple path: an Apple Developer account, App Store Connect app record, provisioning profile, code signing, and setting the price to the **$2.99** tier. Apple delivers updates, so no `latest.json` is involved.

---

## How the trial + license work

- First launch writes a timestamp to the app config dir (`first_run`). For 7 days the app is fully usable.
- After 7 days, a blocking gate asks for a license key or a Ko-fi purchase.
- A license key is an Ed25519-signed token (`payload.signature`, base64url). The app verifies it against the embedded public key. Perpetual keys have no expiry; time-limited keys carry an `exp`.
- The stored key lives at `<app config dir>/license.key`. Everything is offline; nothing phones home.
