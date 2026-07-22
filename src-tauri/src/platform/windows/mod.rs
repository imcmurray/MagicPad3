//! Windows 11 backend: SetupAPI enumeration, Precision driver install hooks,
//! PTP-oriented settings (registry / future IOCTL).
//!
//! Priority: Magic Trackpad 3 USB-C (VID_05AC&PID_0324) + Bluetooth PTP.

mod device;
mod driver;
mod settings;

use crate::error::AppResult;
use crate::models::*;
use crate::platform::TrackpadBackend;

pub struct WindowsBackend {
    settings_path: std::path::PathBuf,
}

impl WindowsBackend {
    pub fn new() -> Self {
        let settings_path = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("MagicPadCompanion")
            .join("settings.json");
        Self { settings_path }
    }
}

impl TrackpadBackend for WindowsBackend {
    fn platform_info(&self) -> PlatformInfo {
        PlatformInfo {
            kind: PlatformKind::Windows,
            os_name: "Windows".into(),
            os_version: Some(std::env::var("OS").unwrap_or_else(|_| "Windows_NT".into())),
            arch: std::env::consts::ARCH.into(),
            desktop: None,
            capabilities: vec![
                "device_detect".into(),
                "battery".into(),
                "precision_driver".into(),
                "settings".into(),
                "gestures_system".into(),
            ],
        }
    }

    fn list_devices(&self) -> AppResult<Vec<DeviceInfo>> {
        device::enumerate_devices()
    }

    fn battery(&self, device_id: &str) -> AppResult<Option<BatteryInfo>> {
        device::read_battery(device_id)
    }

    fn get_settings(&self) -> AppResult<TrackpadSettings> {
        settings::load(&self.settings_path)
    }

    fn set_settings(&self, s: &TrackpadSettings) -> AppResult<()> {
        settings::save(&self.settings_path, s)?;
        settings::apply_system(s)
    }

    fn get_gestures(&self) -> AppResult<GestureMap> {
        let mut map = GestureMap::default();
        map.backend = "windows_precision".into();
        // On Windows, multi-finger gestures are largely owned by the OS /
        // PTP stack; we expose bindings as documentation + future registry hooks.
        for b in &mut map.bindings {
            b.available = true;
        }
        Ok(map)
    }

    fn set_gestures(&self, gestures: &GestureMap) -> AppResult<()> {
        settings::save_gestures(&self.settings_path, gestures)?;
        // Full OS gesture remapping requires additional hooks; persist for now.
        Ok(())
    }

    fn driver_status(&self) -> AppResult<DriverStatus> {
        driver::status()
    }

    fn install_driver(&self) -> AppResult<DriverActionResult> {
        driver::install()
    }

    fn uninstall_driver(&self) -> AppResult<DriverActionResult> {
        driver::uninstall()
    }
}
