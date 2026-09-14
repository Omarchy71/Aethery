import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Switch } from "@/components/ui/switch";
import { useConnectionStore } from "@/state/connectionStore";

/**
 * OS autostart + auto-connect + start-minimized toggles. Deliberately NOT
 * locked mid-session like the tunnel options above: flipping them changes
 * nothing about the running connection, only what happens on next boot /
 * launch.
 */
export function StartupSettings() {
  const autostart = useConnectionStore((s) => s.profile.autostart);
  const autoConnect = useConnectionStore((s) => s.profile.auto_connect);
  const setAutostart = useConnectionStore((s) => s.setAutostart);
  const setAutoConnect = useConnectionStore((s) => s.setAutoConnect);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [startMinimized, setStartMinimized] = useState(false);

  const onAutostart = async (next: boolean) => {
    setBusy(true);
    setError(null);
    try {
      await setAutostart(next);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    invoke<boolean>("get_start_minimized")
      .then((v) => setStartMinimized(v))
      .catch(() => {});
  }, []);

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs">Start on system boot</span>
        <Switch
          checked={autostart}
          onCheckedChange={(v) => void onAutostart(v)}
          disabled={busy}
          aria-label="Start on system boot"
        />
      </div>
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs">Auto-connect on launch</span>
        <Switch
          checked={autoConnect}
          onCheckedChange={setAutoConnect}
          aria-label="Auto-connect on launch"
        />
      </div>
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs">Start minimized</span>
        <Switch
          checked={startMinimized}
          onCheckedChange={(v) => {
            setStartMinimized(v);
            void invoke("set_start_minimized", { enabled: v });
          }}
          aria-label="Start minimized (open into the taskbar)"
        />
      </div>
      {error && <p className="text-xs text-status-error">{error}</p>}
    </div>
  );
}
