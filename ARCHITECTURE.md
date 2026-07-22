# MagicPad Companion — Architecture

Cross-platform companion for **Apple Magic Trackpad 3 (A3120, USB-C)** and earlier Magic Trackpad 1/2 models.

## Goals

| Priority | Platform | Capability |
|----------|----------|------------|
| P0 | Windows 11 | Precision Touchpad gestures, USB-C + Bluetooth, battery, driver install |
| P1 | EndeavourOS / Arch | libinput + udev + input-remapper / optional daemon |
| P2 | Other Linux | Graceful degradation (detect + basic settings) |
| P2 | macOS | Status / battery display; system Settings for gestures |

## High-level diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     Svelte UI (Tauri 2)                      │
│  Status · Settings · Gestures · Driver · Logs · Help        │
└───────────────────────────┬─────────────────────────────────┘
                            │  invoke / events
┌───────────────────────────▼─────────────────────────────────┐
│                    commands/ (IPC surface)                    │
│  get_devices · get/set_settings · install_driver · logs …   │
└───────────────────────────┬─────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│              platform::TrackpadBackend (trait)                │
│         shared models: DeviceInfo, Settings, GestureMap       │
└───────┬─────────────────────┬─────────────────────┬─────────┘
        │                     │                     │
   ┌────▼────┐          ┌─────▼─────┐         ┌─────▼─────┐
   │ Windows │          │   Linux   │         │   macOS   │
   │ winapi  │          │ udev/sys  │         │ CoreHID   │
   │ SetupAPI│          │ libinput  │         │ (limited) │
   │ PTP reg │          │ remapper  │         │           │
   └────┬────┘          └─────┬─────┘         └───────────┘
        │                     │
   Precision driver      udev rules +
   (vitoplantamura /     systemd unit +
    imbushuo lineage)    gesture profiles
```

## Platform abstraction

All platform work goes through `src-tauri/src/platform/traits.rs`:

```rust
pub trait TrackpadBackend: Send + Sync {
    fn list_devices(&self) -> Result<Vec<DeviceInfo>>;
    fn battery(&self, id: &str) -> Result<Option<BatteryInfo>>;
    fn get_settings(&self) -> Result<TrackpadSettings>;
    fn set_settings(&self, s: &TrackpadSettings) -> Result<()>;
    fn get_gestures(&self) -> Result<GestureMap>;
    fn set_gestures(&self, g: &GestureMap) -> Result<()>;
    fn driver_status(&self) -> Result<DriverStatus>;
    fn install_driver(&self) -> Result<DriverActionResult>;
    fn uninstall_driver(&self) -> Result<DriverActionResult>;
}
```

`platform::mod.rs` selects the backend at compile time (`cfg(target_os = ...)`). Shared code never imports OS crates directly.

## Device identity

| Model | Interface | Typical IDs |
|-------|-----------|-------------|
| Magic Trackpad 3 (A3120) USB-C | USB HID | `VID_05AC` + `PID_0324` (and related PIDs as discovered) |
| Magic Trackpad 2 (Lightning) | USB HID | `VID_05AC` + `PID_0265` / `PID_030E` |
| Magic Trackpad 2/3 | Bluetooth | Apple HID over BT; paired via OS Bluetooth stack |
| Magic Trackpad 1 | Bluetooth | Older HID profile |

Detection strategy:

- **Windows**: SetupAPI / HID class + device instance paths; battery via PTP driver IOCTL or Bluetooth GATT where available.
- **Linux**: udev `ID_VENDOR_ID=05ac` + known product IDs; battery via `power_supply` sysfs or HID feature reports when exposed by the kernel/hid-magicmouse.

## Driver strategy

### Windows (primary)

**Recommended package:** [vitoplantamura/MagicTrackpad2ForWindows](https://github.com/vitoplantamura/MagicTrackpad2ForWindows)  
Fork of [imbushuo/mac-precision-touchpad](https://github.com/imbushuo/mac-precision-touchpad) with:

- Microsoft-signed packages (no test-signing hacks)
- USB-C Magic Trackpad support
- Battery level, haptic options, control panel

**Companion role:**

1. Detect whether AmtPtp / Apple USB Precision Touchpad devices are present.
2. One-click: download latest release zip (or use bundled cache), pick AMD64/ARM64 INF, elevate and `pnputil /add-driver ... /install`.
3. Uninstall via `pnputil /delete-driver` + Device Manager guidance.
4. Conflict detection: warn if Magic Utilities / Trackpad++ are installed.

We **do not** ship binary driver blobs in the main git tree by default (license + size + signing). Scripts under `scripts/` and the Driver panel download from the upstream release at install time, with checksum verification when published.

### Linux (EndeavourOS / Arch first)

1. **Kernel**: modern kernels handle Magic Trackpad 2 well via `hid-magicmouse` / multitouch. USB-C MT3 support improves over time — document kernel version expectations.
2. **udev rules** (`packaging/linux/99-magic-trackpad.rules`): tag device, set permissions for users in `input` group, optional hwdb.
3. **libinput**: natural scroll, tap, accel via GNOME/KDE settings *or* `libinput` quirks / local config written by the app.
4. **input-remapper**: optional advanced 3/4-finger maps as JSON profiles installed by the app.
5. **Optional systemd user service**: future custom gesture daemon (Rust) for compositor-agnostic actions when libinput + remapper are insufficient.

Graceful degradation: if only basic HID mouse/touchpad shows up, UI still shows connection + battery when available and links to troubleshooting.

### macOS

Native trackpad support is complete in System Settings. Companion is read-only status + battery + link-out; no driver install.

## Settings model (unified)

| Setting | Windows | Linux | macOS |
|---------|---------|-------|-------|
| Cursor speed / accel | PTP registry / driver control | libinput AccelSpeed | System Settings (read-only link) |
| Tap-to-click | PTP | libinput Tap | — |
| Natural scroll | PTP | libinput NaturalScrolling | — |
| Pinch sensitivity | PTP where exposed | compositor / remapper | — |
| Drag lock | PTP | libinput | — |
| Force / click threshold | driver-specific | limited | — |
| 3/4-finger gestures | Windows PTP / Settings | remapper / compositor | System Settings |

Unknown capabilities surface as “Not available on this platform” rather than silent no-ops.

## Security & permissions (Tauri 2)

- Minimal capability set: `core:default`, dialog, opener, process, log — no shell unless driver install needs an elevated helper.
- Windows driver install: spawn elevated helper (`pnputil`) via explicit user action only; no always-on admin.
- Linux: write udev rules / packages only after confirmation; prefer `pkexec` for root steps.
- No network except optional driver download from known GitHub release URLs.
- CSP locked down; no remote frontend.

## Frontend structure

Svelte 5 + Vite. Theme: CSS variables with `prefers-color-scheme` + manual override. Native feel:

- **Windows 11**: Segoe UI variable stack, subtle acrylic-like surfaces, rounded 8px cards.
- **Linux (Budgie / labwc)**: system font stack, slightly denser spacing, respects dark/light.

## Build & release

| Artifact | Tool |
|----------|------|
| Windows MSI/NSIS EXE | `cargo tauri build` on Windows (or cross with care) |
| Linux AppImage + DEB | `cargo tauri build` on Arch/Ubuntu runner |
| CI | GitHub Actions matrix: windows-latest, ubuntu-latest |

## Extensibility

- New OS: implement `TrackpadBackend`, add `cfg` arm in `platform/mod.rs`.
- New device PID: extend `KNOWN_DEVICES` constant list + udev match.
- Custom gesture actions: extend `GestureAction` enum + remapper profile generator.

## Priority order for implementation

1. Shared models + trait + mock-friendly stubs  
2. Windows device enumeration + driver status/install hooks (USB-C PID focus)  
3. Linux udev/sysfs detection + settings apply path  
4. Gesture mapping UI + remapper export  
5. Packaging & release automation  
6. macOS polish  
