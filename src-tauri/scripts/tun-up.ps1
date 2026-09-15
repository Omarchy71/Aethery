# Bring up the Aethery VPN on Windows: start hev-socks5-tunnel (Wintun),
# add bypass routes for Aether's own remote addresses, move the default
# route and DNS onto the TUN adapter.
#
# Runs ELEVATED via UAC (one prompt per connect). The UAC boundary cannot
# pipe stdout back, so every line is appended to tun-last.log and the final
# exit code to tun-exit.code in the state dir; the Rust backend replays the
# log into the app log. Idempotent: safe to re-run when already up.
#
# Args: STATE_DIR HEV_EXE HEV_CONFIG TUN_NAME DNS_CSV AETHER_PID
#   AETHER_PID may be 0 — then the single live aether.exe is auto-detected.
param(
    [string]$StateDir,
    [string]$HevExe,
    [string]$HevConfig,
    [string]$TunName,
    [string]$DnsCsv,
    [uint32]$AetherPid
)

$ErrorActionPreference = 'Stop'
$LogFile   = Join-Path $StateDir 'tun-last.log'
$PidFile   = Join-Path $StateDir 'hev.pid'
$StateFile = Join-Path $StateDir 'tun-net.json'
$ExitFile  = Join-Path $StateDir 'tun-exit.code'

if (Test-Path $LogFile)  { Remove-Item $LogFile -Force -ErrorAction SilentlyContinue }
if (Test-Path $ExitFile) { Remove-Item $ExitFile -Force -ErrorAction SilentlyContinue }

function Log($m) {
    "[tun-up] $m" | Out-File -FilePath $LogFile -Append -Encoding ascii
}

function Finish($code) {
    "$code" | Out-File -FilePath $ExitFile -Encoding ascii
    exit $code
}

function IsLoopbackOrLocal($ip) {
    $s = $ip.ToString()
    if ($ip.AddressFamily -eq 'InterNetwork') {
        return $s.StartsWith('127.') -or $s.StartsWith('169.254.') -or
               $s.StartsWith('224.') -or $s.StartsWith('255.') -or $s -eq '0.0.0.0'
    }
    $low = $s.ToLower()
    return $low -eq '::1' -or $low -eq '::' -or $low.StartsWith('fe80') -or $low.StartsWith('ff0')
}

Log "starting (tun=$TunName)"

try {
# Must be elevated: adapter creation, routes and DNS all need it.
$isAdmin = ([Security.Principal.WindowsPrincipal]`
    [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(`
    [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) { Log "ERROR: not elevated, refusing"; Finish 2 }

if (-not (Test-Path $HevExe))  { Log "ERROR: hev binary missing: $HevExe"; Finish 1 }
if (-not (Test-Path $HevConfig)) { Log "ERROR: hev config missing: $HevConfig"; Finish 1 }
$HevDir = Split-Path $HevExe -Parent
if (-not (Test-Path (Join-Path $HevDir 'wintun.dll'))) {
    Log "ERROR: wintun.dll missing next to hev.exe"; Finish 1
}

# A previous session's hev, if any (killed below). NOTE: no early exit when
# the adapter is already up — bypass routes belong to the OLD session's
# gateway IPs, and reusing them for this session's Aether remotes would loop
# tunnel traffic back into the TUN. Always rebuild from the live sockets.
$stalePid = $null
if (Test-Path $PidFile) {
    $stalePid = (Get-Content $PidFile -ErrorAction SilentlyContinue | Select-Object -First 1)
}

# Drop bypass routes a previous (possibly crashed) session left behind.
if (Test-Path $StateFile) {
    try {
        $old = Get-Content $StateFile -Raw | ConvertFrom-Json
        foreach ($b in @($old.bypasses)) {
            Remove-NetRoute -DestinationPrefix $b -Confirm:$false -ErrorAction SilentlyContinue | Out-Null
        }
        if (@($old.bypasses).Count -gt 0) { Log "cleared stale bypass routes" }
    } catch { }
}

# Snapshot the current IPv4 default route (fallback when VPN goes down).
$orig = Get-NetRoute -DestinationPrefix '0.0.0.0/0' -ErrorAction SilentlyContinue |
    Sort-Object RouteMetric | Select-Object -First 1
if ($null -eq $orig) { Log "ERROR: no IPv4 default route found"; Finish 1 }
$OrigGw = $orig.NextHop
$OrigIf = $orig.InterfaceIndex
Log "original default via $OrigGw (if $OrigIf)"

# Kill a stale hev from a crashed session, if any.
if ($stalePid -match '^\d+$') {
    $stale = Get-Process -Id $stalePid -ErrorAction SilentlyContinue
    if ($null -ne $stale) {
        Log "stopping stale hev (pid $stalePid)"
        Stop-Process -Id $stalePid -Force -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 1
    }
    Remove-Item $PidFile -Force -ErrorAction SilentlyContinue
}

# Resolve Aether's PID for bypass-route derivation.
if ($AetherPid -eq 0) {
    $AetherPid = (Get-Process -Name 'aether' -ErrorAction SilentlyContinue |
        Select-Object -First 1).Id
    if ($null -eq $AetherPid) { $AetherPid = 0 }
}

Log "starting hev-socks5-tunnel ($HevConfig)"
$hev = Start-Process -FilePath $HevExe -ArgumentList @($HevConfig) `
    -WorkingDirectory $HevDir -WindowStyle Hidden -PassThru
"$($hev.Id)" | Out-File -FilePath $PidFile -Encoding ascii

# Wait for the Wintun adapter hev creates.
$adapter = $null
for ($i = 0; $i -lt 50; $i++) {
    $adapter = Get-NetAdapter -Name $TunName -ErrorAction SilentlyContinue
    if ($null -ne $adapter) { break }
    Start-Sleep -Milliseconds 200
}
if ($null -eq $adapter) {
    Log "ERROR: $TunName did not appear after 10s"
    Stop-Process -Id $hev.Id -Force -ErrorAction SilentlyContinue
    Remove-Item $PidFile -Force -ErrorAction SilentlyContinue
    Finish 1
}
$TunIf = $adapter.ifIndex
Log "adapter $TunName ready (if $TunIf)"

# Loop avoidance: Aether's own remote addresses stay on the physical uplink
# via explicit /32 (/128) bypass routes. --mark exists on Linux/Android only,
# so on Windows this socket-derived list is what keeps tunnel traffic from
# looping back into the TUN. Protocol-agnostic: whatever the outer gateway
# IPs are (MASQUE / WireGuard / gool), they are Aether's live sockets.
$bypasses = @()
if ($AetherPid -ne 0) {
    $remotes = @()
    foreach ($c in (Get-NetTCPConnection -OwningProcess $AetherPid -ErrorAction SilentlyContinue)) {
        $remotes += $c.RemoteAddress
    }
    foreach ($u in (Get-NetUDPEndpoint -OwningProcess $AetherPid -ErrorAction SilentlyContinue)) {
        $remotes += $u.RemoteAddress
    }
    $seen = @{}
    foreach ($r in ($remotes | Sort-Object -Unique)) {
        try { $ip = [System.Net.IPAddress]$r } catch { continue }
        if (IsLoopbackOrLocal $ip) { continue }
        if ($seen.ContainsKey($r)) { continue }
        $seen[$r] = $true
        if ($ip.AddressFamily -eq 'InterNetwork') {
            $prefix = "$r/32"
            New-NetRoute -DestinationPrefix $prefix -NextHop $OrigGw `
                -InterfaceIndex $OrigIf -RouteMetric 1 -ErrorAction SilentlyContinue | Out-Null
        } else {
            $prefix = "$r/128"
            $gw6 = (Get-NetRoute -DestinationPrefix '::/0' -ErrorAction SilentlyContinue |
                Sort-Object RouteMetric | Select-Object -First 1).NextHop
            if ($null -ne $gw6 -and $gw6 -ne '::') {
                New-NetRoute -DestinationPrefix $prefix -NextHop $gw6 `
                    -RouteMetric 1 -ErrorAction SilentlyContinue | Out-Null
            } else { continue }
        }
        $bypasses += $prefix
    }
    Log "bypass routes for Aether remotes: $($bypasses.Count)"
} else {
    Log "WARNING: aether.exe PID unknown, no bypass routes (tunnel may loop)"
}

# Default via TUN with a winning metric; the original default stays as
# fallback (higher metric) so teardown is just a delete. route.exe only
# reports via $LASTEXITCODE — an unchecked failure here would leave the
# backend believing VPN is up while traffic never enters the tunnel.
$hasTunDefault = $null -ne (Get-NetRoute -DestinationPrefix '0.0.0.0/0' `
    -InterfaceIndex $TunIf -ErrorAction SilentlyContinue | Select-Object -First 1)
if (-not $hasTunDefault) {
    route add 0.0.0.0 mask 0.0.0.0 0.0.0.0 metric 5 if $TunIf | Out-Null
    if ($LASTEXITCODE -ne 0) {
        # Stale entry from a crashed session — clear and retry once.
        Get-NetRoute -DestinationPrefix '0.0.0.0/0' -InterfaceIndex $TunIf `
            -ErrorAction SilentlyContinue | Remove-NetRoute -Confirm:$false `
            -ErrorAction SilentlyContinue | Out-Null
        route add 0.0.0.0 mask 0.0.0.0 0.0.0.0 metric 5 if $TunIf | Out-Null
    }
    $hasTunDefault = $null -ne (Get-NetRoute -DestinationPrefix '0.0.0.0/0' `
        -InterfaceIndex $TunIf -ErrorAction SilentlyContinue | Select-Object -First 1)
    if (-not $hasTunDefault) {
        Log "ERROR: failed to move default route onto $TunName"
        Finish 1
    }
    Log "default route via $TunName (metric 5)"
}

# Point the resolver into the tunnel (first two entries of DNS_CSV) and make
# the TUN adapter win resolver selection with a low interface metric.
$servers = @($DnsCsv -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne '' } | Select-Object -First 2)
if ($servers.Count -eq 0) { $servers = @('1.1.1.1', '1.0.0.1') }
Set-DnsClientServerAddress -InterfaceAlias $TunName -ServerAddresses $servers
Set-NetIPInterface -InterfaceAlias $TunName -AddressFamily IPv4 -InterfaceMetric 5
Log "DNS -> $($servers -join ', ')"

@{ gateway = $OrigGw; ifIndex = $OrigIf; bypasses = $bypasses } |
    ConvertTo-Json -Compress | Out-File -FilePath $StateFile -Encoding ascii

Log "UP ($TunName)"
Finish 0
} catch {
    # Fail fast with the sentinel instead of hanging the backend's wait:
    # without this any terminating error means a full-timeout connect.
    Log "ERROR: $($_.Exception.Message)"
    Finish 1
}
