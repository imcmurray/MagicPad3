# Windows Precision driver packaging notes

MagicPad Companion **does not vend** proprietary or signed `.sys` blobs in git by default.

## Recommended upstream

**[vitoplantamura/MagicTrackpad2ForWindows](https://github.com/vitoplantamura/MagicTrackpad2ForWindows)**

- Microsoft-signed INF packages  
- USB-C Magic Trackpad support  
- Bluetooth + battery + haptics options  
- Based on [imbushuo/mac-precision-touchpad](https://github.com/imbushuo/mac-precision-touchpad)

## Install layout for one-click companion install

Place the extracted release under:

```
%LOCALAPPDATA%\MagicPadCompanion\drivers\
  AMD64\*.inf
  ARM64\*.inf
```

Or next to the installed EXE:

```
<app>\drivers\AMD64\...
```

Then use **Driver → Install driver** in the app (Administrator may be required for `pnputil`).

## Hardware IDs (USB)

| Device | Typical HWID |
|--------|----------------|
| Magic Trackpad 3 USB-C (A3120) | `USB\VID_05AC&PID_0324` |
| Magic Trackpad 2 | `USB\VID_05AC&PID_0265` / `PID_030E` |

Bluetooth devices appear under BTHENUM / HID with Apple descriptors; the PTP filter driver binds after pairing.

## Conflicts

Uninstall before installing:

- Magic Utilities  
- Trackpad++  
- Older unsigned imbushuo test-mode installs (clean DriverStore)

## Manual install

1. Download release zip from upstream Releases.  
2. Right-click architecture INF → Install.  
3. Replug USB-C or re-pair Bluetooth.  

## License

Upstream driver is **GPL-2.0**. If you redistribute binaries, comply with GPL (source offer, etc.).
