#!/usr/bin/env bash
# Builds the macOS CLI binaries on this Mac (Apple Silicon + Intel), signs + notarizes them, packs
# them exactly like CI does (.github/workflows/release.yml) and uploads them to the GitHub release,
# replacing CI's unsigned zips (same file names, --clobber).
#
#   scripts/release-mac.sh              # version from Cargo.toml → release v<version>
#   scripts/release-mac.sh --no-upload  # only build, sign, notarize, zip (into dist-mac/)
#
# One-time setup (same as GIFmeta and Flixlar): "Developer ID Application" certificate in the
# login keychain, notarization credentials
# (`xcrun notarytool store-credentials gifmeta-notary --apple-id <id> --team-id 24Z4BBBVR5`)
# and `gh auth login`. A bare binary can't be stapled: macOS checks notarization online on first run.
set -euo pipefail
cd "$(dirname "$0")/.."

: "${APPLE_SIGNING_IDENTITY:=Developer ID Application: Mads Paaskesen (24Z4BBBVR5)}"
: "${NOTARY_PROFILE:=gifmeta-notary}"
UPLOAD=1
[[ "${1:-}" == "--no-upload" ]] && UPLOAD=0

CRATE=$(sed -n 's/^name = "\(.*\)"/\1/p' Cargo.toml | head -1)
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
TAG="v$VERSION"
OUT=dist-mac
rm -rf "$OUT" && mkdir -p "$OUT"
echo "🔏 $CRATE $TAG, signing as: $APPLE_SIGNING_IDENTITY"
security find-identity -v -p codesigning | grep -q "$APPLE_SIGNING_IDENTITY" ||
  { echo "❌ Certificate not found in the keychain: $APPLE_SIGNING_IDENTITY"; exit 1; }
# Fail fast (before a long build) when the notarization credentials are missing.
xcrun notarytool history --keychain-profile "$NOTARY_PROFILE" >/dev/null 2>&1 || {
  echo "❌ No notarization profile '$NOTARY_PROFILE' in the keychain. Create it once with:"
  echo "   xcrun notarytool store-credentials $NOTARY_PROFILE --apple-id <apple-id> --team-id 24Z4BBBVR5"
  exit 1
}

FILES=()
for PAIR in aarch64-apple-darwin:macos-arm64 x86_64-apple-darwin:macos-x86_64; do
  TARGET=${PAIR%%:*}
  NAME=${PAIR##*:}
  echo "▶ Building $TARGET"
  rustup target add "$TARGET" >/dev/null
  LOCKED=(); [ -f Cargo.lock ] && LOCKED=(--locked)
  cargo build --release ${LOCKED[@]+"${LOCKED[@]}"} --target "$TARGET"
  BIN=target/$TARGET/release/$CRATE

  echo "▶ Signing + notarizing $NAME (takes a few minutes)"
  codesign --force --sign "$APPLE_SIGNING_IDENTITY" --options runtime --timestamp "$BIN"
  codesign --verify --verbose "$BIN"
  ditto -c -k --keepParent "$BIN" "$OUT/notarize-$NAME.zip"
  xcrun notarytool submit "$OUT/notarize-$NAME.zip" --keychain-profile "$NOTARY_PROFILE" --wait
  rm "$OUT/notarize-$NAME.zip"

  # Same layout + name as .github/workflows/release.yml
  DIR="release-$NAME"
  rm -rf "$OUT/$DIR" && mkdir -p "$OUT/$DIR"
  cp README.md LICENSE CHANGELOG.md "$OUT/$DIR/"
  [ -f SUPPORT.md ] && cp SUPPORT.md "$OUT/$DIR/"
  cp -r examples "$OUT/$DIR/"
  cp "$BIN" "$OUT/$DIR/"
  ARCHIVE="$CRATE-$TAG-$NAME.zip"
  (cd "$OUT" && zip -qr "$ARCHIVE" "$DIR")
  FILES+=("$OUT/$ARCHIVE")
done

if [[ $UPLOAD == 1 ]]; then
  # CI's release build uploads files with the same names: if it is still running it would
  # overwrite the signed ones afterwards.
  CI_STATUS=$(gh run list --workflow release.yml --limit 1 --json status -q '.[0].status' 2>/dev/null || echo unknown)
  if [[ "$CI_STATUS" != "completed" ]]; then
    echo "⏳ The release workflow is '$CI_STATUS'. Wait until it is done, then run:"
    echo "   gh release upload $TAG ${FILES[*]} --clobber"
    exit 1
  fi
  echo "▶ Uploading to release $TAG (replaces CI's unsigned zips)"
  gh release upload "$TAG" "${FILES[@]}" --clobber
fi
echo "✅ Done:"; printf '   %s\n' "${FILES[@]}"
