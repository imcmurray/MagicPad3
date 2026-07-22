//! macOS backend — graceful degradation.
//! Native gestures and pairing live in System Settings; we expose status only.

use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::platform::TrackpadBackend;

pub struct MacosBackend {
    config_dir: std::path::PathBuf,
}

impl MacosBackend {
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("MagicPadCompanion");
        let _ = std::fs::create_dir_all(&config_dir);
        Self { config_dir }
    }
}

impl TrackpadBackend for MacosBackend {
    fn platform_info(&self) -> PlatformInfo {
        PlatformInfo {
            kind: PlatformKind::Macos,
            os_name: "macOS".into(),
            os_version: None,
            arch: std::env::consts::ARCH.into(),
            desktop: Some("Aqua".into()),
            capabilities: vec!["status_only".into(), "battery_best_effort".into()],
        }
    }

    fn list_devices(&self) -> AppResult<Vec<DeviceInfo>> {
        // Future: IOKit HID matching for Apple Multitouch Device.
        // For v0.1 return empty + note in driver status.
        Ok(vec![])
    }

    fn battery(&self, _: &str) -> AppResult<Option<BatteryInfo>> {
        Ok(None)
    }

    fn get_settings(&self) -> AppResult<TrackpadSettings> {
        let path = self.config_dir.join("settings.json");
        if path.exists() {
            let t = std::fs::read_to_string(path)?;
            return Ok(serde_json::from_str(&t)?);
        }
        Ok(TrackpadSettings::default())
    }

    fn set_settings(&self, s: &TrackpadSettings) -> AppResult<()> {
        std::fs::write(
            self.config_dir.join("settings.json"),
            serde_json::to_string_pretty(s)?,
        )?;
        Err(AppError::Unsupported(
            "On macOS, change trackpad settings in System Settings → Trackpad. Values were saved locally only.".into(),
        ))
    }

    fn get_gestures(&self) -> AppResult<GestureMap> {
        let mut map = GestureMap::default();
        map.backend = "macos_system".into();
        for b in &mut map.bindings {
            b.available = false;
        }
        Ok(map)
    }

    fn set_gestures(&self, _: &GestureMap) -> AppResult<()> {
        Err(AppError::Unsupported(
            "Configure gestures in System Settings → Trackpad.".into(),
        ))
    }

    fn driver_status(&self) -> AppResult<DriverStatus> {
        Ok(DriverStatus {
            state: DriverState::NotApplicable,
            name: "Apple native".into(),
            version: None,
            detail: "macOS includes first-party Magic Trackpad support. No third-party driver is required.".into(),
            can_install: false,
            can_uninstall: false,
            recommended_source: None,
            conflicts: vec![],
        })
    }

    fn install_driver(&self) -> AppResult<DriverActionResult> {
        Err(AppError::Unsupported("Not applicable on macOS".into()))
    }

    fn uninstall_driver(&self) -> AppResult<DriverActionResult> {
        Err(AppError::Unsupported("Not applicable on macOS".into()))
    }
}
