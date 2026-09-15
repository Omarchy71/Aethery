# Downloads the pinned hev-socks5-tunnel Windows build (tun2socks for VPN
# mode) into src-tauri/binaries/. Mirrors fetch-hev.sh conventions: the
# version pin is read from fetch-hev.sh so the two tracks never drift.
#
# Places: hev.exe, wintun.dll, msys-2.0.dll (all three ship side-by-side —
# the exe needs both DLLs in its own directory at runtime).
# Run from the repo root:  powershell -ExecutionPolicy Bypass -File src-tauri/binaries/fetch-hev.ps1
$ErrorActionPreference = 'Stop'

$DestDir = Split-Path $MyInvocation.MyCommand.Path -Parent

$pinLine = Get-Content (Join-Path $DestDir 'fetch-hev.sh') |
    Where-Object { $_ -match '^HEV_VERSION=' } | Select-Object -First 1
if ($null -eq $pinLine -or $pinLine -notmatch '"([^"]+)"') {
    throw 'Could not read HEV_VERSION from fetch-hev.sh'
}
$HevVersion = $Matches[1]
$Asset = 'hev-socks5-tunnel-win64.zip'
$Url = "https://github.com/heiher/hev-socks5-tunnel/releases/download/$HevVersion/$Asset"

Push-Location $DestDir
try {
    curl.exe -sL -o $Asset $Url
    $size = (Get-Item $Asset).Length
    if ($size -lt 1000000) { throw "Downloaded $Asset looks wrong ($size bytes)" }

    if (Test-Path 'hev-win-tmp') { Remove-Item 'hev-win-tmp' -Recurse -Force }
    Expand-Archive $Asset -DestinationPath 'hev-win-tmp'
    $inner = Get-ChildItem 'hev-win-tmp' -Directory | Select-Object -First 1

    Move-Item (Join-Path $inner.FullName 'hev-socks5-tunnel.exe') 'hev.exe' -Force
    Move-Item (Join-Path $inner.FullName 'wintun.dll') 'wintun.dll' -Force
    $msys = Join-Path $inner.FullName 'msys-2.0.dll'
    if (Test-Path $msys) { Move-Item $msys 'msys-2.0.dll' -Force }

    Remove-Item 'hev-win-tmp' -Recurse -Force
    Remove-Item $Asset -Force
    Write-Host "hev-socks5-tunnel $HevVersion ready at $DestDir\hev.exe (+ wintun.dll)"
} finally {
    Pop-Location
}
