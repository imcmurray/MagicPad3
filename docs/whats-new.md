# What's New

Release notes for **MagicPad Companion**.  
GitHub releases: [imcmurray/MagicPad3/releases](https://github.com/imcmurray/MagicPad3/releases)

---

## v0.3.3

**Tag:** [v0.3.3](https://github.com/imcmurray/MagicPad3/releases/tag/v0.3.3) · **Compare:** [v0.3.2…v0.3.3](https://github.com/imcmurray/MagicPad3/compare/v0.3.2...v0.3.3)

### Highlights

- **3-finger tap → Budgie Screenshot** by default on Linux (`org.buddiesofbudgie.BudgieScreenshot -i`)
- **4-finger tap** left unbound so you can set a **Custom** command (e.g. `flameshot gui`)
- **Custom command field** in the Gestures UI for any shell command per binding
- Multi-finger taps use libinput **hold** gestures and require **3+ fingers** (a normal 1-finger click no longer opens Screenshot)

### Linux gesture stack (0.3.x series, polished in 0.3.3)

- User daemon: `libinput debug-events` → actions via `wtype` / commands
- `magicpad-gestures.service` runs under **`sg input`** so `/dev/input` works without a full re-login after `usermod -aG input`
- Status checklist uses **account** membership (`getent` / `id user`), not flaky session groups
- 4-finger swipe **browser back/forward** uses a Wayland-friendly key/mouse cascade (not only Alt+Left)
- Pinch in/out → zoom (`Ctrl+-` / `Ctrl+=`)
- EndeavourOS/Arch installer enables the daemon, seeds `gestures.json`, and installs to **one** prefix (`~/.local/bin` if it exists, else `/usr/local/bin`)

### Install / repair

```bash
git clone https://github.com/imcmurray/MagicPad3.git
cd MagicPad3
./scripts/install-endeavouros.sh --local   # or omit --local for latest GitHub .deb
./scripts/install-endeavouros.sh --verify
```

Full guide: [linux-install.md](./linux-install.md)

---

## v0.3.2

**Tag:** [v0.3.2](https://github.com/imcmurray/MagicPad3/releases/tag/v0.3.2)

- 3/4-finger tap can open Budgie Screenshot
- Gesture daemon packaging refinements

---

## v0.3.1

**Tag:** [v0.3.1](https://github.com/imcmurray/MagicPad3/releases/tag/v0.3.1)

- Pinch in/out mapped to browser/app zoom (`Ctrl+-` / `Ctrl+=`)

---

## v0.3.0

**Tag:** [v0.3.0](https://github.com/imcmurray/MagicPad3/releases/tag/v0.3.0)

- First **Linux multi-finger gesture daemon** for Budgie + labwc (Windows already uses Precision gestures)
- 3/4-finger swipes → workspace / window / desktop actions via `wtype`
- User systemd unit + seed config under `~/.config/magicpad-companion/`

---

## v0.2.2

**Tag:** [v0.2.2](https://github.com/imcmurray/MagicPad3/releases/tag/v0.2.2)

- App icon matches the UI logo
- Packaging / release polish

---

## v0.2.1

**Tag:** [v0.2.1](https://github.com/imcmurray/MagicPad3/releases/tag/v0.2.1)

- Multi-device **Status** collapse by identity (VID/PID + serial) so one trackpad does not appear as several rows

---

## v0.2.0

**Tag:** [v0.2.0](https://github.com/imcmurray/MagicPad3/releases/tag/v0.2.0)

- Multi-device Status handling for Windows multi-interface HID enumeration

---

## v0.1.0

**Tag:** [v0.1.0](https://github.com/imcmurray/MagicPad3/releases/tag/v0.1.0)

- Initial public release: Tauri 2 + Svelte companion for Magic Trackpad on Windows 11 and Linux
- Device scan, settings, gesture map UI, driver/udev hooks, theme, logs, troubleshooting

---

## Links

| Resource | URL |
|----------|-----|
| Latest release | https://github.com/imcmurray/MagicPad3/releases/latest |
| Windows install | [windows-install.md](./windows-install.md) |
| Linux install | [linux-install.md](./linux-install.md) |
| Troubleshooting | [troubleshooting.md](./troubleshooting.md) |
| Architecture | [../ARCHITECTURE.md](../ARCHITECTURE.md) |
