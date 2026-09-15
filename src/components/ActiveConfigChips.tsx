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
  // Mirror profiles.rs precedence: a custom rules file skips the generated
  // Iran preset, so the chip must not claim it while the file field is set.
  if (profile.direct_iran && profile.routes_file.trim() === "") chips.push("IR direct");
  // Mirror profiles.rs gating: a stored pin only counts when it is actually
  // sent for the selected protocol, so switching protocols can't show a
  // stale "Pinned endpoint" for an inert field.
  const p = profile.protocol;
  const pinActive =
    ((p === "auto" || p === "masque" || p === "wireguard") && profile.peer !== "") ||
    ((p === "wireguard" || p === "gool") && profile.wg_peer !== "") ||
    (p === "gool" && (profile.wiw_outer !== "" || profile.wiw_inner !== "")) ||
    (p === "mim" && (profile.mim_outer !== "" || profile.mim_inner !== ""));
  if (pinActive) {
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
