# EndeavourOS / Arch Linux installation guide

## Install

```bash
git clone https://github.com/imcmurray/MagicPad3.git
cd MagicPad3
./scripts/install-endeavouros.sh
```

Then launch **MagicPad Companion** from the app menu (or `magicpad-companion`), replug the trackpad, and open **Status**.

### Uninstall

```bash
./scripts/install-endeavouros.sh uninstall
```

Config under `~/.config/magicpad-companion` is left in place.

That's the whole CLI. Re-run the installer anytime to update or repair.

`scripts/install-linux.sh` is the same script.

---

## What it does

1. Installs packages: WebKit/GTK runtime, `libinput-tools`, `wtype`, …
2. Adds you to the **input** group (daemon uses `sg input` so a full re-login is usually optional)
3. Installs the app to **one** place:
   - `~/.local` if `~/.local/bin` exists  
   - otherwise `/usr/local`  
   (removes leftovers from the other tree)
4. App binary: local release build if you have one under `src-tauri/target/release/`, otherwise latest GitHub **.deb**
5. udev rules for the Magic Trackpad
6. Gesture daemon (`magicpad-gestures.service` under `sg input`) + default `gestures.json`
7. Prints a short checklist

---

## Build from source (optional)

```bash
sudo pacman -S --needed rust nodejs npm webkit2gtk-4.1 base-devel \
  curl wget openssl appmenu-gtk-module libappindicator-gtk3 librsvg \
  libinput-tools wtype

git clone https://github.com/imcmurray/MagicPad3.git
cd MagicPad3
npm install
npm run tauri -- build --bundles deb
./scripts/install-endeavouros.sh    # picks up the local build automatically
```

### Dev loop

```bash
npm install
npm run tauri dev
```

`tauri dev` serves the UI from `http://localhost:1420`. For production IPC and the correct icon, use a full build + the installer.

---

## Multi-finger gestures (Budgie / labwc)

Windows uses the Precision driver. On Linux (labwc), MagicPad runs a small user daemon:

```
libinput debug-events  →  swipes / taps / pinch  →  wtype / commands
```

### Defaults

| Gesture | Action |
|---------|--------|
| 3-finger swipe L/R | Prev/Next desktop |
| 3-finger swipe up/down | App switcher / Alt+Tab |
| 4-finger swipe L/R | Browser back/forward |
| 4-finger swipe up/down | Show desktop / Raven |
| Pinch out/in | Zoom in/out |
| 3-finger tap | Budgie Screenshot |
| 4-finger tap | Unbound — set **Custom** (e.g. `flameshot gui`) |

### Status / repair

```bash
systemctl --user status magicpad-gestures.service
systemctl --user restart magicpad-gestures.service
journalctl --user -u magicpad-gestures.service -f
./scripts/install-endeavouros.sh    # full repair
```

If Status claims you're not in the **input** group after `usermod`, reinstall a current build — the checklist uses account membership (`getent`), and the daemon runs under `sg input`.

### Custom: Flameshot on 4-finger tap

1. `sudo pacman -S flameshot`
2. Gestures → **4-finger Tap** → **Custom** → `flameshot gui` → **Save**

---

## Manual package check

```bash
sudo pacman -S --needed webkit2gtk-4.1 gtk3 libappindicator-gtk3 \
  librsvg xdg-utils binutils tar curl polkit libinput-tools wtype
```

## input-remapper (optional advanced)

```bash
sudo pacman -S input-remapper   # or AUR
systemctl enable --now input-remapper
```

Profiles may be staged under:

```
~/.config/input-remapper-2/presets/Magic Trackpad/MagicPad.json
~/.config/magicpad-companion/input-remapper/MagicPad.json
```

## Verify by hand

```bash
lsusb | grep -i 05ac
libinput list-devices | grep -i -A5 trackpad
magicpad-companion
systemctl --user status magicpad-gestures.service
```

## Other distros

- **Fedora / openSUSE**: build from source or extract the DEB; adjust packages.  
- **Ubuntu 24.04+**: `sudo apt install ./MagicPad.Companion_*_amd64.deb` from [Releases](https://github.com/imcmurray/MagicPad3/releases).  

## Wayland note

The app sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` by default on Linux to avoid WebKitGTK crashes on some Budgie/labwc setups.
