//! Shared domain models for MagicPad Companion.
//! Platform backends map OS-specific state into these types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Apple USB vendor ID.
pub const APPLE_VID: u16 = 0x05AC;

/// Known Magic Trackpad product IDs (USB / reported HID).
/// Extend as new hardware IDs are confirmed in the field.
pub const KNOWN_PIDS: &[(u16, &str)] = &[
    (0x0324, "Magic Trackpad 3 (USB-C)"),
    (0x0325, "Magic Trackpad 3 (USB-C, variant)"),
    (0x0265, "Magic Trackpad 2"),
    (0x030E, "Magic Trackpad 2 (USB)"),
    (0x024F, "Magic Trackpad 1 / early"),
    (0x0229, "Magic Trackpad (legacy)"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionType {
    UsbC,
    Usb,
    Bluetooth,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformKind {
    Windows,
    Linux,
    Macos,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryInfo {
    pub percent: Option<u8>,
    pub charging: Option<bool>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub model: String,
    pub vid: Option<u16>,
    pub pid: Option<u16>,
    pub connection: ConnectionType,
    pub connected: bool,
    pub battery: Option<BatteryInfo>,
    pub driver_bound: Option<bool>,
    pub path: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackpadSettings {
    /// 0.0 – 1.0 normalized pointer speed
    pub pointer_speed: f32,
    /// -1.0 – 1.0 acceleration (platform-mapped)
    pub acceleration: f32,
    pub tap_to_click: bool,
    pub natural_scroll: bool,
    /// 0.0 – 1.0
    pub pinch_sensitivity: f32,
    pub drag_lock: bool,
    /// 0.0 – 1.0 force / click threshold when available
    pub force_threshold: f32,
    pub right_click_two_finger: bool,
    pub horizontal_scroll: bool,
}

impl Default for TrackpadSettings {
    fn default() -> Self {
        Self {
            pointer_speed: 0.5,
            acceleration: 0.0,
            tap_to_click: true,
            natural_scroll: true,
            pinch_sensitivity: 0.5,
            drag_lock: false,
            force_threshold: 0.5,
            right_click_two_finger: true,
            horizontal_scroll: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GestureTrigger {
    ThreeFingerSwipeLeft,
    ThreeFingerSwipeRight,
    ThreeFingerSwipeUp,
    ThreeFingerSwipeDown,
    ThreeFingerTap,
    FourFingerSwipeLeft,
    FourFingerSwipeRight,
    FourFingerSwipeUp,
    FourFingerSwipeDown,
    FourFingerTap,
    PinchIn,
    PinchOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GestureAction {
    None,
    MissionControl,
    AppExpose,
    DesktopShow,
    NextDesktop,
    PrevDesktop,
    BrowserBack,
    BrowserForward,
    SwitchApp,
    NotificationCenter,
    VolumeUp,
    VolumeDown,
    MediaPlayPause,
    /// Ctrl+=  — browser / document zoom in
    ZoomIn,
    /// Ctrl+-  — browser / document zoom out
    ZoomOut,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GestureBinding {
    pub trigger: GestureTrigger,
    pub action: GestureAction,
    /// Free-form payload for `Custom` (e.g. shell command key or remapper id)
    pub custom: Option<String>,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GestureMap {
    pub bindings: Vec<GestureBinding>,
    pub backend: String,
}

impl Default for GestureMap {
    fn default() -> Self {
        use GestureAction as A;
        use GestureTrigger as T;
        let defaults = [
            (T::ThreeFingerSwipeLeft, A::PrevDesktop),
            (T::ThreeFingerSwipeRight, A::NextDesktop),
            (T::ThreeFingerSwipeUp, A::MissionControl),
            (T::ThreeFingerSwipeDown, A::AppExpose),
            (T::ThreeFingerTap, A::None),
            (T::FourFingerSwipeLeft, A::BrowserBack),
            (T::FourFingerSwipeRight, A::BrowserForward),
            (T::FourFingerSwipeUp, A::DesktopShow),
            (T::FourFingerSwipeDown, A::NotificationCenter),
            (T::FourFingerTap, A::None),
            (T::PinchIn, A::ZoomOut),
            (T::PinchOut, A::ZoomIn),
        ];
        Self {
            bindings: defaults
                .into_iter()
                .map(|(trigger, action)| GestureBinding {
                    trigger,
                    action,
                    custom: Option::None,
                    available: true,
                })
                .collect(),
            backend: "default".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriverState {
    Unknown,
    NotInstalled,
    Installed,
    Outdated,
    Conflict,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverStatus {
    pub state: DriverState,
    pub name: String,
    pub version: Option<String>,
    pub detail: String,
    pub can_install: bool,
    pub can_uninstall: bool,
    pub recommended_source: Option<String>,
    pub conflicts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverActionResult {
    pub success: bool,
    pub message: String,
    pub log_lines: Vec<String>,
    pub needs_reboot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformInfo {
    pub kind: PlatformKind,
    pub os_name: String,
    pub os_version: Option<String>,
    pub arch: String,
    pub desktop: Option<String>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub platform: PlatformInfo,
    pub devices: Vec<DeviceInfo>,
    pub settings: TrackpadSettings,
    pub gestures: GestureMap,
    pub driver: DriverStatus,
}

/// Linux gesture daemon readiness (Windows always reports unavailable).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GestureDaemonStatus {
    pub available: bool,
    pub running: bool,
    pub libinput_ok: bool,
    pub wtype_ok: bool,
    pub input_group: bool,
    pub unit_installed: bool,
    pub message: String,
}

pub fn model_name_for_pid(pid: u16) -> String {
    KNOWN_PIDS
        .iter()
        .find(|(p, _)| *p == pid)
        .map(|(_, n)| (*n).to_string())
        .unwrap_or_else(|| format!("Apple HID device (PID {pid:04X})"))
}

pub fn is_known_trackpad_pid(pid: u16) -> bool {
    KNOWN_PIDS.iter().any(|(p, _)| *p == pid)
}
