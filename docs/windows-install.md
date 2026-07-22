# Windows 11 — install & home PC testing

This guide is the path for **personal home PC testing** of MagicPad Companion plus the Microsoft-signed Precision Touchpad driver.

You do **not** need to sign drivers yourself. Use the upstream signed package (see [driver-notes.md](../packaging/windows/driver-notes.md)).

---

## Quick path (recommended)

### A. One-time folders + checklist

In PowerShell (from a clone of this repo):

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\windows-home-setup.ps1
```

That creates:

```
%LOCALAPPDATA%\MagicPadCompanion\drivers\   ← put extracted INF tree here
%APPDATA%\MagicPadCompanion\                ← app settings
```

…and can open the right GitHub Releases pages.

### B. Install MagicPad Companion

1. Open [GitHub Releases](https://github.com/imcmurray/MagicPad3/releases).
2. Download either:
   - **NSIS setup** (`MagicPad Companion_*_x64-setup.exe`) — easiest, or  
   - **Portable** `magicpad-companion.exe` — no installer
3. Run it.  
   - **SmartScreen** may say “Windows protected your PC” because the app is not Authenticode-signed yet.  
   - **More info → Run anyway** is expected for these test builds.
4. Launch **MagicPad Companion**.

### C. Precision driver (gestures / battery / USB-C)

1. Uninstall **Magic Utilities** and **Trackpad++** if present.
2. Download the latest release from  
   [vitoplantamura/MagicTrackpad2ForWindows](https://github.com/vitoplantamura/MagicTrackpad2ForWindows/releases)  
   (Microsoft-signed Precision packages; USB-C + Bluetooth lineage).
3. Extract so architecture folders land under the drivers path:

   ```
   %LOCALAPPDATA%\MagicPadCompanion\drivers\
     AMD64\   ← Intel/AMD 64-bit Windows (most home PCs)
     ARM64\   ← Snapdragon / ARM Windows only
   ```

   You need at least one `.inf` under `AMD64` or `ARM64` (or nested under that tree).

4. In the app: **Driver → Install driver**.  
   - Approve elevation if prompted, or re-run the app **as Administrator** once if `pnputil` fails.
5. Unplug/replug USB-C **or** remove + re-pair Bluetooth.
6. **Status** should show the trackpad; gestures should follow Windows Precision Touchpad behavior.

### USB-C Magic Trackpad 3 check

Device Manager → device → Details → Hardware Ids should include something like:

```
USB\VID_05AC&PID_0324
```

---

## Build from source (if no release artifact yet)

Prerequisites:

- [Rust](https://rustup.rs/) stable (MSVC toolchain)
- [Node.js 20+](https://nodejs.org/)
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with “Desktop development with C++”
- WebView2 (preinstalled on Windows 11)

```powershell
git clone https://github.com/imcmurray/MagicPad3.git
cd MagicPad3
npm install
npm run tauri -- build --bundles nsis,msi
```

Artifacts:

| Kind | Path |
|------|------|
| NSIS installer | `src-tauri\target\release\bundle\nsis\` |
| MSI | `src-tauri\target\release\bundle\msi\` |
| Portable EXE | `src-tauri\target\release\magicpad-companion.exe` |

Dev loop:

```powershell
npm install
npm run tauri dev
```

CI also builds Windows installers on every push to `main` — download the **windows-installers** artifact from the Actions run if a Release is not published yet:

https://github.com/imcmurray/MagicPad3/actions

---

## Verify checklist

| Check | Expected |
|-------|----------|
| Status tab | Device connected (USB-C / USB / Bluetooth) |
| Driver tab | Installed / no conflict with Magic Utilities |
| Gestures | 3/4-finger Windows PTP gestures work |
| Settings | Apply persists (app JSON; system where supported) |
| Logs | No repeated `pnputil` failures after admin install |
| Device Manager | Precision / AmtPtp binding; signed driver package |

---

## Uninstall

**App:** Windows Settings → Apps → MagicPad Companion, or delete the portable EXE.

**Driver:**

1. App **Driver → Uninstall** for package list guidance, or  
2. Device Manager → uninstall Apple / AmtPtp Precision devices, or  
3. [DriverStore Explorer](https://github.com/lostindark/DriverStoreExplorer) to purge packages, then reboot.

---

## Troubleshooting (Windows)

| Symptom | Fix |
|---------|-----|
| SmartScreen block | More info → Run anyway (unsigned test build) |
| Install driver fails | Run app as Administrator once; confirm `.inf` under `drivers\AMD64` |
| No device in Status | Cable data lines; another port; Device Manager VID_05AC |
| Gestures missing | Driver not bound; conflict software; replug / re-pair |
| Battery blank | Expected until PTP stack exposes battery; still useful for gestures |
| BT flaky | Pair while charging via USB-C; remove device and pair again |

More: [troubleshooting.md](./troubleshooting.md)

---

## Security notes for home testing

- Prefer the **Microsoft-signed** vitoplantamura package; avoid test-signing mode and random mirror zips.
- Only place driver trees you downloaded yourself into `%LOCALAPPDATA%\MagicPadCompanion\drivers\`.
- The companion app is not code-signed yet — SmartScreen warnings are expected until an Authenticode pipeline exists.
- Driver binaries are **not** shipped inside this git repo (license / signing / size).
