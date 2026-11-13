#!/usr/bin/env bash
# Builds a .deb package and an AppImage from a release binary.
#
# Usage: installer/package-linux.sh <version> <x64|arm64> <path to cashflow binary> <output dir>
set -euo pipefail

version=$1
arch=$2
bin=$3
out=$4

case "$arch" in
  x64)   deb_arch=amd64; tool_arch=x86_64 ;;
  arm64) deb_arch=arm64; tool_arch=aarch64 ;;
  *) echo "unknown arch: $arch" >&2; exit 1 ;;
esac

root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$out"

# --- .deb ---
deb="$work/deb"
install -Dm755 "$bin" "$deb/usr/bin/cashflow"
install -Dm644 "$root/installer/linux/cashflow.desktop" "$deb/usr/share/applications/cashflow.desktop"
install -Dm644 "$root/assets/icon.png" "$deb/usr/share/icons/hicolor/512x512/apps/cashflow.png"
mkdir -p "$deb/DEBIAN"
cat > "$deb/DEBIAN/control" <<EOF
Package: cashflow
Version: $version
Section: utils
Priority: optional
Architecture: $deb_arch
Maintainer: Sauli Anto
Depends: libc6, libfontconfig1, libxkbcommon-x11-0, libvulkan1
Homepage: https://github.com/isaul32/cashflow
Description: Monthly income and expense tracker with a Sankey flow chart
 CashFlow shows where your money flows: income sources flow into your total,
 which flows out to expense categories and savings. Data is kept in plain
 JSON files on your computer.
EOF
dpkg-deb --root-owner-group --build "$deb" "$out/cashflow-$version-linux-$arch.deb"

# --- AppImage ---
appdir="$work/CashFlow.AppDir"
install -Dm755 "$bin" "$appdir/usr/bin/cashflow"
install -Dm644 "$root/installer/linux/cashflow.desktop" "$appdir/cashflow.desktop"
install -Dm644 "$root/assets/icon.png" "$appdir/cashflow.png"
ln -s usr/bin/cashflow "$appdir/AppRun"
curl -fsSL -o "$work/appimagetool" \
  "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$tool_arch.AppImage"
chmod +x "$work/appimagetool"
# Runners have no FUSE, so let the tool unpack itself before running.
ARCH="$tool_arch" APPIMAGE_EXTRACT_AND_RUN=1 "$work/appimagetool" "$appdir" \
  "$out/cashflow-$version-linux-$arch.AppImage"
