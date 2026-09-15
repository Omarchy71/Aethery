//! TUN / VPN mode for Aethery.
//!
//! Aether itself only exposes a local SOCKS5 (+ optional HTTP) proxy — there
//! is no TUN device on its side. VPN mode closes that gap by layering a
//! `tun2socks` forwarder ([hev-socks5-tunnel](https://github.com/heiher/hev-socks5-tunnel),
//! bundled as the `hev` sidecar next to `aether`) between a kernel TUN
//! interface and Aether's SOCKS port, then moving the system default route
//! and DNS onto that interface:
//!
//! ```text
//! apps ──► default route ──► aether0 (TUN) ──► hev ──► 127.0.0.1:1819 ──► aether ──► internet
//! ```
//!
//! Two platform backends share this interface:
//!
//! * **Linux**: the privileged half (interface creation, routes,
//!   `/etc/resolv.conf`) runs through one-shot `pkexec` shell scripts, and
//!   routing loops are avoided with a firewall mark — Aether is launched
//!   with `--mark <TUN_FWMARK>` and an `ip rule fwmark … table main`
//!   exception keeps marked packets on the real uplink.
//! * **Windows**: the privileged half runs through one-shot PowerShell
//!   scripts (`tun-up.ps1` / `tun-down.ps1`) launched elevated via UAC, with
//!   hev's official `win64` build over the Wintun driver (`wintun.dll` must
//!   sit next to `hev.exe`). `--mark` exists on Linux/Android only, so loop
//!   avoidance instead adds explicit `/32` bypass routes for Aether's own
//!   live remote addresses through the original gateway — derived from the
//!   tunnel process's actual sockets at connect time, so it works for every
//!   protocol (MASQUE / WireGuard / gool) without parsing logs.
//!
//! Either way the GUI itself never runs elevated, the user gets a single
//! admin prompt per connect/disconnect, and a failed VPN step degrades to a
//! plain proxy instead of failing the connection.

use crate::events::{now_millis, LogEvent, LOG_EVENT};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// Whether this OS has a TUN backend. VPN mode is a no-op everywhere else:
/// the profile flag still loads/saves, but nothing is brought up.
pub const SUPPORTED: bool = cfg!(target_os = "linux") || cfg!(target_os = "windows");

/// Kernel interface / adapter name. Deliberately NOT `tun0` — test machines
/// (and other VPNs) commonly already own `tun0`, and colliding with it would
/// hijack or break an unrelated tunnel.
pub const TUN_NAME: &str = "aether0";
/// Address assigned to the TUN interface (hev-socks5-tunnel convention:
/// documentation/reserved TEST-NET-2 space, never routed publicly).
pub const TUN_IPV4: &str = "198.18.0.1";
/// ULA address for the adapter when the profile uses IPv6 (hev sample
/// default; same reserved-space idea as the IPv4 address).
pub const TUN_IPV6: &str = "fc00::1";
pub const TUN_MTU: u32 = 1500;
/// Firewall mark shared by Aether's `--mark` and hev's `mark:` — packets
/// carrying it bypass the TUN via `ip rule … table main`. Linux only:
/// Aether has no `--mark` on Windows, where bypass routes do this job.
pub const TUN_FWMARK: u32 = 0x9e;
pub const TUN_FWMARK_STR: &str = "0x9e";
/// DNS used for the system resolver while the VPN is up when the profile has
/// no explicit `dns` value (mirrors Aether's own default).
pub const FALLBACK_DNS: &str = "1.1.1.1,1.0.0.1";

const HEV_PID_FILE: &str = "hev.pid";
const HEV_CONFIG_FILE: &str = "hev-tunnel.yml";
const HEV_LOG_FILE: &str = "hev.log";
/// Transcript the elevated PowerShell helpers write (the UAC boundary can't
/// pipe stdout back, so the script logs to a file and Rust replays it).
/// Also the handoff the elevated launcher uses to report its exit code.
#[cfg(target_os = "windows")]
const WIN_LOG_FILE: &str = "tun-last.log";
#[cfg(target_os = "windows")]
const WIN_EXIT_FILE: &str = "tun-exit.code";
/// Exit code the elevated launcher reports when the UAC prompt is declined.
#[cfg(target_os = "windows")]
const UAC_CANCELLED_CODE: i32 = 1223;

#[derive(Serialize, Clone, Debug)]
pub struct VpnStatus {
    pub active: bool,
    pub iface: String,
    pub detail: String,
}

fn data_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
}

/// Locate a bundled resource binary, falling back to the source tree so
/// `tauri dev` works without a bundle install.
fn resolve_resource(app: &AppHandle, dir: &str, name: &str) -> Option<PathBuf> {
    if let Ok(res) = app.path().resource_dir() {
        let p = res.join(dir).join(name);
        if p.exists() {
            return Some(p);
        }
    }
    // Dev fallback: <repo>/src-tauri/<dir>/<name> next to Cargo.toml.
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(dir)
        .join(name);
    if dev.exists() {
        return Some(dev);
    }
    None
}

fn resolve_hev(app: &AppHandle) -> Result<PathBuf, String> {
    let exe = if cfg!(target_os = "windows") {
        "hev.exe"
    } else {
        "hev"
    };
    let p = resolve_resource(app, "binaries", exe).ok_or_else(|| {
        if cfg!(target_os = "windows") {
            "tun2socks binary not found (run src-tauri/binaries/fetch-hev.ps1)".to_string()
        } else {
            "tun2socks binary not found (run src-tauri/binaries/fetch-hev.sh)".to_string()
        }
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755));
    }
    // hev's Windows build drives the Wintun driver through a DLL that must
    // sit next to the exe — fail fast with a clear message instead of a
    // cryptic elevated-process startup error after the UAC prompt.
    #[cfg(target_os = "windows")]
    {
        let sibling = p.with_file_name("wintun.dll");
        if !sibling.exists() {
            return Err("wintun.dll missing next to hev.exe (re-run fetch-hev.ps1)".into());
        }
    }
    Ok(p)
}

fn resolve_script(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    resolve_resource(app, "scripts", name)
        .ok_or_else(|| format!("VPN helper script missing: scripts/{name}"))
}

fn emit_log(app: &AppHandle, line: String) {
    let _ = app.emit(
        LOG_EVENT,
        &LogEvent {
            line,
            timestamp: now_millis(),
        },
    );
}

/// Split `--bind`-style `host:port` into the dial target for hev. A `0.0.0.0`
/// (LAN-share) bind is dialed back over loopback — hev must not aim at the
/// wildcard address itself.
pub fn socks_target(bind_address: &str) -> (String, u16) {
    if let Ok(sock) = bind_address.parse::<std::net::SocketAddr>() {
        let host = if sock.ip().is_unspecified() {
            "127.0.0.1".to_string()
        } else {
            sock.ip().to_string()
        };
        return (host, sock.port());
    }
    ("127.0.0.1".to_string(), 1819)
}

/// hev YAML config. Same schema on both platforms; Windows omits `mark:`
/// (no SO_MARK there — bypass routes handle loop avoidance) and always
/// carries a ULA IPv6 address so dual-stack profiles work.
fn hev_config_for(host: &str, port: u16, data: &Path, for_windows: bool) -> String {
    let mark_line = if for_windows {
        String::new()
    } else {
        format!("         \u{20}mark: {mark}\n", mark = TUN_FWMARK)
    };
    let ipv6_line = if for_windows {
        format!("         \u{20}ipv6: '{addr}'\n", addr = TUN_IPV6)
    } else {
        String::new()
    };
    format!(
        "# Generated by Aethery VPN mode — do not edit (regenerated per connect).\n\
         tunnel:\n\
         \u{20}name: {iface}\n\
         \u{20}mtu: {mtu}\n\
         \u{20}multi-queue: false\n\
         \u{20}ipv4: {addr}\n\
         {ipv6_line}\
         socks5:\n\
         \u{20}address: {host}\n\
         \u{20}port: {port}\n\
         \u{20}udp: 'udp'\n\
         {mark_line}\
         misc:\n\
         \u{20}log-file: '{log}'\n\
         \u{20}log-level: warn\n\
         \u{20}task-stack-size: 86016\n",
        iface = TUN_NAME,
        mtu = TUN_MTU,
        addr = TUN_IPV4,
        host = host,
        port = port,
        log = data.join(HEV_LOG_FILE).display(),
    )
}

#[cfg(unix)]
fn pid_alive(data: &Path) -> bool {
    let pid: i32 = std::fs::read_to_string(data.join(HEV_PID_FILE))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    pid > 0 && Path::new(&format!("/proc/{pid}")).exists()
}

/// Same exact-PID tasklist check as `aether::orphan` on Windows: a substring
/// search would mistake e.g. pid 123 for 1234.
#[cfg(target_os = "windows")]
fn pid_alive(data: &Path) -> bool {
    let pid: u32 = std::fs::read_to_string(data.join(HEV_PID_FILE))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    if pid == 0 {
        return false;
    }
    let out = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output();
    let Ok(out) = out else {
        return false;
    };
    let want = pid.to_string();
    String::from_utf8_lossy(&out.stdout).lines().any(|line| {
        line.split(',')
            .nth(1)
            .is_some_and(|f| f.trim_matches('"') == want)
    })
}

#[cfg(unix)]
fn iface_exists() -> bool {
    std::process::Command::new("ip")
        .args(["link", "show", TUN_NAME])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Unprivileged adapter check — plain `netsh` reads need no elevation, so
/// this never prompts. Matches the full adapter name only, not a prefix.
#[cfg(target_os = "windows")]
fn iface_exists() -> bool {
    let out = std::process::Command::new("netsh")
        .args([
            "interface",
            "show",
            "interface",
            &format!("name={TUN_NAME}"),
        ])
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).lines().any(|l| {
            let t = l.trim_end();
            t == TUN_NAME || t.ends_with(&format!(" {TUN_NAME}"))
        }),
        _ => false,
    }
}

/// Unprivileged liveness check — safe to call anywhere, never prompts.
pub fn is_up(app: &AppHandle) -> bool {
    pid_alive(&data_dir(app)) && iface_exists()
}

pub fn status(app: &AppHandle) -> VpnStatus {
    let data = data_dir(app);
    let alive = pid_alive(&data);
    let iface = iface_exists();
    let (active, detail) = match (alive, iface) {
        (true, true) => (true, format!("TUN {TUN_NAME} via hev-socks5-tunnel")),
        (true, false) => (false, "hev running but interface missing".into()),
        (false, true) => (
            false,
            format!("stale {TUN_NAME} interface, hev not running"),
        ),
        (false, false) => (false, "down".into()),
    };
    VpnStatus {
        active,
        iface: TUN_NAME.into(),
        detail,
    }
}

/// Run `pkexec sh <script> <args…>` with a hard timeout, forwarding output
/// lines into the app log. Returns the script's stdout on success.
#[cfg(unix)]
fn run_privileged(
    app: &AppHandle,
    script: &Path,
    args: &[String],
    timeout_secs: u64,
) -> Result<String, String> {
    if !cfg!(target_os = "linux") {
        return Err("VPN mode is Linux-only".into());
    }
    let mut child = std::process::Command::new("pkexec")
        .arg("sh")
        .arg(script)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("pkexec failed to start ({e}); is polkit installed?"))?;

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                return Err("privileged VPN step timed out".into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(200)),
            Err(e) => return Err(format!("waiting on pkexec: {e}")),
        }
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("reading pkexec output: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    for line in stdout.lines().chain(stderr.lines()) {
        let line = line.trim();
        if !line.is_empty() {
            emit_log(app, format!("[tun] {line}"));
        }
    }
    if out.status.success() {
        Ok(stdout)
    } else {
        Err(format!(
            "privileged VPN step failed ({}): {}",
            out.status,
            stderr.lines().next().unwrap_or("see log").trim()
        ))
    }
}

/// Quote one argument for embedding in a PowerShell single-quoted string.
#[cfg(target_os = "windows")]
fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Run a `.ps1` helper elevated via UAC with a hard timeout.
///
/// `Start-Process -Verb RunAs` can't pipe the elevated child's stdout back
/// across the UAC boundary (and its `-Wait` is unreliable for elevated
/// targets), so completion is signalled with a sentinel file: the script
/// always writes `<data>\tun-exit.code` last, and logs every line to
/// `<data>\tun-last.log`, which is replayed into the app log here.
#[cfg(target_os = "windows")]
fn run_elevated(
    app: &AppHandle,
    script: &Path,
    args: &[String],
    timeout_secs: u64,
) -> Result<String, String> {
    let data = data_dir(app);
    let log_file = data.join(WIN_LOG_FILE);
    let exit_file = data.join(WIN_EXIT_FILE);
    let _ = std::fs::remove_file(&log_file);
    let _ = std::fs::remove_file(&exit_file);

    let mut argv = vec![
        "-NoProfile".to_string(),
        "-ExecutionPolicy".to_string(),
        "Bypass".to_string(),
        "-File".to_string(),
        script.display().to_string(),
    ];
    argv.extend(args.iter().cloned());
    let argv_ps: Vec<String> = argv.iter().map(|a| ps_quote(a)).collect();
    let inner = format!(
        "try {{ $p = Start-Process -FilePath 'powershell.exe' -ArgumentList {args} -Verb RunAs -PassThru; exit 0 }} catch {{ exit {cancel} }}",
        args = argv_ps.join(","),
        cancel = UAC_CANCELLED_CODE,
    );
    let mut child = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &inner])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to start powershell ({e})"))?;

    // The launcher exits once the UAC reply arrives; the elevated script
    // signals its own completion via the sentinel file. Wait for both.
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let code: i32 = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if let Some(c) = status.code() {
                    if c == UAC_CANCELLED_CODE {
                        return Err(
                            "Administrator approval was declined — proxy still works".into()
                        );
                    }
                    if c != 0 {
                        return Err(format!(
                            "could not request elevation (launcher exit {c}) — proxy still works"
                        ));
                    }
                }
                break wait_for_sentinel(&exit_file, deadline)?;
            }
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                return Err("elevated VPN step timed out".into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(200)),
            Err(e) => return Err(format!("waiting on elevation prompt: {e}")),
        }
    };

    let transcript = std::fs::read_to_string(&log_file).unwrap_or_default();
    for line in transcript.lines() {
        let line = line.trim();
        if !line.is_empty() {
            emit_log(app, format!("[tun] {line}"));
        }
    }
    if code == 0 {
        Ok(transcript)
    } else if code == UAC_CANCELLED_CODE {
        Err("Administrator approval was declined — proxy still works".into())
    } else {
        let first = transcript
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .unwrap_or("see log");
        Err(format!("elevated VPN step failed (exit {code}): {first}"))
    }
}

/// Block until the elevated helper writes its exit-code sentinel.
#[cfg(target_os = "windows")]
fn wait_for_sentinel(exit_file: &Path, deadline: Instant) -> Result<i32, String> {
    loop {
        if let Ok(raw) = std::fs::read_to_string(exit_file) {
            if let Ok(code) = raw.trim().parse::<i32>() {
                return Ok(code);
            }
        }
        if Instant::now() >= deadline {
            return Err("elevated VPN step timed out".into());
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Bring the TUN up after Aether's SOCKS port is live. Returns true when
/// traffic is actually flowing through the TUN; false keeps the session as a
/// plain proxy (never fails the connection — a VPN setup problem must not
/// kill a working proxy).
///
/// `aether_pid` is the live tunnel process; on Windows its current remote
/// addresses become direct bypass routes so tunnel traffic can't loop back
/// into the TUN (Linux uses `--mark` + fwmark instead and ignores this).
pub fn bring_up(app: &AppHandle, bind_address: &str, dns: &str, aether_pid: u32) -> bool {
    if !SUPPORTED {
        emit_log(
            app,
            "[tun] VPN mode is not supported on this OS — proxy still works".into(),
        );
        return false;
    }
    let data = data_dir(app);
    if let Err(e) = std::fs::create_dir_all(&data) {
        emit_log(app, format!("[tun] cannot use data dir: {e}"));
        return false;
    }
    if is_up(app) {
        emit_log(app, format!("[tun] {TUN_NAME} already up"));
        return true;
    }
    let hev = match resolve_hev(app) {
        Ok(p) => p,
        Err(e) => {
            emit_log(app, format!("[tun] {e}"));
            return false;
        }
    };
    let (script_name, prompt_hint) = if cfg!(target_os = "windows") {
        ("tun-up.ps1", "UAC prompt expected")
    } else {
        ("tun-up.sh", "polkit prompt expected")
    };
    let script = match resolve_script(app, script_name) {
        Ok(p) => p,
        Err(e) => {
            emit_log(app, format!("[tun] {e}"));
            return false;
        }
    };
    let (host, port) = socks_target(bind_address);
    let cfg = hev_config_for(&host, port, &data, cfg!(target_os = "windows"));
    if let Err(e) = std::fs::write(data.join(HEV_CONFIG_FILE), &cfg) {
        emit_log(app, format!("[tun] cannot write hev config: {e}"));
        return false;
    }
    let dns_csv = {
        let d = dns.trim();
        if d.is_empty() {
            FALLBACK_DNS.to_string()
        } else {
            d.to_string()
        }
    };
    emit_log(
        app,
        format!("[tun] bringing up {TUN_NAME} via 127.0.0.1:{port} ({prompt_hint})…"),
    );
    let args = if cfg!(target_os = "windows") {
        vec![
            data.display().to_string(),
            hev.display().to_string(),
            data.join(HEV_CONFIG_FILE).display().to_string(),
            TUN_NAME.to_string(),
            dns_csv,
            aether_pid.to_string(),
        ]
    } else {
        vec![
            data.display().to_string(),
            hev.display().to_string(),
            data.join(HEV_CONFIG_FILE).display().to_string(),
            TUN_NAME.to_string(),
            TUN_FWMARK_STR.to_string(),
            dns_csv,
        ]
    };
    #[cfg(target_os = "windows")]
    let result = run_elevated(app, &script, &args, 120);
    #[cfg(unix)]
    let result = run_privileged(app, &script, &args, 120);
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    let result: Result<String, String> = Err("VPN mode is not supported on this OS".into());
    match result {
        Ok(_) if is_up(app) => {
            emit_log(
                app,
                format!("[tun] {TUN_NAME} up — system traffic via tunnel"),
            );
            true
        }
        Ok(_) => {
            emit_log(
                app,
                "[tun] setup ran but interface is not up — proxy still works".into(),
            );
            false
        }
        Err(e) => {
            emit_log(
                app,
                format!("[tun] not activated ({e}) — proxy still works"),
            );
            false
        }
    }
}

/// Tear the TUN down. Best-effort and prompt-free when already down.
pub fn bring_down(app: &AppHandle) {
    if !SUPPORTED {
        return;
    }
    if !pid_alive(&data_dir(app)) && !iface_exists() {
        return;
    }
    let script_name = if cfg!(target_os = "windows") {
        "tun-down.ps1"
    } else {
        "tun-down.sh"
    };
    let script = match resolve_script(app, script_name) {
        Ok(p) => p,
        Err(e) => {
            emit_log(app, format!("[tun] {e}"));
            return;
        }
    };
    let data = data_dir(app);
    let args = if cfg!(target_os = "windows") {
        vec![data.display().to_string(), TUN_NAME.to_string()]
    } else {
        vec![
            data.display().to_string(),
            TUN_NAME.to_string(),
            TUN_FWMARK_STR.to_string(),
        ]
    };
    #[cfg(target_os = "windows")]
    let result = run_elevated(app, &script, &args, 60);
    #[cfg(unix)]
    let result = run_privileged(app, &script, &args, 30);
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    let result: Result<String, String> = Err("VPN mode is not supported on this OS".into());
    match result {
        Ok(_) => emit_log(app, format!("[tun] {TUN_NAME} down — routes/DNS restored")),
        Err(e) => emit_log(app, format!("[tun] teardown issue ({e})")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_bind_targets_loopback() {
        assert_eq!(socks_target("127.0.0.1:1819"), ("127.0.0.1".into(), 1819));
    }

    #[test]
    fn wildcard_bind_dials_back_over_loopback() {
        assert_eq!(socks_target("0.0.0.0:1919"), ("127.0.0.1".into(), 1919));
    }

    #[test]
    fn garbage_bind_falls_back_to_default() {
        assert_eq!(socks_target("127.0.0.1:"), ("127.0.0.1".into(), 1819));
    }

    #[test]
    fn config_points_at_socks_and_names_tun() {
        let cfg = hev_config_for("127.0.0.1", 1819, Path::new("/tmp/x"), false);
        assert!(cfg.contains("name: aether0"), "{cfg}");
        assert!(cfg.contains("address: 127.0.0.1"), "{cfg}");
        assert!(cfg.contains("port: 1819"), "{cfg}");
        assert!(cfg.contains("mark: 158"), "{cfg}");
    }

    #[test]
    fn windows_config_has_no_mark_but_has_ipv6() {
        let cfg = hev_config_for("127.0.0.1", 1819, Path::new("/tmp/x"), true);
        assert!(cfg.contains("name: aether0"), "{cfg}");
        assert!(cfg.contains("address: 127.0.0.1"), "{cfg}");
        assert!(!cfg.contains("mark:"), "{cfg}");
        assert!(cfg.contains("ipv6:"), "{cfg}");
    }
}
