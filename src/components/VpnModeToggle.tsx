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
  const tunMtu = useConnectionStore((s) => s.profile.tun_mtu);
  const setTunMtu = useConnectionStore((s) => s.setTunMtu);
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
      <label className="flex items-center justify-between gap-2">
        <span
          className="text-[11px] text-muted-foreground"
          title="TUN adapter MTU. 1500 is the default; PPPoE and most Iranian connections are stabler at 1280-1420."
        >
          TUN MTU
        </span>
        <input
          type="text"
          inputMode="numeric"
          value={String(tunMtu)}
          disabled={locked}
          onChange={(e) => {
            const v = e.target.value.replace(/\D/g, "").slice(0, 4);
            setTunMtu(v === "" ? 0 : Number(v));
          }}
          onBlur={() => {
            if (!tunMtu || tunMtu < 1280 || tunMtu > 9000) setTunMtu(1500);
          }}
          placeholder="1500"
          className="h-8 w-20 rounded-md bg-black/20 px-2 text-center font-mono text-xs text-foreground ring-1 ring-white/10 outline-none focus:ring-primary disabled:opacity-50"
          aria-label="TUN adapter MTU"
        />
      </label>
    </div>
  );
}
