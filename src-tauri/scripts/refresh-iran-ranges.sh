#!/usr/bin/env bash
# Refresh the embedded Iranian prefix lists from the daily-aggregated source.
# Run from the repo root:  bash src-tauri/scripts/refresh-iran-ranges.sh
set -euo pipefail

BASE="https://github.com/Cod3ByAmir/iran-ip-ranges/releases/latest/download"
OUT="src-tauri/data"

curl -sSL --max-time 120 "$BASE/iran-ipv4.txt" -o "$OUT/iran-v4.txt"
curl -sSL --max-time 120 "$BASE/iran-ipv6.txt" -o "$OUT/iran-v6.txt"

# Validate: every non-empty line must be a CIDR.
python3 - "$OUT/iran-v4.txt" "$OUT/iran-v6.txt" <<'EOF'
import ipaddress, sys
total = 0
for path in sys.argv[1:]:
    for line in open(path):
        line = line.strip()
        if not line:
            continue
        ipaddress.ip_network(line)  # raises on junk
        total += 1
print(f"OK: {total} prefixes")
EOF
