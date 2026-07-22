# EndeavourOS / Arch Linux installation guide

## Requirements

- Modern kernel with multitouch / `hid-magicmouse` support  
- User session with libinput (Budgie, GNOME, KDE, labwc/wlroots, etc.)  
- Optional: `input-remapper`, `polkit`

## Install MagicPad Companion

### AppImage / DEB (releases)

```bash
# AppImage
chmod +x MagicPad_Companion_*.AppImage
./MagicPad_Companion_*.AppImage

# Or install DEB on Debian-family; on Arch prefer AppImage or local build
```

### Build from source (EndeavourOS / Arch)

```bash
sudo pacman -S --needed rust nodejs npm webkit2gtk-4.1 base-devel \
  curl wget openssl appmenu-gtk-module libappindicator-gtk3 librsvg
# Tauri 2 also needs: https://v2.tauri.app/start/prerequisites/

git clone https://github.com/imcmurray/MagicPad3.git
cd MagicPad3
npm install
npm run tauri build
# AppImage/DEB/RPM under src-tauri/target/release/bundle/
```

### Dev loop

```bash
npm install
npm run tauri dev
```

## System helpers (udev)

```bash
chmod +x scripts/install-linux.sh
./scripts/install-linux.sh
# or: Driver → Install helpers in the app (pkexec)
```

Add yourself to the `input` group if event nodes are restricted:

```bash
sudo usermod -aG input "$USER"
# re-login
```

## input-remapper (optional advanced gestures)

```bash
sudo pacman -S input-remapper
systemctl enable --now input-remapper.service
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
```

## Other distros

- **Fedora / openSUSE**: AppImage or build with distro WebKitGTK packages.  
- **Ubuntu 24.04+**: DEB target from `tauri build` when built on Debian-like host.  
- Settings apply via `gsettings` / KDE config when available; otherwise companion JSON is the source of truth.
