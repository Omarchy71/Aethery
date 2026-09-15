import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { useConnectionStore } from "@/state/connectionStore";

const INPUT =
  "h-8 w-full rounded-md bg-black/20 px-2 font-mono text-xs text-foreground ring-1 ring-white/10 outline-none focus:ring-primary disabled:opacity-50";

/**
 * MASQUE evasion beyond the noize profile: TLS ClientHello fragmentation on
 * the HTTP/2 carrier (--fragment) and Encrypted Client Hello (--ech) hide
 * the SNI from DPI. MASQUE-family protocols only; the backend drops these
 * flags for WireGuard/gool.
 */
export function MasqueEvasionSettings() {
  const profile = useConnectionStore((s) => s.profile);
  const status = useConnectionStore((s) => s.status);
  const setFragment = useConnectionStore((s) => s.setFragment);
  const setFragmentSize = useConnectionStore((s) => s.setFragmentSize);
  const setFragmentDelay = useConnectionStore((s) => s.setFragmentDelay);
  const setEch = useConnectionStore((s) => s.setEch);
  const locked = status.state !== "Idle" && status.state !== "Error";
  const masqueFamily =
    profile.protocol === "auto" || profile.protocol === "masque" || profile.protocol === "mim";
  const disabled = locked || !masqueFamily;
  const echMode = profile.ech === "" ? "off" : profile.ech === "auto" ? "auto" : "custom";

  return (
    <div className="flex flex-col gap-2 rounded-md bg-black/10 p-2 ring-1 ring-white/10">
      <div className="flex items-center justify-between">
        <span className="text-xs">Fragment ClientHello (--fragment)</span>
        <Switch
          checked={profile.fragment}
          onCheckedChange={setFragment}
          disabled={disabled}
          aria-label="Fragment TLS ClientHello"
        />
      </div>
      {profile.fragment && (
        <div className="flex gap-2">
          <label className="flex flex-1 flex-col gap-1">
            <span className="text-[11px] text-muted-foreground">Chunk size (default 16-32)</span>
            <input
              type="text"
              value={profile.fragment_size}
              disabled={disabled}
              onChange={(e) => setFragmentSize(e.target.value.trim())}
              placeholder="16-32"
              spellCheck={false}
              className={INPUT}
              aria-label="Fragment chunk size"
            />
          </label>
          <label className="flex flex-1 flex-col gap-1">
            <span className="text-[11px] text-muted-foreground">Delay ms (default 2-10)</span>
            <input
              type="text"
              value={profile.fragment_delay}
              disabled={disabled}
              onChange={(e) => setFragmentDelay(e.target.value.trim())}
              placeholder="2-10"
              spellCheck={false}
              className={INPUT}
              aria-label="Fragment delay"
            />
          </label>
        </div>
      )}
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs">Encrypted ClientHello</span>
        <Select
          value={echMode}
          onValueChange={(v) => {
            if (v === "off") setEch("");
            else if (v === "auto") setEch("auto");
            else setEch(profile.ech === "" || profile.ech === "auto" ? "" : profile.ech);
          }}
          disabled={disabled}
        >
          <SelectTrigger size="sm" className="w-28" aria-label="ECH mode">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="off">Off</SelectItem>
            <SelectItem value="auto">Auto</SelectItem>
            <SelectItem value="custom">Custom</SelectItem>
          </SelectContent>
        </Select>
      </div>
      {echMode === "custom" && (
        <input
          type="text"
          value={profile.ech === "auto" ? "" : profile.ech}
          disabled={disabled}
          onChange={(e) => setEch(e.target.value.trim())}
          placeholder="Paste base64 ECH config"
          spellCheck={false}
          className={INPUT}
          aria-label="Custom ECH config"
        />
      )}
      <p className="text-[10px] leading-4 text-muted-foreground">
        Fragment needs the HTTP/2 transport. {masqueFamily ? "" : "Select a MASQUE protocol to use these."}
      </p>
    </div>
  );
}
