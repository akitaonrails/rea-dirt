#!/bin/sh
# packaging/package.sh [--no-appimage] <package-folder>
# Builds the zip folder (e.g. dist/DiRTLauncher-v0.1.0/), its .zip beside it,
# and the AppImage (portable Wine, sdk/packaging/appimage-wine.sh) in the
# same parent folder. CI calls it from launcher-ci.yml's package-command
# (rea-launcher-sdk docs/playbooks/ci-and-releases.md).
#
# Environment: REA_RELEASE_DIR holds DiRTLauncher.exe (CI sets it; default
# target/i686-pc-windows-gnu/release); REA_CACHE caches pinned downloads.
# Everything the folder holds must be listed in packaging/package.allow.
set -eu
ROOT=$(cd "$(dirname "$0")/.." && pwd -P)
appimage=1
if [ "${1:-}" = --no-appimage ]; then appimage='' && shift; fi
[ $# -eq 1 ] || { echo "usage: $0 [--no-appimage] <package-folder>" >&2; exit 2; }
pkg=$1
name=$(basename "$pkg")
case $name in '' | *[!A-Za-z0-9._-]*) echo "package: bad package name '$name'" >&2; exit 2 ;; esac
dist=$(dirname "$pkg")
release=${REA_RELEASE_DIR:-$ROOT/target/i686-pc-windows-gnu/release}
[ -f "$release/DiRTLauncher.exe" ] || { echo "package: no $release/DiRTLauncher.exe (cargo build --release --target i686-pc-windows-gnu)" >&2; exit 1; }

rm -rf "$pkg"
mkdir -p "$pkg/linux" "$pkg/licenses"
cp "$release/DiRTLauncher.exe" "$pkg/"
cp "$ROOT/README.md" "$ROOT/THIRD_PARTY.md" "$ROOT/CHANGELOG.md" \
  "$ROOT/LICENSE-MIT" "$ROOT/LICENSE-APACHE" "$pkg/"
cp "$ROOT/linux/dirt-play" "$ROOT/linux/README.md" "$pkg/linux/"
# The helper finds the SDK play library beside itself.
cp "$ROOT/sdk/rea-play.sh" "$pkg/linux/rea-play.sh"
chmod +x "$pkg/linux/dirt-play"
cp "$ROOT"/packaging/licenses/* "$pkg/licenses/"

# DXVK for Linux, from its pins (verified sha256, mirror fallback, cached).
# REA_NO_FETCH=1 (packaging/tests) leaves it out: the test runs offline.
if [ -z "${REA_NO_FETCH:-}" ]; then
  work=$(mktemp -d)
  trap 'rm -rf "$work"' EXIT
  . "$ROOT/sdk/packaging/pins.sh"
  PIN_FILE=$ROOT/packaging/third-party.txt
  if [ -n "${REA_CACHE:-}" ]; then PIN_CACHE=$REA_CACHE/pins; fi
  tarball=$(pin_fetch dxvk "$work")
  top=$(basename "$tarball" .tar.gz)
  tar -xOf "$tarball" "$top/x32/d3d9.dll" > "$pkg/linux/d3d9.dll"
  cp "$(pin_fetch dxvk-license "$work")" "$pkg/linux/dxvk-LICENSE"
fi

(cd "$dist" && rm -f "$name.zip" && zip -qr "$name.zip" "$name")

if [ -n "$appimage" ]; then
  sh "$ROOT/sdk/packaging/appimage-wine.sh" --app dirt --exe DiRTLauncher.exe \
    --env-prefix DIRT \
    --icon "$ROOT/packaging/appimage/dirt-launcher.png" \
    --desktop "$ROOT/packaging/appimage/dirt-launcher.desktop" \
    --payload linux/d3d9.dll --doc linux/dxvk-LICENSE --doc licenses \
    ${REA_CACHE:+--cache "$REA_CACHE"} \
    --name "$name" "$pkg" "$dist"
fi
