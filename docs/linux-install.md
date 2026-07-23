# EndeavourOS / Arch Linux installation guide

## Quick install (recommended)

One script installs runtime deps, the latest GitHub **.deb** release (extracted for Arch), desktop entry, icons, and **udev** helpers:

```bash
git clone https://github.com/imcmurray/MagicPad3.git
cd MagicPad3
chmod +x scripts/install-endeavouros.sh
./scripts/install-endeavouros.sh
```

Then:

```bash
magicpad-companion
# or open “MagicPad Companion” from the app menu
```

Replug the Magic Trackpad (or re-pair Bluetooth) and open the **Status** tab.

### Installer options

| Flag | Effect |
|------|--------|
| *(default)* | Full install from latest GitHub release **+ gesture daemon** |
| `--local` | Use a local `tauri build` under `src-tauri/target/release/` |
| `--deb PATH` | Install from a specific `.deb` file |
| `--user` | Install app under `~/.local` (udev still needs root) |
| `--helpers` | udev + remapper staging **+ gesture daemon** |
| `--gestures` | Gesture daemon only (app already installed) |
| `--no-gestures` | Full/helpers install without starting the daemon |
| `--with-remapper` | Also `pacman -S input-remapper` if available |
| `--skip-deps` | Skip pacman package install |
| `--uninstall` | Remove app, udev rule, and gesture service |

Examples:

```bash
# Helpers only (udev + gestures), if the app is already installed
./scripts/install-endeavouros.sh --helpers

# Re-enable / repair the gesture daemon only
./scripts/install-endeavouros.sh --gestures

# After a local production build
npm run tauri -- build --bundles deb
./scripts/install-endeavouros.sh --local

# User-local binary (no root for app files)
./scripts/install-endeavouros.sh --user
```

`scripts/install-linux.sh` is a thin wrapper around the same script.

---

## Requirements

- Modern kernel with multitouch / `hid-magicmouse` support  
- User session with libinput (Budgie, GNOME, KDE, labwc/wlroots, etc.)  
- Optional: `input-remapper`, `polkit`

### Runtime packages (installed by the script)

```bash
sudo pacman -S --needed webkit2gtk-4.1 gtk3 libappindicator-gtk3 \
  librsvg xdg-utils binutils tar curl polkit
```

## Manual install from a release `.deb`

Arch does not use dpkg by default. The installer extracts the DEB:

```bash
# Or let the script download it
curl -LO https://github.com/imcmurray/MagicPad3/releases/latest/download/MagicPad.Companion_0.2.2_amd64.deb
./scripts/install-endeavouros.sh --deb ./MagicPad.Companion_0.2.2_amd64.deb
```

## Build from source

```bash
sudo pacman -S --needed rust nodejs npm webkit2gtk-4.1 base-devel \
  curl wget openssl appmenu-gtk-module libappindicator-gtk3 librsvg

git clone https://github.com/imcmurray/MagicPad3.git
cd MagicPad3
npm install
npm run tauri -- build --bundles deb
./scripts/install-endeavouros.sh --local
```

### Dev loop

```bash
npm install
npm run tauri dev
# or after a release build:
./scripts/run.sh
```

## System helpers (udev)

Installed automatically by the full installer. Manual:

```bash
./scripts/install-endeavouros.sh --helpers
# or: Driver → Install helpers in the app (pkexec)
```

Add yourself to the `input` group if event nodes are restricted:

```bash
sudo usermod -aG input "$USER"
# re-login
```

## Multi-finger gestures (Budgie / labwc)

Windows uses the Precision driver for gestures. On Linux, **labwc does not
bind 3/4-finger swipes natively**, so MagicPad runs a small user daemon:

```
libinput debug-events  →  detect swipes  →  wtype (Super+Page Up/Down, …)
```

### One-time setup (installer does this)

The EndeavourOS installer (`./scripts/install-endeavouros.sh`) by default:

1. Installs **libinput-tools** and **wtype**
2. Adds your user to the **input** group
3. Seeds `~/.config/magicpad-companion/gestures.json` (if missing)
4. Installs and enables **`magicpad-gestures.service`** (user systemd)
5. Adds an XDG autostart entry as backup

```bash
# Full install (includes daemon)
./scripts/install-endeavouros.sh

# Or only the daemon, if the app is already installed
./scripts/install-endeavouros.sh --gestures
```

**Log out and back in** once if you were just added to `input`.

### Enable / repair from the app

1. Open **Gestures**
2. Confirm daemon checklist (libinput-tools / wtype / input group)
3. Click **Save gestures** (or **Start daemon**)

```bash
systemctl --user status magicpad-gestures.service
systemctl --user restart magicpad-gestures.service
```

### Default Budgie/labwc mappings

| Gesture | Action | Shortcut injected |
|---------|--------|-------------------|
| 3-finger swipe L/R | Prev/Next desktop | Super+Page Up/Down |
| 3-finger swipe up | App switcher | Super+Tab |
| 3-finger swipe down | Alt+Tab | Alt+Tab |
| 4-finger swipe up | Show desktop | Super+D |
| 4-finger swipe down | Raven panel | Super+A |
| Pinch out | Zoom in | Ctrl+= |
| Pinch in | Zoom out | Ctrl+- |
| 3-finger tap | Budgie Screenshot | `org.buddiesofbudgie.BudgieScreenshot -i` |
| 4-finger tap | Budgie Screenshot | same |

Pinch zoom works in Firefox, Chromium, many Electron apps, LibreOffice, etc.
Focus the window you want to zoom first. Multi-finger taps use libinput
**hold** gestures (short hold ≈ tap).

### Manual test

```bash
# should print GESTURE_SWIPE_* when you 3-finger swipe
libinput debug-events
# inject a workspace switch
wtype -M logo -k Prior -m logo
magicpad-companion --gestures   # foreground daemon
```

## input-remapper (optional advanced)

```bash
sudo pacman -S input-remapper   # or AUR
systemctl enable --now input-remapper
# or: ./scripts/install-endeavouros.sh --with-remapper
```

Profiles are staged under:

```
~/.config/input-remapper-2/presets/Magic Trackpad/MagicPad.json
~/.config/magicpad-companion/input-remapper/MagicPad.json
```

## Verify

```bash
lsusb | grep -i 05ac
libinput list-devices | grep -i -A5 trackpad
magicpad-companion
```

In the app **Status** tab you should see a single Magic Trackpad entry (USB-C / Bluetooth).

## Uninstall

```bash
./scripts/install-endeavouros.sh --uninstall
# Config left at: ~/.config/magicpad-companion
```

## Other distros

- **Fedora / openSUSE**: build from source or extract the DEB manually; adjust packages.  
- **Ubuntu 24.04+**: install the release `.deb` with `sudo apt install ./MagicPad.Companion_*_amd64.deb`.  
- Settings apply via `gsettings` / KDE config when available; otherwise companion JSON is the source of truth.

## Wayland note

The app sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` by default on Linux to avoid WebKitGTK crashes on some Budgie/labwc setups.
