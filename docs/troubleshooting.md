# Troubleshooting

See also the in-app **Help** tab.

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
