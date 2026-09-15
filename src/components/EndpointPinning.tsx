import { useConnectionStore } from "@/state/connectionStore";

const INPUT =
  "h-8 w-full rounded-md bg-black/20 px-2 font-mono text-xs text-foreground ring-1 ring-white/10 outline-none focus:ring-primary disabled:opacity-50";

function EndpointInput({
  value,
  onChange,
  disabled,
  label,
  placeholder,
}: {
  value: string;
  onChange: (v: string) => void;
  disabled: boolean;
  label: string;
  placeholder: string;
}) {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-[11px] text-muted-foreground">{label}</span>
      <input
        type="text"
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(e.target.value.trim())}
        placeholder={placeholder}
        spellCheck={false}
        className={INPUT}
        aria-label={label}
      />
    </label>
  );
}

/**
 * Pinned endpoints (`ip:port`) — skip the route scan for known-good
 * addresses. Naming both hops of gool/mim disables scanning entirely;
 * naming one scans for the other. Sections follow the selected protocol:
 * a hop flag auto-selects its own protocol in the core, so each input is
 * only sent for the family it belongs to (see profiles.rs).
 */
export function EndpointPinning() {
  const profile = useConnectionStore((s) => s.profile);
  const status = useConnectionStore((s) => s.status);
  const setPeer = useConnectionStore((s) => s.setPeer);
  const setWgPeer = useConnectionStore((s) => s.setWgPeer);
  const setWiwOuter = useConnectionStore((s) => s.setWiwOuter);
  const setWiwInner = useConnectionStore((s) => s.setWiwInner);
  const setMimOuter = useConnectionStore((s) => s.setMimOuter);
  const setMimInner = useConnectionStore((s) => s.setMimInner);
  const locked = status.state !== "Idle" && status.state !== "Error";
  const protocol = profile.protocol;

  return (
    <div className="flex flex-col gap-2 rounded-md bg-black/10 p-2 ring-1 ring-white/10">
      {(protocol === "auto" || protocol === "masque" || protocol === "wireguard") && (
        <EndpointInput
          value={profile.peer}
          onChange={setPeer}
          disabled={locked}
          label="Pinned endpoint (--peer)"
          placeholder="e.g. 162.159.196.1:443"
        />
      )}
      {(protocol === "wireguard" || protocol === "gool") && (
        <EndpointInput
          value={profile.wg_peer}
          onChange={setWgPeer}
          disabled={locked}
          label="Pinned WireGuard peer (--wg-peer)"
          placeholder="e.g. 162.159.192.1:2408"
        />
      )}
      {protocol === "gool" && (
        <>
          <EndpointInput
            value={profile.wiw_outer}
            onChange={setWiwOuter}
            disabled={locked}
            label="gool outer hop (--wiw-outer)"
            placeholder="e.g. 162.159.192.1:2408"
          />
          <EndpointInput
            value={profile.wiw_inner}
            onChange={setWiwInner}
            disabled={locked}
            label="gool inner hop (--wiw-inner)"
            placeholder="e.g. 188.114.96.1:2408"
          />
        </>
      )}
      {protocol === "mim" && (
        <>
          <EndpointInput
            value={profile.mim_outer}
            onChange={setMimOuter}
            disabled={locked}
            label="mim outer hop (--mim-outer)"
            placeholder="e.g. 162.159.192.1:443"
          />
          <EndpointInput
            value={profile.mim_inner}
            onChange={setMimInner}
            disabled={locked}
            label="mim inner hop (--mim-inner)"
            placeholder="e.g. 162.159.204.1:443"
          />
        </>
      )}
      <p className="text-[10px] leading-4 text-muted-foreground">
        Empty means scan. Only <code>ip:port</code> is sent — hostnames are ignored.
      </p>
    </div>
  );
}
