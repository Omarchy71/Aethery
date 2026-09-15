#!/usr/bin/env bash
# Downloads the pinned hev-socks5-tunnel release binary (tun2socks for VPN
# mode) into src-tauri/binaries/hev. Mirrors fetch-aether.sh conventions:
# direct binary asset, executable bit, no archives to clean up.
set -euo pipefail

HEV_VERSION="2.17.1"
REPO="heiher/hev-socks5-tunnel"
DEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)   ASSET="hev-socks5-tunnel-linux-x86_64" ;;
  Linux-aarch64)  ASSET="hev-socks5-tunnel-linux-arm64" ;;
  Darwin-x86_64)  ASSET="hev-socks5-tunnel-darwin-x86_64" ;;
  Darwin-arm64)   ASSET="hev-socks5-tunnel-darwin-arm64" ;;
  *) echo "Unsupported platform for hev-socks5-tunnel: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

URL="https://github.com/${REPO}/releases/download/${HEV_VERSION}/${ASSET}"

cd "$DEST_DIR"
curl -sL -o hev "$URL"
chmod +x hev

if [ ! -s hev ] || [ "$(wc -c <hev)" -lt 100000 ]; then
  echo "Downloaded hev binary looks wrong (too small)" >&2
  rm -f hev
  exit 1
fi
echo "hev-socks5-tunnel $HEV_VERSION ready at $DEST_DIR/hev"
