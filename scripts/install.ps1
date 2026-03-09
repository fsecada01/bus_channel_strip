# install.ps1 — Copy VST3 and CLAP bundles to system plugin directories.
# Run from the repo root: powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install.ps1
param(
    [string]$Vst3Dir = "C:\Program Files\Common Files\VST3",
    [string]$ClapDir = "C:\Program Files\Common Files\CLAP"
)

$ErrorActionPreference = "Stop"

# Admin check
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "ERROR: admin rights required - re-run from an elevated terminal (Run as Administrator)"
    exit 1
}

# VST3
$vst3Src = "target\bundled\Bus-Channel-Strip.vst3\Contents\x86_64-win\Bus-Channel-Strip.vst3"
$vst3Dst = "$Vst3Dir\Bus-Channel-Strip.vst3\Contents\x86_64-win\Bus-Channel-Strip.vst3"
if (-not (Test-Path $vst3Src)) {
    Write-Host "ERROR: $vst3Src not found - run 'just bundle' first"
    exit 1
}
New-Item -ItemType Directory -Force (Split-Path $vst3Dst) | Out-Null
Copy-Item -Force $vst3Src $vst3Dst
if (Test-Path $vst3Dst) {
    $mb = [math]::Round((Get-Item $vst3Dst).Length / 1MB, 1)
    Write-Host "  [OK] VST3 installed ($mb MB) -> $vst3Dst"
} else {
    Write-Host "ERROR: VST3 copy failed - check permissions"
    exit 1
}

# CLAP
$clapSrc = "target\bundled\Bus-Channel-Strip.clap"
$clapDst = "$ClapDir\Bus-Channel-Strip.clap"
if (-not (Test-Path $clapSrc)) {
    Write-Host "ERROR: $clapSrc not found - run 'just bundle' first"
    exit 1
}
New-Item -ItemType Directory -Force $ClapDir | Out-Null
Copy-Item -Force $clapSrc $clapDst
if (Test-Path $clapDst) {
    $mb = [math]::Round((Get-Item $clapDst).Length / 1MB, 1)
    Write-Host "  [OK] CLAP installed ($mb MB) -> $clapDst"
} else {
    Write-Host "ERROR: CLAP copy failed - check permissions"
    exit 1
}

Write-Host "  Done. Rescan plugins in your DAW."
