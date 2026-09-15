#!/usr/bin/env sh
# Tear down the Aethery VPN: stop hev, remove the TUN route/rule, restore DNS.
#
# Runs as ROOT via `pkexec sh tun-down.sh …`. Idempotent and safe when
# already down (plain `ip` failures are tolerated, never fatal).
#
# Args: STATE_DIR TUN_NAME FWMARK
set -eu

STATE_DIR="$1"; TUN_NAME="$2"; FWMARK="$3"

PID_FILE="$STATE_DIR/hev.pid"
BACKUP_DIR="$STATE_DIR/net-backup"
RESOLV_BACKUP="$BACKUP_DIR/resolv.conf"
RESOLV_LINK_BACKUP="$BACKUP_DIR/resolv.conf.link"
RESOLV="/etc/resolv.conf"

log() { echo "[tun-down] $*"; }

if [ -f "$PID_FILE" ]; then
  PID="$(cat "$PID_FILE")"
  if kill -0 "$PID" 2>/dev/null; then
    log "stopping hev (pid $PID)"
    kill "$PID" 2>/dev/null || true
    sleep 1
    kill -9 "$PID" 2>/dev/null || true
  fi
  rm -f "$PID_FILE"
fi

ip route del default dev "$TUN_NAME" metric 5 2>/dev/null || true
log "TUN default route removed"

ip rule del fwmark "$FWMARK" table main priority 100 2>/dev/null || true
log "fwmark $FWMARK rule removed"

# Restore the resolver: symlink first (systemd-resolved stub setups),
# else plain file content.
if [ -f "$RESOLV_LINK_BACKUP" ]; then
  TARGET="$(cat "$RESOLV_LINK_BACKUP")"
  rm -f "$RESOLV"
  ln -s "$TARGET" "$RESOLV"
  rm -f "$RESOLV_LINK_BACKUP" "$RESOLV_BACKUP"
  log "resolv.conf symlink restored (-> $TARGET)"
elif [ -f "$RESOLV_BACKUP" ]; then
  cat "$RESOLV_BACKUP" >"$RESOLV"
  rm -f "$RESOLV_BACKUP"
  log "resolv.conf restored"
fi

if ip link show "$TUN_NAME" >/dev/null 2>&1; then
  ip link del "$TUN_NAME" 2>/dev/null || true
  log "interface $TUN_NAME removed"
fi

log "DOWN"
