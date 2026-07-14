#!/usr/bin/env bash
#
# Package the Mac App Store build of Glint.
#
# Builds the sandboxed `appstore` variant (no updater, no trial/license gate),
# embeds the provisioning profile, signs the app with the Apple Distribution
# identity + sandbox entitlements, and produces a signed .pkg ready to upload.
#
# One-time prerequisites:
#   1. Apple Distribution and "3rd Party Mac Developer Installer" certificates
#      in your login keychain (both are already present on this machine).
#   2. A Mac App Store provisioning profile for dev.peterdsp.glint, downloaded
#      from developer.apple.com and saved as:
#          src-tauri/embedded.provisionprofile
#   3. tauri-cli available (`cargo tauri` or `tauri`).
#
# Usage:  scripts/mas-package.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_CERT="Apple Distribution: PETROS DHESPOLLARI (YTS4KJBX3P)"
PKG_CERT="3rd Party Mac Developer Installer: PETROS DHESPOLLARI (YTS4KJBX3P)"
ENTITLEMENTS="$ROOT/src-tauri/entitlements.appstore.plist"
PROFILE="$ROOT/src-tauri/embedded.provisionprofile"

if [ ! -f "$PROFILE" ]; then
  echo "Missing $PROFILE" >&2
  echo "Create a Mac App Store provisioning profile for dev.peterdsp.glint at" >&2
  echo "developer.apple.com -> Profiles, download it, and save it there." >&2
  exit 1
fi

# 1. Build the sandboxed app bundle only (the .pkg is produced below).
cd "$ROOT/src-tauri"
if command -v cargo-tauri >/dev/null 2>&1 || cargo tauri --version >/dev/null 2>&1; then
  cargo tauri build --no-default-features --features appstore \
    --config tauri.appstore.conf.json --bundles app
else
  tauri build --no-default-features --features appstore \
    --config tauri.appstore.conf.json --bundles app
fi

APP="$(ls -d "$ROOT/src-tauri/target/release/bundle/macos/"*.app | head -1)"
echo "Built app bundle: $APP"

# 2. Embed the provisioning profile.
cp "$PROFILE" "$APP/Contents/embedded.provisionprofile"

# 3. Sign nested code inside-out, then the app itself, with the sandbox
#    entitlements. Mac App Store apps are signed WITHOUT hardened runtime.
if [ -d "$APP/Contents/Frameworks" ]; then
  find "$APP/Contents/Frameworks" -type f \( -name "*.dylib" -o -perm -111 \) | while read -r f; do
    codesign --force --timestamp --sign "$APP_CERT" "$f"
  done
fi
codesign --force --timestamp --entitlements "$ENTITLEMENTS" --sign "$APP_CERT" "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"

# 4. Build the signed installer package.
PKG="$ROOT/src-tauri/target/release/bundle/Glint.pkg"
productbuild --component "$APP" /Applications --sign "$PKG_CERT" "$PKG"
echo "Packaged: $PKG"

cat <<EOF

Next: upload to App Store Connect (app id 6790709729).
With an App Store Connect API key (.p8 in ~/.appstoreconnect/private_keys/):

  xcrun altool --upload-app -t macos -f "$PKG" \\
    --apiKey <KEY_ID> --apiIssuer <ISSUER_ID>

or drag "$PKG" into Transporter.app.
EOF
