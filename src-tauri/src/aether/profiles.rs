use serde::{Deserialize, Serialize};

/// `Auto` resolves to Aether's own default (MASQUE). Aether's own `scan_mode`
/// already performs multi-route discovery internally (confirmed by manually
/// running the real binary), so Aether-GUI does not implement a client-side
/// protocol-fallback retry loop on top of this.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Auto,
    Masque,
    Wireguard,
    Gool,
    Mim,
}

impl Protocol {
    /// The literal menu choice Aether expects at its "Protocol:" prompt.
    /// Flags passed up front normally suppress the prompt entirely, so the
    /// Mim choice is fallback-only (4th entry, matching the core's menu).
    pub fn as_menu_choice(&self) -> &'static str {
        match self {
            Protocol::Auto | Protocol::Masque => "1",
            Protocol::Wireguard => "2",
            Protocol::Gool => "3",
            Protocol::Mim => "4",
        }
    }

    /// MASQUE-family protocols share the MASQUE noize table, transport and
    /// evasion flags; WireGuard and gool share the WG ones.
    pub fn is_masque_family(&self) -> bool {
        matches!(self, Protocol::Auto | Protocol::Masque | Protocol::Mim)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ScanMode {
    Turbo,
    Balanced,
    Thorough,
    Stealth,
    Ironclad,
}

impl ScanMode {
    pub fn as_menu_choice(&self) -> &'static str {
        match self {
            ScanMode::Turbo => "1",
            ScanMode::Balanced => "2",
            ScanMode::Thorough => "3",
            ScanMode::Stealth => "4",
            ScanMode::Ironclad => "5",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IpVersion {
    V4,
    V6,
    Both,
}

impl IpVersion {
    pub fn as_menu_choice(&self) -> &'static str {
        match self {
            IpVersion::V4 => "1",
            IpVersion::V6 => "2",
            IpVersion::Both => "3",
        }
    }
}

/// Obfuscation profile for MASQUE connections. The profile shapes how much
/// junk/padding Aether injects to disguise the handshake from DPI.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MasqueNoize {
    Firewall,
    Gfw,
    Off,
}

impl MasqueNoize {
    pub fn as_flag(&self) -> &'static str {
        match self {
            MasqueNoize::Firewall => "firewall",
            MasqueNoize::Gfw => "gfw",
            MasqueNoize::Off => "off",
        }
    }
}

/// Obfuscation profile for WireGuard and gool connections.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WgNoize {
    Balanced,
    Aggressive,
    Light,
    Off,
}

impl WgNoize {
    pub fn as_flag(&self) -> &'static str {
        match self {
            WgNoize::Balanced => "balanced",
            WgNoize::Aggressive => "aggressive",
            WgNoize::Light => "light",
            WgNoize::Off => "off",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ConnectionProfile {
    pub protocol: Protocol,
    pub scan_mode: ScanMode,
    pub ip_version: IpVersion,
    /// Aether ≥1.1.1: reuse the last known-working gateway with a quick
    /// recheck instead of a full scan. `serde(default)` keeps profiles saved
    /// by older versions of this app loading cleanly.
    #[serde(default = "default_true")]
    pub quick_reconnect: bool,
    /// Aether ≥1.2.0: run the MASQUE tunnel over HTTP/2 (TCP) instead of the
    /// default HTTP/3 (QUIC) — for networks that block or throttle UDP.
    /// Passed as the AETHER_MASQUE_HTTP2 env var, not a flag: there is no
    /// `--h3` flag, and setting the env to any value also suppresses 1.2.0's
    /// new interactive "MASQUE transport" prompt in both directions.
    #[serde(default)]
    pub masque_http2: bool,
    /// Split the TLS ClientHello on the MASQUE HTTP/2 carrier (`--fragment`)
    /// to defeat inspectors that read the SNI from a single packet. TCP-only:
    /// only sent for MASQUE-family protocols.
    #[serde(default)]
    pub fragment: bool,
    /// Optional `--fragment-size` / `--fragment-delay` overrides (`16-32` /
    /// `2-10` are the core defaults). Only sent when `fragment` is on and
    /// the value parses as `n` or `a-b`.
    #[serde(default)]
    pub fragment_size: String,
    #[serde(default)]
    pub fragment_delay: String,
    /// Encrypted Client Hello (`--ech`): `auto` fetches an ECH config and
    /// hides the SNI, or paste a custom base64 config. Empty disables it.
    /// Only sent for MASQUE-family protocols.
    #[serde(default)]
    pub ech: String,
    /// Obfuscation profile for MASQUE (firewall/gfw/off). Passed as
    /// `--noize <value>`. Only sent when the active protocol is MASQUE-based.
    #[serde(default = "default_masque_noize")]
    pub masque_noize: MasqueNoize,
    /// Obfuscation profile for WireGuard/gool (balanced/aggressive/light/off).
    /// Only sent when the active protocol is WireGuard or gool.
    #[serde(default = "default_wg_noize")]
    pub wg_noize: WgNoize,
    /// WireGuard persistent-keepalive interval in seconds (`--keepalive`,
    /// core default 5). Empty keeps the core default. Only sent for
    /// WireGuard/gool — NAT on mobile networks otherwise drops idle UDP.
    #[serde(default)]
    pub wg_keepalive: String,
    /// Pinned endpoint (`--peer`, `ip:port`): skip the scan for a known-good
    /// address. Only sent for Auto/MASQUE/WireGuard — gool's hops have their
    /// own settings below, and naming a hop auto-selects its protocol.
    #[serde(default)]
    pub peer: String,
    /// Pinned WireGuard peer (`--wg-peer`): the warp-in-warp outer hop.
    /// Only sent for WireGuard/gool.
    #[serde(default)]
    pub wg_peer: String,
    /// Pinned WARP-in-WARP hops (`--wiw-outer` / `--wiw-inner`, `ip:port`).
    /// Name both and no scan runs at all; name one and the scan finds the
    /// other. Only sent for gool.
    #[serde(default)]
    pub wiw_outer: String,
    #[serde(default)]
    pub wiw_inner: String,
    /// Pinned MASQUE-in-MASQUE hops (`--mim-outer` / `--mim-inner`).
    /// Only sent for mim.
    #[serde(default)]
    pub mim_outer: String,
    #[serde(default)]
    pub mim_inner: String,
    /// Local SOCKS5 listen address (`--bind`). Aether defaults to
    /// 127.0.0.1:1819; users can change the port or bind to 0.0.0.0 for LAN.
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    /// Expose an HTTP CONNECT proxy (`--http-proxy`). HTTPS is served
    /// through the same port via CONNECT — the core has no separate
    /// HTTPS listener. Off by default.
    #[serde(default)]
    pub http_proxy_enabled: bool,
    /// Listen address for the HTTP proxy, e.g. 127.0.0.1:1820 or
    /// 0.0.0.0:1820 to share it on the LAN. Only sent when
    /// `http_proxy_enabled` is set.
    #[serde(default = "default_http_proxy_address")]
    pub http_proxy_address: String,
    /// Aether ≥1.5.0: optional resolvers used *inside* the tunnel. Kept as
    /// Aether's comma-separated CLI format, for example `1.1.1.1,1.0.0.1`.
    #[serde(default)]
    pub dns: String,
    /// Aether ≥1.5.0: Cloudflare Zero Trust organization name. An empty
    /// value means the normal consumer WARP flow.
    #[serde(default)]
    pub zero_trust_team: String,
    /// Which Zero Trust credential field is active in the GUI. This controls
    /// what is handed to the core, rather than being a core flag itself.
    #[serde(default)]
    pub zero_trust_auth: ZeroTrustAuth,
    /// Email used for Cloudflare Access one-time-code sign-in. Sensitive
    /// values are erased before the successful profile is persisted.
    #[serde(default)]
    pub access_email: String,
    /// Cloudflare Access service-token client id.
    #[serde(default)]
    pub access_client_id: String,
    /// Cloudflare Access service-token secret.
    #[serde(default)]
    pub access_client_secret: String,
    /// A pre-obtained Cloudflare Access enrolment JWT.
    #[serde(default)]
    pub access_token: String,
    /// Route HTTP/HTTPS through the organization's Gateway proxy. This is
    /// intentionally off by default because the organization can log it.
    #[serde(default)]
    pub zero_trust_gateway: bool,
    /// Aether ≥1.5.0 routing lists. Entries are comma/newline separated in
    /// the same format accepted by `--route-block` and `--route-direct`.
    #[serde(default)]
    pub route_block: String,
    #[serde(default)]
    pub route_direct: String,
    /// Also send high-traffic Iranian destinations straight out
    /// (`--route-direct` merged with the baked-in list below): domestic
    /// sites and banking stay fast and reachable, and less traffic enters
    /// the tunnel. Merged with `route_direct`, never replacing it.
    #[serde(default)]
    pub direct_iran: bool,
    /// Optional path to an Aether routing file with [block]/[direct] sections.
    #[serde(default)]
    pub routes_file: String,
    /// Start the app automatically when the OS boots. Persisted here, but
    /// the real effect is the OS autostart entry managed via set_autostart.
    #[serde(default)]
    pub autostart: bool,
    /// Connect automatically shortly after the app launches. Purely a GUI
    /// flag — never forwarded to the core.
    #[serde(default)]
    pub auto_connect: bool,
    /// VPN mode: after the SOCKS port is live, layer hev-socks5-tunnel + a
    /// default route + DNS on top so ALL system traffic uses the tunnel, not
    /// just proxy-configured apps. Tunnel behavior, so it lives in the
    /// profile — but it never reaches the core CLI except as `--mark`
    /// (Linux only) for loop avoidance. On Windows loop avoidance is done
    /// with direct bypass routes (see tun.rs) since Aether has no `--mark`
    /// there.
    #[serde(default)]
    pub vpn_mode: bool,
    /// MTU of the TUN adapter (`aether0`). 1500 is the default; PPPoE and
    /// most Iranian last-miles are more stable at 1280–1420. Clamped to
    /// 1280–9000 when the TUN is brought up.
    #[serde(default = "default_tun_mtu")]
    pub tun_mtu: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ZeroTrustAuth {
    #[default]
    Email,
    Service,
    Token,
}

fn default_true() -> bool {
    true
}

fn default_masque_noize() -> MasqueNoize {
    MasqueNoize::Firewall
}

fn default_wg_noize() -> WgNoize {
    WgNoize::Balanced
}

fn default_bind_address() -> String {
    "127.0.0.1:1819".into()
}

fn default_http_proxy_address() -> String {
    "127.0.0.1:1820".into()
}

fn default_tun_mtu() -> u32 {
    crate::tun::TUN_MTU
}

/// Baked-in `--route-direct` entries enabled by `direct_iran`: `private`
/// covers LAN/loopback/CGNAT, the rest are high-traffic Iranian destinations
/// (shops, video, banks of records) that stay faster and more reachable
/// outside the tunnel.
const IR_DIRECT_PRESET: &str = "private,digikala.com,aparat.com,filimo.com,telewebion.com,varzesh3.com,snapp.ir,cafebazaar.ir,divar.ir,namava.ir,shad.ir";

/// Accept only `ip:port` — the core needs a port and resolves no names here,
/// so hostnames and bare IPs are dropped rather than forwarded.
fn validated_endpoint(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    t.parse::<std::net::SocketAddr>()
        .ok()
        .map(|a| a.to_string())
}

/// Accept `n` or `a-b` of positive integers (fragment sizes/delays).
fn validated_range(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    let parts: Vec<&str> = t.split('-').collect();
    let ok = (parts.len() == 1 || parts.len() == 2)
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    if ok {
        Some(t.to_string())
    } else {
        None
    }
}

impl ConnectionProfile {
    /// CLI flags for Aether ≥1.1.1 — the whole profile is passed up front so
    /// the interactive prompts never appear (the PTY prompt-answering in
    /// pty.rs stays as a fallback). One of the two quick-reconnect flags is
    /// ALWAYS passed: without either, 1.1.1 asks its own interactive
    /// "reconnect with last gateway?" question, which the GUI must never
    /// leave unanswered.
    pub fn as_args(&self) -> Vec<String> {
        let mut args = Vec::with_capacity(20);
        match self.protocol {
            Protocol::Auto => {}
            Protocol::Masque => args.push("--masque".into()),
            Protocol::Wireguard => args.push("--wg".into()),
            Protocol::Gool => args.push("--gool".into()),
            Protocol::Mim => args.push("--mim".into()),
        }
        args.push(match self.scan_mode {
            ScanMode::Turbo => "--turbo".into(),
            ScanMode::Balanced => "--balanced".into(),
            ScanMode::Thorough => "--thorough".into(),
            ScanMode::Stealth => "--stealth".into(),
            ScanMode::Ironclad => "--ironclad".into(),
        });
        args.push(match self.ip_version {
            IpVersion::V4 => "-4".into(),
            IpVersion::V6 => "-6".into(),
            IpVersion::Both => "--dual".into(),
        });
        args.push(if self.quick_reconnect {
            "--quick-reconnect".into()
        } else {
            "--no-quick-reconnect".into()
        });
        // Noize profile — pick the value matching the active protocol family.
        args.push("--noize".into());
        args.push(
            if self.protocol.is_masque_family() {
                self.masque_noize.as_flag()
            } else {
                self.wg_noize.as_flag()
            }
            .into(),
        );
        // Endpoint pinning: skip the scan for known-good addresses. Each
        // flag is only sent for the protocol family it belongs to — naming
        // a hop auto-selects its protocol in the core, so an unfiltered
        // forward could silently switch protocols underneath the user.
        match self.protocol {
            Protocol::Auto | Protocol::Masque | Protocol::Wireguard => {
                if let Some(addr) = validated_endpoint(&self.peer) {
                    args.push("--peer".into());
                    args.push(addr);
                }
            }
            _ => {}
        }
        match self.protocol {
            Protocol::Wireguard | Protocol::Gool => {
                if let Some(addr) = validated_endpoint(&self.wg_peer) {
                    args.push("--wg-peer".into());
                    args.push(addr);
                }
            }
            _ => {}
        }
        if self.protocol == Protocol::Gool {
            if let Some(addr) = validated_endpoint(&self.wiw_outer) {
                args.push("--wiw-outer".into());
                args.push(addr);
            }
            if let Some(addr) = validated_endpoint(&self.wiw_inner) {
                args.push("--wiw-inner".into());
                args.push(addr);
            }
        }
        if self.protocol == Protocol::Mim {
            if let Some(addr) = validated_endpoint(&self.mim_outer) {
                args.push("--mim-outer".into());
                args.push(addr);
            }
            if let Some(addr) = validated_endpoint(&self.mim_inner) {
                args.push("--mim-inner".into());
                args.push(addr);
            }
        }
        // WireGuard keepalive: keep NAT bindings (notably mobile CGNAT)
        // from expiring on idle UDP. Empty keeps the core default (5s).
        if matches!(self.protocol, Protocol::Wireguard | Protocol::Gool) {
            let k = self.wg_keepalive.trim();
            if !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit()) {
                args.push("--keepalive".into());
                args.push(k.into());
            }
        }
        // MASQUE evasion: ClientHello fragmentation (HTTP/2 carrier) and
        // Encrypted Client Hello hide the SNI from DPI. MASQUE-family only.
        if self.protocol.is_masque_family() {
            if self.fragment {
                args.push("--fragment".into());
                if let Some(v) = validated_range(&self.fragment_size) {
                    args.push("--fragment-size".into());
                    args.push(v);
                }
                if let Some(v) = validated_range(&self.fragment_delay) {
                    args.push("--fragment-delay".into());
                    args.push(v);
                }
            }
            let ech = self.ech.trim();
            if !ech.is_empty() {
                args.push("--ech".into());
                args.push(ech.into());
            }
        }
        // Only forward --bind when non-default and parseable.
        if self.bind_address != default_bind_address()
            && self.bind_address.parse::<std::net::SocketAddr>().is_ok()
        {
            args.push("--bind".into());
            args.push(self.bind_address.clone());
        }
        // Only forward --http-proxy when explicitly enabled with a
        // parseable address; HTTPS is served on the same port via CONNECT.
        if self.http_proxy_enabled
            && self
                .http_proxy_address
                .parse::<std::net::SocketAddr>()
                .is_ok()
        {
            args.push("--http-proxy".into());
            args.push(self.http_proxy_address.clone());
        }
        if !self.dns.trim().is_empty() {
            args.push("--dns".into());
            args.push(self.dns.trim().into());
        }
        if !self.zero_trust_team.trim().is_empty() {
            args.push("--team".into());
            args.push(self.zero_trust_team.trim().into());
            if self.zero_trust_gateway {
                args.push("--gateway".into());
            }
        }
        if !self.route_block.trim().is_empty() {
            args.push("--route-block".into());
            args.push(self.route_block.trim().into());
        }
        {
            // The Iran preset merges with — never replaces — the user's own
            // direct list, so custom entries and the preset compose.
            let mut direct = self.route_direct.trim().to_string();
            if self.direct_iran {
                if !direct.is_empty() {
                    direct.push(',');
                }
                direct.push_str(IR_DIRECT_PRESET);
            }
            if !direct.is_empty() {
                args.push("--route-direct".into());
                args.push(direct);
            }
        }
        if !self.routes_file.trim().is_empty() {
            args.push("--routes".into());
            args.push(self.routes_file.trim().into());
        }
        // VPN mode: mark Aether's own sockets so the fwmark policy rule keeps
        // them on the real uplink instead of looping them back into the TUN.
        // `--mark` exists on Linux/Android only — never send it elsewhere
        // (Windows avoids loops with direct bypass routes in tun-up.ps1),
        // and never a bare `--vpn` flag (the core has no such option; the
        // TUN layer is entirely this app's job).
        if self.vpn_mode && cfg!(target_os = "linux") {
            args.push("--mark".into());
            args.push(crate::tun::TUN_FWMARK_STR.into());
        }
        args
    }

    /// The core accepts Zero Trust credentials as flags too, but putting a
    /// JWT or service secret in the process command line exposes it to other
    /// local processes. pty.rs supplies the selected credential as an env var
    /// instead, and this method ensures only that one method is ever sent.
    pub fn zero_trust_env(&self) -> Option<(&'static str, &str)> {
        if self.zero_trust_team.trim().is_empty() {
            return None;
        }
        match self.zero_trust_auth {
            ZeroTrustAuth::Email if !self.access_email.trim().is_empty() => {
                Some(("AETHER_ACCESS_EMAIL", self.access_email.trim()))
            }
            ZeroTrustAuth::Service
                if !self.access_client_id.trim().is_empty()
                    && !self.access_client_secret.trim().is_empty() =>
            {
                // The id and secret need separate variables, so this method
                // cannot represent service credentials. pty.rs handles that
                // pair directly after consulting `zero_trust_auth`.
                None
            }
            ZeroTrustAuth::Token if !self.access_token.trim().is_empty() => {
                Some(("AETHER_ACCESS_TOKEN", self.access_token.trim()))
            }
            _ => None,
        }
    }
}

impl Default for ConnectionProfile {
    fn default() -> Self {
        // Mirrors Aether's own defaults.
        Self {
            protocol: Protocol::Auto,
            scan_mode: ScanMode::Balanced,
            ip_version: IpVersion::V4,
            quick_reconnect: true,
            masque_http2: false,
            masque_noize: MasqueNoize::Firewall,
            wg_noize: WgNoize::Balanced,
            wg_keepalive: String::new(),
            peer: String::new(),
            wg_peer: String::new(),
            wiw_outer: String::new(),
            wiw_inner: String::new(),
            mim_outer: String::new(),
            mim_inner: String::new(),
            fragment: false,
            fragment_size: String::new(),
            fragment_delay: String::new(),
            ech: String::new(),
            bind_address: default_bind_address(),
            http_proxy_enabled: false,
            http_proxy_address: default_http_proxy_address(),
            dns: String::new(),
            zero_trust_team: String::new(),
            zero_trust_auth: ZeroTrustAuth::Email,
            access_email: String::new(),
            access_client_id: String::new(),
            access_client_secret: String::new(),
            access_token: String::new(),
            zero_trust_gateway: false,
            route_block: String::new(),
            route_direct: String::new(),
            direct_iran: false,
            routes_file: String::new(),
            autostart: false,
            auto_connect: false,
            vpn_mode: false,
            tun_mtu: default_tun_mtu(),
        }
    }
}

const STORE_FILE: &str = "profile.json";
const STORE_KEY: &str = "last_successful_profile";

/// Loads the last profile that reached `Connected`, or the hardcoded default
/// on first run. Only ever written by `save()` at the moment a connection
/// actually succeeds (see aether/mod.rs) — never on a mere attempt, so a bad
/// guess can't poison future one-click connects.
pub fn load(app: &tauri::AppHandle) -> ConnectionProfile {
    use tauri_plugin_store::StoreExt;
    app.store(STORE_FILE)
        .ok()
        .and_then(|s| s.get(STORE_KEY))
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

pub fn save(app: &tauri::AppHandle, profile: &ConnectionProfile) {
    use tauri_plugin_store::StoreExt;
    if let Ok(store) = app.store(STORE_FILE) {
        // A successful connection profile is useful to remember, but Access
        // credentials are not. Leave them in process memory only; the next
        // app launch will ask for them again rather than writing a JWT,
        // service secret or email address into profile.json.
        let mut persisted = profile.clone();
        persisted.access_email.clear();
        persisted.access_client_id.clear();
        persisted.access_client_secret.clear();
        persisted.access_token.clear();
        if let Ok(value) = serde_json::to_value(persisted) {
            store.set(STORE_KEY, value);
            let _ = store.save();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_omits_bind_flag() {
        let p = ConnectionProfile::default();
        let args = p.as_args();
        assert!(!args.iter().any(|a| a == "--bind"), "args={args:?}");
    }

    #[test]
    fn custom_port_emits_bind() {
        let p = ConnectionProfile {
            bind_address: "127.0.0.1:1919".into(),
            ..Default::default()
        };
        let args = p.as_args();
        let i = args
            .iter()
            .position(|a| a == "--bind")
            .expect("missing --bind");
        assert_eq!(args.get(i + 1).map(String::as_str), Some("127.0.0.1:1919"));
    }

    #[test]
    fn lan_bind_emits_bind() {
        let p = ConnectionProfile {
            bind_address: "0.0.0.0:1819".into(),
            ..Default::default()
        };
        let args = p.as_args();
        let i = args
            .iter()
            .position(|a| a == "--bind")
            .expect("missing --bind");
        assert_eq!(args.get(i + 1).map(String::as_str), Some("0.0.0.0:1819"));
    }

    #[test]
    fn lan_with_custom_port_emits_bind() {
        let p = ConnectionProfile {
            bind_address: "0.0.0.0:9999".into(),
            ..Default::default()
        };
        let args = p.as_args();
        let i = args
            .iter()
            .position(|a| a == "--bind")
            .expect("missing --bind");
        assert_eq!(args.get(i + 1).map(String::as_str), Some("0.0.0.0:9999"));
    }

    #[test]
    fn invalid_bind_is_not_forwarded() {
        let p = ConnectionProfile {
            bind_address: "127.0.0.1:".into(),
            ..Default::default()
        };
        let args = p.as_args();
        assert!(!args.iter().any(|a| a == "--bind"), "args={args:?}");
    }

    #[test]
    fn old_profile_json_gets_defaults() {
        let json = r#"{"protocol":"auto","scan_mode":"balanced","ip_version":"v4","quick_reconnect":true,"masque_http2":false}"#;
        let p: ConnectionProfile = serde_json::from_str(json).unwrap();
        assert_eq!(p.bind_address, "127.0.0.1:1819");
        assert_eq!(p.masque_noize, MasqueNoize::Firewall);
        assert!(!p.autostart);
        assert!(!p.auto_connect);
        assert!(!p.vpn_mode);
        assert!(p.peer.is_empty());
        assert!(p.wg_peer.is_empty());
        assert!(p.wiw_outer.is_empty());
        assert!(p.wiw_inner.is_empty());
        assert!(p.mim_outer.is_empty());
        assert!(p.mim_inner.is_empty());
        assert!(!p.fragment);
        assert!(p.fragment_size.is_empty());
        assert!(p.fragment_delay.is_empty());
        assert!(p.ech.is_empty());
        assert!(!p.direct_iran);
        assert_eq!(p.tun_mtu, 1500);
        assert!(p.wg_keepalive.is_empty());
    }

    #[test]
    fn default_emits_noize() {
        let p = ConnectionProfile::default();
        let args = p.as_args();
        let i = args
            .iter()
            .position(|a| a == "--noize")
            .expect("missing --noize");
        assert_eq!(args.get(i + 1).map(String::as_str), Some("firewall"));
    }

    #[test]
    fn v150_options_emit_without_credentials() {
        let p = ConnectionProfile {
            dns: "9.9.9.9,1.1.1.1".into(),
            zero_trust_team: "acme".into(),
            zero_trust_gateway: true,
            route_block: "ads.example".into(),
            route_direct: "private".into(),
            routes_file: "C:/routes.txt".into(),
            ..Default::default()
        };
        assert_eq!(
            p.as_args(),
            vec![
                "--balanced",
                "-4",
                "--quick-reconnect",
                "--noize",
                "firewall",
                "--dns",
                "9.9.9.9,1.1.1.1",
                "--team",
                "acme",
                "--gateway",
                "--route-block",
                "ads.example",
                "--route-direct",
                "private",
                "--routes",
                "C:/routes.txt"
            ]
        );
    }

    #[test]
    fn zero_trust_email_is_provided_as_an_environment_value() {
        let p = ConnectionProfile {
            zero_trust_team: "acme".into(),
            access_email: "me@example.com".into(),
            ..Default::default()
        };
        assert_eq!(
            p.zero_trust_env(),
            Some(("AETHER_ACCESS_EMAIL", "me@example.com"))
        );
        assert!(!p.as_args().iter().any(|arg| arg.contains("me@example.com")));
    }

    #[test]
    fn default_profile_has_no_mark_flag() {
        let args = ConnectionProfile::default().as_args();
        assert!(!args.iter().any(|a| a == "--mark"), "args={args:?}");
    }

    #[test]
    fn vpn_mode_never_leaks_a_bare_vpn_flag() {
        let p = ConnectionProfile {
            vpn_mode: true,
            ..Default::default()
        };
        let args = p.as_args();
        assert!(
            !args.iter().any(|a| a == "--vpn" || a == "--tun"),
            "args={args:?}"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn vpn_mode_emits_mark_for_loop_avoidance() {
        let p = ConnectionProfile {
            vpn_mode: true,
            ..Default::default()
        };
        let args = p.as_args();
        let i = args
            .iter()
            .position(|a| a == "--mark")
            .expect("missing --mark");
        assert_eq!(
            args.get(i + 1).map(String::as_str),
            Some(crate::tun::TUN_FWMARK_STR)
        );
    }

    /// Windows has no `--mark` (Linux/Android only) — loop avoidance there
    /// is direct bypass routes added by tun-up.ps1, never a core flag.
    #[cfg(target_os = "windows")]
    #[test]
    fn vpn_mode_emits_no_mark_on_windows() {
        let p = ConnectionProfile {
            vpn_mode: true,
            ..Default::default()
        };
        let args = p.as_args();
        assert!(!args.iter().any(|a| a == "--mark"), "args={args:?}");
    }

    fn flag_value(args: &[String], flag: &str) -> Option<String> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1).cloned())
    }

    #[test]
    fn default_emits_none_of_the_new_flags() {
        let args = ConnectionProfile::default().as_args();
        for f in [
            "--peer",
            "--wg-peer",
            "--wiw-outer",
            "--wiw-inner",
            "--mim-outer",
            "--mim-inner",
            "--mim",
            "--fragment",
            "--fragment-size",
            "--fragment-delay",
            "--ech",
            "--keepalive",
        ] {
            assert!(!args.iter().any(|a| a == f), "{f} leaked: {args:?}");
        }
        assert!(!args.iter().any(|a| a == "--route-direct"), "args={args:?}");
    }

    #[test]
    fn peer_pin_emitted_for_masque_and_validated() {
        let p = ConnectionProfile {
            protocol: Protocol::Masque,
            peer: "162.159.196.1:443".into(),
            ..Default::default()
        };
        assert_eq!(
            flag_value(&p.as_args(), "--peer").as_deref(),
            Some("162.159.196.1:443")
        );
        // Hostnames and bare IPs are dropped, never forwarded.
        for bad in ["example.com:443", "162.159.196.1", "not an address", ""] {
            let p = ConnectionProfile {
                peer: bad.into(),
                ..Default::default()
            };
            assert!(
                flag_value(&p.as_args(), "--peer").is_none(),
                "bad peer {bad:?} leaked"
            );
        }
    }

    #[test]
    fn peer_pin_never_leaks_across_protocol_families() {
        // --peer on gool could hand hop selection to the wrong flag set.
        let p = ConnectionProfile {
            protocol: Protocol::Gool,
            peer: "162.159.196.1:443".into(),
            wiw_outer: "162.159.192.1:2408".into(),
            wiw_inner: "188.114.96.1:2408".into(),
            ..Default::default()
        };
        let args = p.as_args();
        assert!(flag_value(&args, "--peer").is_none(), "args={args:?}");
        assert_eq!(
            flag_value(&args, "--wiw-outer").as_deref(),
            Some("162.159.192.1:2408")
        );
        assert_eq!(
            flag_value(&args, "--wiw-inner").as_deref(),
            Some("188.114.96.1:2408")
        );
    }

    #[test]
    fn mim_selects_mim_flag_and_masque_noize() {
        let p = ConnectionProfile {
            protocol: Protocol::Mim,
            mim_outer: "162.159.192.1:443".into(),
            ..Default::default()
        };
        let args = p.as_args();
        assert!(args.iter().any(|a| a == "--mim"), "args={args:?}");
        assert_eq!(
            flag_value(&args, "--mim-outer").as_deref(),
            Some("162.159.192.1:443")
        );
        assert_eq!(flag_value(&args, "--noize").as_deref(), Some("firewall"));
        assert_eq!(Protocol::Mim.as_menu_choice(), "4");
    }

    #[test]
    fn fragment_and_ech_stay_in_masque_family() {
        let p = ConnectionProfile {
            protocol: Protocol::Masque,
            fragment: true,
            fragment_size: "8-16".into(),
            fragment_delay: "1-5".into(),
            ech: "auto".into(),
            ..Default::default()
        };
        let args = p.as_args();
        assert!(args.iter().any(|a| a == "--fragment"), "args={args:?}");
        assert_eq!(
            flag_value(&args, "--fragment-size").as_deref(),
            Some("8-16")
        );
        assert_eq!(
            flag_value(&args, "--fragment-delay").as_deref(),
            Some("1-5")
        );
        assert_eq!(flag_value(&args, "--ech").as_deref(), Some("auto"));
        // Garbage ranges fall back to core defaults (flag without value).
        let p = ConnectionProfile {
            fragment_size: "abc".into(),
            ..p
        };
        assert!(flag_value(&p.as_args(), "--fragment-size").is_none());
        // WireGuard has no ClientHello to fragment.
        let p = ConnectionProfile {
            protocol: Protocol::Wireguard,
            fragment: true,
            ech: "auto".into(),
            ..Default::default()
        };
        let args = p.as_args();
        assert!(!args.iter().any(|a| a == "--fragment"), "args={args:?}");
        assert!(flag_value(&args, "--ech").is_none(), "args={args:?}");
    }

    #[test]
    fn keepalive_emitted_for_wireguard_only() {
        let p = ConnectionProfile {
            protocol: Protocol::Gool,
            wg_keepalive: "25".into(),
            ..Default::default()
        };
        assert_eq!(
            flag_value(&p.as_args(), "--keepalive").as_deref(),
            Some("25")
        );
        let p = ConnectionProfile {
            protocol: Protocol::Masque,
            wg_keepalive: "25".into(),
            ..Default::default()
        };
        assert!(flag_value(&p.as_args(), "--keepalive").is_none());
        let p = ConnectionProfile {
            protocol: Protocol::Wireguard,
            wg_keepalive: "soon".into(),
            ..Default::default()
        };
        assert!(flag_value(&p.as_args(), "--keepalive").is_none());
    }

    #[test]
    fn iran_preset_merges_with_user_direct_list() {
        // Preset alone.
        let p = ConnectionProfile {
            direct_iran: true,
            ..Default::default()
        };
        let v = flag_value(&p.as_args(), "--route-direct").expect("missing");
        assert!(v.starts_with("private,"), "{v}");
        assert!(v.contains("snapp.ir"), "{v}");
        // User entries compose in front, never replaced.
        let p = ConnectionProfile {
            route_direct: "mybank.ir".into(),
            direct_iran: true,
            ..Default::default()
        };
        let v = flag_value(&p.as_args(), "--route-direct").expect("missing");
        assert!(v.starts_with("mybank.ir,private,"), "{v}");
    }
}
