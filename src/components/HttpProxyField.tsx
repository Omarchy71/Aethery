import { useConnectionStore } from "@/state/connectionStore";
import { Switch } from "@/components/ui/switch";

const DEFAULT_PORT = "1820";
const LOOPBACK = "127.0.0.1";
const ANY = "0.0.0.0";

function splitAddr(addr: string): { host: string; port: string } {
  const last = addr.lastIndexOf(":");
  if (last === -1) return { host: LOOPBACK, port: addr || DEFAULT_PORT };
  return { host: addr.slice(0, last) || LOOPBACK, port: addr.slice(last + 1) || DEFAULT_PORT };
}

export function HttpProxyField() {
  const enabled = useConnectionStore((s) => s.profile.http_proxy_enabled);
  const addr = useConnectionStore((s) => s.profile.http_proxy_address);
  const setEnabled = useConnectionStore((s) => s.setHttpProxyEnabled);
  const setAddr = useConnectionStore((s) => s.setHttpProxyAddress);
  const status = useConnectionStore((s) => s.status);
  const locked = status.state !== "Idle" && status.state !== "Error";

  const { host, port } = splitAddr(addr);
  const lan = host === ANY;

  const rebuild = (h: string, p: string) =>
    setAddr(`${h}:${p || DEFAULT_PORT}`);

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <span className="text-xs text-muted-foreground">Enable HTTP/HTTPS proxy</span>
        <Switch
          checked={enabled}
          onCheckedChange={setEnabled}
          disabled={locked}
          aria-label="Enable HTTP/HTTPS proxy"
        />
      </div>
      <div className="flex items-center justify-between">
        <input
          type="text"
          inputMode="numeric"
          value={port}
          disabled={locked || !enabled}
          onChange={(e) => {
            const v = e.target.value.replace(/\D/g, "").slice(0, 5);
            rebuild(host, v);
          }}
          onBlur={() => {
            const n = Number(port);
            if (!port || n < 1 || n > 65535) rebuild(host, DEFAULT_PORT);
          }}
          className="h-8 w-20 rounded-md bg-black/20 px-2 text-center text-xs text-foreground ring-1 ring-white/10 outline-none focus:ring-primary disabled:opacity-50"
          aria-label="HTTP proxy port"
        />
        <div className="flex items-center gap-1.5">
          <span className="text-xs text-muted-foreground">Allow connections from the LAN</span>
          <Switch
            checked={lan}
            onCheckedChange={(on) => rebuild(on ? ANY : LOOPBACK, port)}
            disabled={locked || !enabled}
            aria-label="Allow HTTP proxy connections from the LAN"
          />
        </div>
      </div>
    </div>
  );
}
