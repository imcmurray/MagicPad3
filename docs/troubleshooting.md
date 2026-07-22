# Troubleshooting

See also the in-app **Help** tab.

## Windows home testing

| Symptom | Fix |
|---------|-----|
| SmartScreen blocks app | More info → Run anyway (test builds are unsigned) |
| Driver install fails | Put INF under `%LOCALAPPDATA%\MagicPadCompanion\drivers\AMD64\`; run app as Admin once |
| No gestures after install | Replug USB-C / re-pair BT; remove Magic Utilities / Trackpad++ |
| Need a build | [Releases](https://github.com/imcmurray/MagicPad3/releases) or Actions artifact `windows-installers` |

Full guide: [windows-install.md](./windows-install.md)

## USB issues

| Symptom | Fix |
|---------|-----|
| No device in Status | Data-capable cable; another port; `lsusb` / Device Manager |
| Enumerates then drops | Power management / autosuspend; try direct port |
| Wrong driver (Windows) | Uninstall conflicting stacks; install PTP package |

## Gesture lag

- Prefer USB-C for diagnosis.  
- Close high-CPU overlays.  
- Lower acceleration / pinch sensitivity.  
- Linux: check for double-processing (libinput + remapper both mapping).  

## Reconnection

- Windows Bluetooth: remove device, pair again while trackpad is charging via USB-C.  
- Linux: `bluetoothctl` reconnect; ensure firmware/kernel not blacklisting HID.  

## Battery missing

Expected until the stack exposes `power_supply` (Linux) or PTP battery IOCTL (Windows). Wired charge may show charging without percent.

## Wayland / WebKit crash (Linux)

The app sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` by default on Linux (Arch + Budgie/labwc mitigation). Override by exporting your preferred value before launch.

## Logs

Use the **Logs** tab and attach entries when filing issues.
