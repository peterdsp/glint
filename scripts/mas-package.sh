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
# Signing identities and profile default to the local setup, but CI overrides
# them via env (MAS_APP / MAS_INST / PROFILE) after importing certs into a
# temporary keychain. Same script runs locally and in the release workflow.
APP_CERT="${MAS_APP:-Apple Distribution: PETROS DHESPOLLARI (YTS4KJBX3P)}"
PKG_CERT="${MAS_INST:-3rd Party Mac Developer Installer: PETROS DHESPOLLARI (YTS4KJBX3P)}"
ENTITLEMENTS="$ROOT/src-tauri/entitlements.appstore.plist"
PROFILE="${PROFILE:-$ROOT/src-tauri/embedded.provisionprofile}"

if [ ! -f "$PROFILE" ]; then
  echo "Missing $PROFILE" >&2
  echo "Create a Mac App Store provisioning profile for dev.peterdsp.glint at" >&2
  echo "developer.apple.com -> Profiles, download it, and save it there." >&2
  exit 1
fi

# 1. Build the sandboxed app bundle (the .pkg is produced below). Builds for
#    the host arch (arm64 on Apple silicon runners/machines). To ship a
#    universal build later, add `--target universal-apple-darwin` here once the
#    x86_64 toolchain is in place.
cd "$ROOT/src-tauri"
TAURI="cargo tauri"
command -v cargo-tauri >/dev/null 2>&1 || cargo tauri --version >/dev/null 2>&1 || TAURI="tauri"
$TAURI build --config tauri.appstore.conf.json --bundles app \
  -- --no-default-features --features appstore

APP="$(ls -d "$ROOT/src-tauri/target/release/bundle/macos/"*.app 2>/dev/null | head -1)"
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
mkdir -p "$(dirname "$PKG")"
productbuild --component "$APP" /Applications --sign "$PKG_CERT" "$PKG"
echo "Packaged: $PKG"

cat <<EOF

Next: upload to App Store Connect (app id 6790709729).
With an App Store Connect API key (.p8 in ~/.appstoreconnect/private_keys/):

  xcrun altool --upload-app -t macos -f "$PKG" \\
    --apiKey <KEY_ID> --apiIssuer <ISSUER_ID>

or drag "$PKG" into Transporter.app.
EOF
