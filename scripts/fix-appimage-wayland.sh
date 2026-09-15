#!/usr/bin/env bash
# Strip bundled libwayland from a Tauri/linuxdeploy AppImage and repack it.
#
# Why: linuxdeploy bundles the build machine's libwayland-*.so (Ubuntu 22.04
# era, 1.20) into the AppImage. On modern hosts (mesa 25/26 + recent
# compositors) that stale libwayland breaks EGL display creation inside
# WebKitGTK -> the Tauri window stays black and the app dies with:
#   "Could not create default EGL display: EGL_BAD_PARAMETER. Aborting..."
# Removing the 4 bundled libs forces the host's libwayland (>= 1.22, stable
# ABI) to load, which fixes rendering. Verified on Omarchy/Arch (mesa 26,
# Hyprland) with Aethery 0.9.0.
#
# Usage:  fix-appimage-wayland.sh [--in-place] <input.AppImage>... [output.AppImage]
#   Single input without --in-place: writes <input>-nowayland.AppImage.
#   --in-place: each input is replaced by its fixed version (for CI release
#     artifacts, keeps file names stable). Accepts multiple inputs.
# Needs:  appimagetool runnable without FUSE (auto-downloaded user-space).
set -euo pipefail

INPLACE=0
if [ "${1:-}" = "--in-place" ] || [ "${1:-}" = "-i" ]; then
  INPLACE=1
  shift
fi
[ $# -ge 1 ] || { echo "usage: fix-appimage-wayland.sh [--in-place] <input.AppImage>... [output.AppImage]" >&2; exit 2; }
if [ "$INPLACE" = 0 ] && [ $# -gt 2 ]; then
  echo "multiple inputs require --in-place" >&2
  exit 2
fi
OUT="${2:-}"
if [ "$INPLACE" = 0 ] && [ $# -eq 1 ]; then
  OUT="${1%.AppImage}-nowayland.AppImage"
fi

fix_one() {
  local IN="$1" OUT="$2"
  TOOLS="$HOME/.cache/aethery-tools"
  AIT="$TOOLS/appimagetool"

  if [ ! -x "$AIT" ]; then
    mkdir -p "$TOOLS"
    echo "==> downloading appimagetool (user-space)"
    curl -sSL --max-time 300 -o "$AIT" \
      https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x "$AIT"
  fi

  WORK="$(mktemp -d)"
  trap 'rm -rf "$WORK"' EXIT

  echo "==> extracting $IN (via its own runtime, no FUSE needed)"
  (cd "$WORK" && APPIMAGE_EXTRACT_AND_RUN=1 "$IN" --appimage-extract >/dev/null)
  APPDIR="$WORK/squashfs-root"
  [ -x "$APPDIR/AppRun" ] || { echo "extraction failed: no AppRun in $APPDIR" >&2; exit 1; }

  echo "==> removing bundled libwayland (host version wins)"
  removed=0
  while IFS= read -r lib; do
    rm -f "$lib"
    removed=$((removed + 1))
    echo "    dropped $(basename "$lib")"
  done < <(find "$APPDIR" -name 'libwayland-*.so*' -not -name '*.debug')
  [ "$removed" -gt 0 ] || { echo "no bundled libwayland found, nothing to do" >&2; exit 1; }

  echo "==> repacking -> $OUT"
  export APPIMAGE_EXTRACT_AND_RUN=1
  "$AIT" "$APPDIR" "$OUT" >/dev/null
  chmod +x "$OUT"
  trap - EXIT
  rm -rf "$WORK"
  ls -lh "$OUT"
  echo "OK: $OUT ($removed wayland libs stripped)"
}

if [ "$INPLACE" = 1 ]; then
  for img in "$@"; do
    tmp="${img%.AppImage}-nowayland-tmp.AppImage"
    fix_one "$img" "$tmp"
    mv -f "$tmp" "$img"
    echo "replaced in place: $img"
  done
else
  fix_one "$1" "$OUT"
fi
