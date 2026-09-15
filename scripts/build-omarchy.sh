#!/usr/bin/env bash
# Native Omarchy / Arch Linux build for Aethery.
# Produces install-free AppImage + deb bundles tuned for this machine's
# toolchain (no sudo needed — everything runs user-space except the
# pre-installed pacman system libs).
#
# Usage:  npm run build:omarchy
# Output: src-tauri/target/release/bundle/{appimage,deb}/
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

echo "==> 1/4 Checking system dependencies (pacman, no install)"
missing=()
for pkg in webkit2gtk-4.1 gtk3 librsvg openssl; do
  pacman -Q "$pkg" >/dev/null 2>&1 || missing+=("$pkg")
done
if ((${#missing[@]})); then
  echo "Missing system packages: ${missing[*]}" >&2
  echo "Ask your admin to run: pacman -S --needed ${missing[*]} libappindicator-gtk3" >&2
  exit 1
fi
command -v cargo >/dev/null || { echo "cargo not found (rustup.rs)" >&2; exit 1; }
command -v npm >/dev/null || { echo "npm not found" >&2; exit 1; }
echo "    system deps OK"

echo "==> 2/4 Fetching pinned Aether core (checksum-verified)"
bash src-tauri/binaries/fetch-aether.sh

echo "==> 2b/4 Fetching pinned hev-socks5-tunnel (Linux VPN tun2socks)"
bash src-tauri/binaries/fetch-hev.sh

echo "==> 2b/4 Ensuring real linuxdeploy AppImages (no FUSE needed)"
# Tauri runs linuxdeploy with APPIMAGE_EXTRACT_AND_RUN=1, which self-extracts
# without FUSE — but its bundler also zeroes 3 magic bytes (seek=8) of the
# cached tool on every run. That patch is harmless on a real AppImage yet
# destroys the python-shim workaround some setups use (shebang becomes
# #!/usr/b<NUL><NUL><NUL>env python3 -> "Exec format error"). So make sure
# the cache holds the real AppImages, restoring from .orig backups if needed.
# Also drop *.bak files here: linuxdeploy globs linuxdeploy-plugin-* in this
# dir and will happily execute a stale backup instead of the real plugin.
rm -f ~/.cache/tauri/*.bak
python3 - <<'EOF'
import os, shutil
cache = os.path.expanduser("~/.cache/tauri")
for name in ("linuxdeploy-x86_64.AppImage", "linuxdeploy-plugin-appimage.AppImage"):
    p = os.path.join(cache, name)
    orig = p + ".orig"
    if not os.path.exists(p):
        print(f"    {name} missing (bundler will download)")
        continue
    with open(p, "rb") as f:
        head = f.read(4)
    size = os.path.getsize(p)
    if head != b"\x7fELF" or size < 1_000_000:
        if os.path.exists(orig):
            shutil.copy2(orig, p)
            os.chmod(p, 0o755)
            print(f"    restored real {name} from .orig")
        else:
            os.remove(p)
            print(f"    removed shim {name} (bundler will re-download)")
    else:
        print(f"    {name} OK")
EOF

echo "==> 2c/4 Tolerating Arch's loader-less gdk-pixbuf in gtk plugin"
# Arch's gdk-pixbuf2 ships no /usr/lib/gdk-pixbuf-2.0/2.10.0 dir (core
# loaders are built into the lib), which makes linuxdeploy-plugin-gtk.sh
# abort on `cp: cannot stat ...`. Patch the cached copy once (marker
# comment); upstream re-downloads only when the file is missing.
python3 - <<'EOF'
import os
p = os.path.expanduser("~/.cache/tauri/linuxdeploy-plugin-gtk.sh")
if not os.path.exists(p):
    print("    plugin not cached yet (bundler will download)")
else:
    src = open(p).read()
    if "skipping pixbuf loader copy" in src:
        print("    gtk plugin already patched")
    else:
        src = src.replace(
            'copy_tree "$gdk_pixbuf_binarydir" "$APPDIR/"',
            'if [ -e "$gdk_pixbuf_binarydir" ]; then\n'
            '    copy_tree "$gdk_pixbuf_binarydir" "$APPDIR/"\n'
            'else\n'
            '    echo "WARNING: $gdk_pixbuf_binarydir not found, skipping pixbuf loader copy"\n'
            'fi',
        ).replace(
            'sed -i "s|$gdk_pixbuf_moduledir/||g" "$APPDIR/$gdk_pixbuf_cache_file"',
            'if [ -f "$APPDIR/$gdk_pixbuf_cache_file" ]; then\n'
            '    sed -i "s|$gdk_pixbuf_moduledir/||g" "$APPDIR/$gdk_pixbuf_cache_file"\n'
            'fi',
        ).replace(
            'echo "Updating pixbuf cache in $APPDIR/$gdk_pixbuf_cache_file"\n',
            'echo "Updating pixbuf cache in $APPDIR/$gdk_pixbuf_cache_file"\n'
            '    mkdir -p "$(dirname "$APPDIR/$gdk_pixbuf_cache_file")"\n',
            1,
        )
        open(p, "w").write(src)
        print("    patched gtk plugin for loader-less gdk-pixbuf")
EOF

echo "==> 3/4 Frontend: typecheck + build"
npm ci --no-audit --no-fund
npm run typecheck
npm run build

echo "==> 4/4 Tauri bundle (appimage, deb)"
# NO_STRIP: Arch ships libraries with RELR relocations (.relr.dyn) that the
# binutils `strip` bundled inside linuxdeploy (2024) cannot parse — without
# this the AppImage bundle aborts on libzstd. Skipping strip only makes the
# AppImage slightly larger.
export NO_STRIP=1
npx tauri build --bundles appimage,deb

echo ""
echo "==> 5/4 Strip bundled libwayland from AppImage (black-window fix)"
# linuxdeploy packs the build host's libwayland (Ubuntu 22.04 era) which
# breaks WebKit EGL on modern hosts (black window + EGL_BAD_PARAMETER).
# See scripts/fix-appimage-wayland.sh for the full story.
for img in src-tauri/target/release/bundle/appimage/*.AppImage; do
  [ -e "$img" ] || continue
  bash scripts/fix-appimage-wayland.sh --in-place "$img"
done

echo ""
echo "Done. Bundles:"
ls -la src-tauri/target/release/bundle/appimage/ src-tauri/target/release/bundle/deb/ 2>/dev/null
