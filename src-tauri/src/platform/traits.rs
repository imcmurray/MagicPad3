use crate::error::AppResult;
use crate::models::{
    BatteryInfo, DeviceInfo, DriverActionResult, DriverStatus, GestureMap, PlatformInfo,
    TrackpadSettings,
};

/// Cross-platform trackpad backend.
///
/// Implement once per OS; UI and commands only talk to this trait.
pub trait TrackpadBackend: Send + Sync {
    fn platform_info(&self) -> PlatformInfo;

    fn list_devices(&self) -> AppResult<Vec<DeviceInfo>>;

    fn battery(&self, device_id: &str) -> AppResult<Option<BatteryInfo>>;

    fn get_settings(&self) -> AppResult<TrackpadSettings>;

    fn set_settings(&self, settings: &TrackpadSettings) -> AppResult<()>;

    fn get_gestures(&self) -> AppResult<GestureMap>;

    fn set_gestures(&self, gestures: &GestureMap) -> AppResult<()>;

    fn driver_status(&self) -> AppResult<DriverStatus>;

    fn install_driver(&self) -> AppResult<DriverActionResult>;

    fn uninstall_driver(&self) -> AppResult<DriverActionResult>;

    /// Install udev rules / remapper profiles / helper packages (Linux), or no-op.
    fn install_system_helpers(&self) -> AppResult<DriverActionResult> {
        Ok(DriverActionResult {
            success: false,
            message: "No system helpers for this platform.".into(),
            log_lines: vec![],
            needs_reboot: false,
        })
    }
}
