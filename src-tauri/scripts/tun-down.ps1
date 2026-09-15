# Tear down the Aethery VPN on Windows: remove the TUN route/bypasses, reset
# DNS, stop hev.
#
# Runs ELEVATED via UAC. Same log/sentinel contract as tun-up.ps1 (see it).
# Idempotent and safe when already down.
#
# Args: STATE_DIR TUN_NAME
param(
    [string]$StateDir,
    [string]$TunName
)

$ErrorActionPreference = 'Stop'
$LogFile   = Join-Path $StateDir 'tun-last.log'
$PidFile   = Join-Path $StateDir 'hev.pid'
$StateFile = Join-Path $StateDir 'tun-net.json'
$ExitFile  = Join-Path $StateDir 'tun-exit.code'

if (Test-Path $LogFile)  { Remove-Item $LogFile -Force -ErrorAction SilentlyContinue }
if (Test-Path $ExitFile) { Remove-Item $ExitFile -Force -ErrorAction SilentlyContinue }

function Log($m) {
    "[tun-down] $m" | Out-File -FilePath $LogFile -Append -Encoding ascii
}

function Finish($code) {
    "$code" | Out-File -FilePath $ExitFile -Encoding ascii
    exit $code
}

Log "stopping (tun=$TunName)"

try {
$isAdmin = ([Security.Principal.WindowsPrincipal]`
    [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(`
    [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) { Log "ERROR: not elevated, refusing"; Finish 2 }

# Routes first, while the adapter still exists for index-scoped deletes.
$tunIf = (Get-NetAdapter -Name $TunName -ErrorAction SilentlyContinue).ifIndex
if ($null -ne $tunIf) {
    Get-NetRoute -DestinationPrefix '0.0.0.0/0' -InterfaceIndex $tunIf `
        -ErrorAction SilentlyContinue | Remove-NetRoute -Confirm:$false `
        -ErrorAction SilentlyContinue | Out-Null
    Log "TUN default route removed"
}

$bypassCount = 0
if (Test-Path $StateFile) {
    try {
        $old = Get-Content $StateFile -Raw | ConvertFrom-Json
        foreach ($b in @($old.bypasses)) {
            Remove-NetRoute -DestinationPrefix $b -Confirm:$false `
                -ErrorAction SilentlyContinue | Out-Null
            $bypassCount++
        }
    } catch { }
}
if ($bypassCount -gt 0) { Log "bypass routes removed ($bypassCount)" }

if ($null -ne (Get-NetAdapter -Name $TunName -ErrorAction SilentlyContinue)) {
    Set-DnsClientServerAddress -InterfaceAlias $TunName -ResetServerAddresses `
        -ErrorAction SilentlyContinue | Out-Null
    Log "TUN DNS reset"
}

if (Test-Path $PidFile) {
    # NOTE: never name this $pid — $PID is a read-only automatic variable
    # and assigning it throws under Stop preference.
    $hevPid = (Get-Content $PidFile -ErrorAction SilentlyContinue | Select-Object -First 1)
    if ($hevPid -match '^\d+$') {
        $hev = Get-Process -Id $hevPid -ErrorAction SilentlyContinue
        if ($null -ne $hev) {
            Log "stopping hev (pid $hevPid)"
            Stop-Process -Id $hevPid -Force -ErrorAction SilentlyContinue
            Start-Sleep -Seconds 1
        }
    }
    Remove-Item $PidFile -Force -ErrorAction SilentlyContinue
}

# The Wintun adapter disappears with hev; note leftovers either way.
if ($null -ne (Get-NetAdapter -Name $TunName -ErrorAction SilentlyContinue)) {
    Log "WARNING: $TunName adapter still present"
}

Remove-Item $StateFile -Force -ErrorAction SilentlyContinue
Log "DOWN"
Finish 0
} catch {
    # Fail fast with the sentinel instead of hanging the backend's wait:
    # without this any terminating error means a full-timeout disconnect.
    Log "ERROR: $($_.Exception.Message)"
    Finish 1
}
