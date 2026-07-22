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
| *(default)* | Full install from latest GitHub release |
| `--local` | Use a local `tauri build` under `src-tauri/target/release/` |
| `--deb PATH` | Install from a specific `.deb` file |
| `--user` | Install app under `~/.local` (udev still needs root) |
| `--helpers` | Only udev + remapper profile + user unit stub |
| `--with-remapper` | Also `pacman -S input-remapper` if available |
| `--skip-deps` | Skip pacman package install |
| `--uninstall` | Remove app files + udev rule |

Examples:

```bash
# Helpers only (udev), if the app is already installed
./scripts/install-endeavouros.sh --helpers

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

## input-remapper (optional advanced gestures)

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
