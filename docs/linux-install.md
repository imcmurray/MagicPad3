# EndeavourOS / Arch Linux installation guide

## Quick install (recommended)

One script installs runtime deps, the latest GitHub **.deb** release (extracted for Arch), desktop entry, icons, **udev** helpers, and the **gesture daemon**:

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
./scripts/install-endeavouros.sh --verify   # checklist
```

Replug the Magic Trackpad (or re-pair Bluetooth) and open the **Status** tab.

### Installer options

| Flag | Effect |
|------|--------|
| *(default)* | Full install from latest GitHub release **+ gesture daemon** |
| `--local` | Use a local `tauri build` under `src-tauri/target/release/` |
| `--deb PATH` | Install from a specific `.deb` file |
| `--user` | Force install under `~/.local` (create bin dir if needed) |
| `--system` | Force install under `/usr/local` |
| `--helpers` | udev + remapper staging **+ gesture daemon** |
| `--gestures` | Gesture daemon only (app already installed) |
| `--no-gestures` | Full/helpers install without starting the daemon |
| `--verify` | Post-install checklist (binary, packages, input group, unit, daemon) |
| `--with-remapper` | Also `pacman -S input-remapper` if available |
| `--skip-deps` | Skip pacman package install |
| `--uninstall` | Remove app (all known paths), udev rule, and gesture service |

**Install location (exactly one, never both):**

- If `~/.local/bin` **exists** → install there (no root for the app binary)
- Otherwise → `/usr/local/bin`
- Re-running the installer **removes** any leftover copy in the other tree

Examples:

```bash
# Helpers only (udev + gestures), if the app is already installed
./scripts/install-endeavouros.sh --helpers

# Re-enable / repair the gesture daemon only
./scripts/install-endeavouros.sh --gestures

# After a local production build
npm run tauri -- build --bundles deb
./scripts/install-endeavouros.sh --local

# Force system prefix even if ~/.local/bin exists
./scripts/install-endeavouros.sh --system

# Checklist only
./scripts/install-endeavouros.sh --verify
```

`scripts/install-linux.sh` is a thin wrapper around the same script.

### What the installer always does

1. **Packages**: WebKit/GTK runtime + `libinput-tools` + `wtype` (+ `binutils` to unpack release `.deb`)
2. **`input` group**: `usermod -aG input` (account membership via getent — not session groups)
3. **App binary**: **one** location only — `~/.local` if `~/.local/bin` exists, else `/usr/local` (purges the other)
4. **udev** rule for Magic Trackpad
5. **Gesture daemon**:
   - Seeds `~/.config/magicpad-companion/gestures.json` if missing
   - Writes `~/.config/systemd/user/magicpad-gestures.service` with  
     `ExecStart=/usr/bin/sg input -c '…/magicpad-companion --gestures'`
   - XDG autostart with the same `sg input` wrapper
   - `systemctl --user enable --now magicpad-gestures.service`
6. **Verify** checklist at the end of full / helpers / gestures installs

`sg input` matters because session supplementary groups lag until re-login after `usermod`. Account membership in `/etc/group` is enough for the daemon.

---

## Requirements

- Modern kernel with multitouch / `hid-magicmouse` support  
- User session with libinput (Budgie, GNOME, KDE, labwc/wlroots, etc.)  
- Optional: `input-remapper`, `polkit`

### Runtime packages (installed by the script)

```bash
sudo pacman -S --needed webkit2gtk-4.1 gtk3 libappindicator-gtk3 \
  librsvg xdg-utils binutils tar curl polkit \
  libinput-tools wtype
```

## Manual install from a release `.deb`

Arch does not use dpkg by default. The installer extracts the DEB:

```bash
# Or let the script download it
curl -LO https://github.com/imcmurray/MagicPad3/releases/latest/download/MagicPad.Companion_0.3.3_amd64.deb
./scripts/install-endeavouros.sh --deb ./MagicPad.Companion_0.3.3_amd64.deb
```

## Build from source

```bash
sudo pacman -S --needed rust nodejs npm webkit2gtk-4.1 base-devel \
  curl wget openssl appmenu-gtk-module libappindicator-gtk3 librsvg \
  libinput-tools wtype

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

**Note:** `tauri dev` serves the UI from `http://localhost:1420` (dev CSP). For production IPC and the correct app icon, use a full `tauri build` + `--local` install.

## System helpers (udev)

Installed automatically by the full installer. Manual:

```bash
./scripts/install-endeavouros.sh --helpers
# or: Driver → Install helpers in the app (pkexec)
```

Add yourself to the `input` group if event nodes are restricted:

```bash
sudo usermod -aG input "$USER"
# Account membership is enough for the daemon (sg input).
# Full re-login only needed for *this shell's* `id -nG` to list input.
systemctl --user restart magicpad-gestures.service
```

## Multi-finger gestures (Budgie / labwc)

Windows uses the Precision driver for gestures. On Linux, **labwc does not
bind 3/4-finger swipes natively**, so MagicPad runs a small user daemon:

```
libinput debug-events  →  detect swipes/taps/pinch  →  wtype / commands
```

### One-time setup (installer does this)

```bash
# Full install (includes daemon)
./scripts/install-endeavouros.sh

# Or only the daemon, if the app is already installed
./scripts/install-endeavouros.sh --gestures
```

### Enable / repair from the app

1. Open **Gestures**
2. Confirm daemon checklist (libinput-tools / wtype / input group)
3. Click **Save gestures** (or **Start daemon**)

```bash
systemctl --user status magicpad-gestures.service
systemctl --user restart magicpad-gestures.service
journalctl --user -u magicpad-gestures.service -f
```

### Default Budgie/labwc mappings

| Gesture | Action | Shortcut / command |
|---------|--------|--------------------|
| 3-finger swipe L/R | Prev/Next desktop | Super+Page Up/Down |
| 3-finger swipe up | App switcher | Super+Tab |
| 3-finger swipe down | Alt+Tab | Alt+Tab |
| 4-finger swipe L/R | Browser back/forward | XF86Back / mouse 8–9 cascade |
| 4-finger swipe up | Show desktop | Super+D |
| 4-finger swipe down | Raven panel | Super+A |
| Pinch out | Zoom in | Ctrl+= |
| Pinch in | Zoom out | Ctrl+- |
| 3-finger tap | Budgie Screenshot | `org.buddiesofbudgie.BudgieScreenshot -i` |
| 4-finger tap | *(unbound)* | Set **Custom** in the app (e.g. Flameshot) |

Pinch zoom works in Firefox, Chromium, many Electron apps, LibreOffice, etc.
Focus the window you want to zoom first. Multi-finger taps use libinput
**hold** gestures and require **3+ fingers** (1-finger holds are ignored so a
normal click does not open Screenshot).

### Custom command example: Flameshot on 4-finger tap

1. Install Flameshot: `sudo pacman -S flameshot`
2. Open MagicPad → **Gestures**
3. Find **4-finger Tap**
4. Action → **Custom**
5. Command field → `flameshot gui`
6. **Save gestures** (restarts the daemon)

Other useful Flameshot commands:

```bash
flameshot gui          # interactive region
flameshot full         # full screen
flameshot gui -d 2000  # delay 2s then capture
```

You can put any shell command in **Custom** (e.g. `kitty`, `notify-send 'hi'`, a script path).

### Manual test

```bash
# should print GESTURE_SWIPE_* when you 3-finger swipe
libinput debug-events
# inject a workspace switch
wtype -M logo -k Prior -m logo
# foreground daemon (uses current shell groups — prefer the systemd unit with sg)
sg input -c 'magicpad-companion --gestures'
```

### Troubleshooting “not in input group” in Status

The Status checklist uses **account** membership (`getent group input` / `id -nG $USER`), not the current process’s session groups. If you already ran `usermod` and the daemon is active under `sg input`, a red ✗ should clear after the fix in v0.3.3+ — reinstall from a current build if it still lies.

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
./scripts/install-endeavouros.sh --verify
lsusb | grep -i 05ac
libinput list-devices | grep -i -A5 trackpad
magicpad-companion
systemctl --user status magicpad-gestures.service
```

In the app **Status** tab you should see a single Magic Trackpad entry (USB-C / Bluetooth).

## Uninstall

```bash
./scripts/install-endeavouros.sh --uninstall
# Removes the app from ~/.local and/or /usr/local (whichever is present),
# plus udev + gesture unit. Config left at: ~/.config/magicpad-companion
```

## Other distros

- **Fedora / openSUSE**: build from source or extract the DEB manually; adjust packages.  
- **Ubuntu 24.04+**: install the release `.deb` with `sudo apt install ./MagicPad.Companion_*_amd64.deb`.  
- Settings apply via `gsettings` / KDE config when available; otherwise companion JSON is the source of truth.

## Wayland note

The app sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` by default on Linux to avoid WebKitGTK crashes on some Budgie/labwc setups.
