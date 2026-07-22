# Windows 11 installation guide

## Requirements

- Windows 11 (Windows 10 AMD64 possible with upstream caveats)
- Administrator account for driver install
- Magic Trackpad 3 (USB-C) or Magic Trackpad 2 (Lightning / BT)

## Install MagicPad Companion

1. Download the latest **NSIS EXE** or **MSI** from GitHub Releases.
2. Run the installer (per-user default).
3. Launch **MagicPad Companion**.

### Build from source

```powershell
# Prerequisites: Rust (MSVC), Node.js 20+, WebView2
git clone https://github.com/imcmurray/MagicPad3.git
cd MagicPad3
npm install
npm run tauri build
# Artifacts under src-tauri\target\release\bundle\
```

## Precision driver (gestures)

1. Uninstall **Magic Utilities** and **Trackpad++** if present.
2. Download [MagicTrackpad2ForWindows](https://github.com/vitoplantamura/MagicTrackpad2ForWindows/releases).
3. Extract to `%LOCALAPPDATA%\MagicPadCompanion\drivers\` (keep `AMD64` / `ARM64` folders).
4. In the app: **Driver → Install driver** (elevate when prompted).
5. Replug USB-C or re-pair Bluetooth.

### USB-C ID

Device Manager should show hardware ID containing:

```
VID_05AC&PID_0324
```

## Verify

- **Status** tab shows connected device + connection type (USB-C / Bluetooth).
- Three/four-finger gestures work as Windows Precision Touchpad.
- Battery may appear when the driver exposes it.

## Uninstall driver

Use **Driver → Uninstall** for guidance, or:

1. Device Manager → uninstall Apple / AmtPtp Precision devices.
2. [DriverStore Explorer](https://github.com/lostindark/DriverStoreExplorer) to purge packages.
