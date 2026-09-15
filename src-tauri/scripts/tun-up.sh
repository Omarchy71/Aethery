#!/usr/bin/env sh
# Bring up the Aethery VPN: start hev-socks5-tunnel, move the default route
# and DNS onto its TUN interface.
#
# Runs as ROOT via `pkexec sh tun-up.sh …` (one polkit prompt per connect).
# Idempotent: safe to re-run when already up.
#
# Args: STATE_DIR HEV_BIN HEV_CONFIG TUN_NAME FWMARK DNS_CSV
set -eu

STATE_DIR="$1"; HEV_BIN="$2"; HEV_CONFIG="$3"
TUN_NAME="$4";  FWMARK="$5";  DNS_CSV="$6"

PID_FILE="$STATE_DIR/hev.pid"
BACKUP_DIR="$STATE_DIR/net-backup"
ROUTE_BACKUP="$BACKUP_DIR/default.route"
RESOLV_BACKUP="$BACKUP_DIR/resolv.conf"
RESOLV_LINK_BACKUP="$BACKUP_DIR/resolv.conf.link"
RESOLV="/etc/resolv.conf"

log() { echo "[tun-up] $*"; }

# Already up? (hev alive AND interface present) — nothing to do.
if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null \
   && ip link show "$TUN_NAME" >/dev/null 2>&1; then
  log "$TUN_NAME already up, nothing to do"
  exit 0
fi

command -v ip >/dev/null || { log "ERROR: iproute2 (ip) not found"; exit 1; }
[ -x "$HEV_BIN" ] || { log "ERROR: hev binary not executable: $HEV_BIN"; exit 1; }
[ -f "$HEV_CONFIG" ] || { log "ERROR: hev config missing: $HEV_CONFIG"; exit 1; }

mkdir -p "$BACKUP_DIR"

# Save the current default route once (fallback when VPN goes down).
if [ ! -f "$ROUTE_BACKUP" ]; then
  ip route show default >"$ROUTE_BACKUP" 2>/dev/null || true
fi
# Back up the resolver once: regular file content AND symlink target.
if [ ! -f "$RESOLV_BACKUP" ] && [ ! -f "$RESOLV_LINK_BACKUP" ]; then
  if [ -L "$RESOLV" ]; then
    readlink "$RESOLV" >"$RESOLV_LINK_BACKUP"
    cp -L "$RESOLV" "$RESOLV_BACKUP" 2>/dev/null || true
  else
    cp "$RESOLV" "$RESOLV_BACKUP" 2>/dev/null || true
  fi
fi

# Kill a stale hev from a crashed session, if any.
if [ -f "$PID_FILE" ]; then
  OLD_PID="$(cat "$PID_FILE")"
  if kill -0 "$OLD_PID" 2>/dev/null; then
    log "stopping stale hev (pid $OLD_PID)"
    kill "$OLD_PID" 2>/dev/null || true
    sleep 1
    kill -9 "$OLD_PID" 2>/dev/null || true
  fi
  rm -f "$PID_FILE"
fi

log "starting hev-socks5-tunnel ($HEV_CONFIG)"
nohup "$HEV_BIN" "$HEV_CONFIG" >>"$STATE_DIR/hev.log" 2>&1 &
echo "$!" >"$PID_FILE"

# Wait for the kernel interface hev creates.
i=0
while ! ip link show "$TUN_NAME" >/dev/null 2>&1; do
  i=$((i + 1))
  if [ "$i" -ge 50 ]; then
    log "ERROR: $TUN_NAME did not appear after 10s (see $STATE_DIR/hev.log)"
    kill "$(cat "$PID_FILE")" 2>/dev/null || true
    rm -f "$PID_FILE"
    exit 1
  fi
  sleep 0.2
done
log "interface $TUN_NAME ready"

# Marked packets (Aether's own sockets via --mark, hev's SOCKS dial-out)
# stay on the main table instead of looping back into the TUN.
if ! ip rule show | grep -q "fwmark $FWMARK"; then
  ip rule add fwmark "$FWMARK" table main priority 100
  log "fwmark $FWMARK bypass rule added"
fi

# Default via TUN with a winning metric; the original default stays as
# fallback (higher metric) so teardown is just a delete.
if ! ip route show default | grep -q "dev $TUN_NAME"; then
  ip route add default dev "$TUN_NAME" metric 5 2>/dev/null \
    || ip route replace default dev "$TUN_NAME" metric 5
  log "default route via $TUN_NAME (metric 5)"
fi

# Point the system resolver into the tunnel (first two entries of DNS_CSV).
DNS1="$(echo "$DNS_CSV" | cut -d, -f1 | tr -d ' ')"
DNS2="$(echo "$DNS_CSV" | cut -d, -f2 -s | tr -d ' ')"
[ -n "$DNS1" ] || DNS1="1.1.1.1"
{
  echo "# Aethery VPN — restored on disconnect"
  echo "nameserver $DNS1"
  [ -n "$DNS2" ] && echo "nameserver $DNS2"
} >"$RESOLV"
log "DNS -> $DNS1${DNS2:+, $DNS2}"

log "UP ($TUN_NAME)"
