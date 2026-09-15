import { Switch } from "@/components/ui/switch";
import { useConnectionStore } from "@/state/connectionStore";

/**
 * VPN mode toggle (Windows TUN). When on, the backend layers hev-socks5-tunnel
 * (Wintun) + a default route + DNS on top of the proxy after connect, so every
 * app (not just proxy-configured ones) goes through the tunnel.
 *
 * Launch flag — locked mid-session like the other profile controls: it
 * changes how the next connect is built (bypass routes, brings up `aether0`).
 */
export function VpnModeToggle() {
  const vpnMode = useConnectionStore((s) => s.profile.vpn_mode);
  const setVpnMode = useConnectionStore((s) => s.setVpnMode);
  const status = useConnectionStore((s) => s.status);
  const locked = status.state !== "Idle" && status.state !== "Error";

  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex items-center justify-between">
        <span className="text-xs">Route all traffic (VPN)</span>
        <Switch
          checked={vpnMode}
          onCheckedChange={setVpnMode}
          disabled={locked}
          aria-label="Route all traffic through VPN"
        />
      </div>
      <p className="text-[11px] leading-snug text-muted-foreground">
        Windows only. Creates the <span className="font-mono">aether0</span> adapter after connect
        and moves the default route + DNS onto it. Needs one Administrator (UAC) approval per
        connect/disconnect. If VPN setup fails, the proxy still connects.
      </p>
    </div>
  );
}
