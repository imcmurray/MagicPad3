# MagicPad Companion — home Windows PC setup helper
# Run in PowerShell (user account is fine; driver install may prompt for admin later).
#
#   Set-ExecutionPolicy -Scope Process Bypass
#   .\scripts\windows-home-setup.ps1
#
# Or from a clone:
#   irm https://raw.githubusercontent.com/imcmurray/MagicPad3/main/scripts/windows-home-setup.ps1 | iex
#   (prefer downloading the script first so you can read it)

$ErrorActionPreference = "Stop"

$AppLocal = Join-Path $env:LOCALAPPDATA "MagicPadCompanion"
$Drivers  = Join-Path $AppLocal "drivers"
$Config   = Join-Path $env:APPDATA "MagicPadCompanion"

$DriverReleases = "https://github.com/vitoplantamura/MagicTrackpad2ForWindows/releases"
$AppReleases    = "https://github.com/imcmurray/MagicPad3/releases"
$AppRepo        = "https://github.com/imcmurray/MagicPad3"
$Docs           = "https://github.com/imcmurray/MagicPad3/blob/main/docs/windows-install.md"

Write-Host ""
Write-Host "MagicPad Companion — home PC setup" -ForegroundColor Cyan
Write-Host "=================================" -ForegroundColor Cyan
Write-Host ""

New-Item -ItemType Directory -Force -Path $Drivers | Out-Null
New-Item -ItemType Directory -Force -Path $Config  | Out-Null

Write-Host "Created / verified:"
Write-Host "  Drivers folder : $Drivers"
Write-Host "  Config folder  : $Config"
Write-Host ""

Write-Host "Next steps (manual — keeps you in control of what runs as admin):" -ForegroundColor Yellow
Write-Host ""
Write-Host "1. Install MagicPad Companion"
Write-Host "   - Download NSIS .exe or magicpad-companion.exe from:"
Write-Host "     $AppReleases"
Write-Host "   - If SmartScreen warns: More info → Run anyway (unsigned test build)."
Write-Host ""
Write-Host "2. Get the Microsoft-signed Precision driver"
Write-Host "   - Open: $DriverReleases"
Write-Host "   - Download the latest Windows release zip for your CPU (AMD64 or ARM64)."
Write-Host "   - Extract so you have something like:"
Write-Host "       $Drivers\AMD64\*.inf"
Write-Host "     or"
Write-Host "       $Drivers\ARM64\*.inf"
Write-Host ""
Write-Host "3. Uninstall conflicts first (if present)"
Write-Host "   - Magic Utilities"
Write-Host "   - Trackpad++"
Write-Host "   - Old test-mode imbushuo installs"
Write-Host ""
Write-Host "4. In MagicPad Companion"
Write-Host "   - Status  → confirm trackpad appears after plug/pair"
Write-Host "   - Driver  → Install driver  (approve UAC / use Admin if needed)"
Write-Host "   - Replug USB-C or re-pair Bluetooth"
Write-Host "   - Settings / Gestures as desired"
Write-Host ""
Write-Host "5. Verify"
Write-Host "   - Device Manager shows Precision Touchpad / AmtPtp binding"
Write-Host "   - 3/4-finger gestures work"
Write-Host "   - HWID includes VID_05AC (USB-C MT3 often PID_0324)"
Write-Host ""
Write-Host "Docs: $Docs"
Write-Host "Repo: $AppRepo"
Write-Host ""

$open = Read-Host "Open Releases pages in browser now? [Y/n]"
if ([string]::IsNullOrWhiteSpace($open) -or $open -match '^[Yy]') {
    Start-Process $AppReleases
    Start-Process $DriverReleases
    Start-Process "explorer.exe" $Drivers
}

Write-Host ""
Write-Host "Done. Drivers folder is ready for the extracted INF tree." -ForegroundColor Green
Write-Host ""
