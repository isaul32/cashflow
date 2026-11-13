#!/usr/bin/env bash
# Builds an unsigned (ad-hoc signed) CashFlow.app and wraps it in a .dmg.
#
# Usage: installer/package-macos.sh <version> <path to cashflow binary> <output dir>
# Run on macOS (uses sips, iconutil, codesign and hdiutil).
set -euo pipefail

version=$1
bin=$2
out=$3

root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$out"

app="$work/stage/CashFlow.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$bin" "$app/Contents/MacOS/cashflow"
chmod +x "$app/Contents/MacOS/cashflow"
sed "s/@VERSION@/$version/g" "$root/installer/macos/Info.plist.in" > "$app/Contents/Info.plist"

# Icon: build an .icns from the 512 px PNG.
iconset="$work/icon.iconset"
mkdir -p "$iconset"
for s in 16 32 64 128 256 512; do
  sips -z "$s" "$s" "$root/assets/icon.png" --out "$iconset/icon_${s}x${s}.png" > /dev/null
done
cp "$iconset/icon_32x32.png" "$iconset/icon_16x16@2x.png"
cp "$iconset/icon_64x64.png" "$iconset/icon_32x32@2x.png"
cp "$iconset/icon_256x256.png" "$iconset/icon_128x128@2x.png"
cp "$iconset/icon_512x512.png" "$iconset/icon_256x256@2x.png"
rm "$iconset/icon_64x64.png"
iconutil -c icns "$iconset" -o "$app/Contents/Resources/icon.icns"

# Apple Silicon refuses to run code without any signature; an ad-hoc one is enough
# (users still have to approve the app once, see the website).
codesign --force --deep --sign - "$app"

ln -s /Applications "$work/stage/Applications"
hdiutil create -volname "CashFlow" -srcfolder "$work/stage" -ov -format UDZO \
  "$out/cashflow-$version-macos-arm64.dmg"
