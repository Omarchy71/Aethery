import { useConnectionStore } from "@/state/connectionStore";

const PROTOCOL_LABEL: Record<string, string> = {
  masque: "MASQUE",
  wireguard: "WireGuard",
  gool: "gool",
  mim: "mim",
};

/**
 * One-glance pills for every non-default tunnel option, so the effective
 * setup is visible without opening Advanced. Read-only; everything is
 * edited where it lives. Renders nothing when the profile is stock.
 */
export function ActiveConfigChips() {
  const profile = useConnectionStore((s) => s.profile);
  const chips: string[] = [];

  if (profile.protocol !== "auto") chips.push(PROTOCOL_LABEL[profile.protocol] ?? profile.protocol);
  if (profile.vpn_mode) chips.push("VPN");
  if (profile.direct_iran) chips.push("IR direct");
  if (
    profile.peer !== "" ||
    profile.wg_peer !== "" ||
    profile.wiw_outer !== "" ||
    profile.wiw_inner !== "" ||
    profile.mim_outer !== "" ||
    profile.mim_inner !== ""
  ) {
    chips.push("Pinned endpoint");
  }
  if (profile.fragment) chips.push("Fragment");
  if (profile.ech !== "") chips.push("ECH");
  if (
    (profile.protocol === "auto" || profile.protocol === "masque" || profile.protocol === "mim") &&
    profile.masque_http2
  ) {
    chips.push("H2");
  }
  if (profile.ip_version !== "v4") chips.push(profile.ip_version.toUpperCase());

  if (chips.length === 0) return null;
  return (
    <div className="flex max-w-xs flex-wrap items-center justify-center gap-1.5" aria-label="Active options">
      {chips.map((c) => (
        <span
          key={c}
          className="rounded-full bg-surface-3 px-2.5 py-0.5 font-mono text-[10px] tracking-wide text-muted-foreground ring-1 ring-white/10"
        >
          {c}
        </span>
      ))}
    </div>
  );
}
