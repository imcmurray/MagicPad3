//! Linux backend (EndeavourOS / Arch first): sysfs + udev discovery,
//! libinput-oriented settings, input-remapper profiles, helper install.

mod device;
mod gestures;
mod helpers;
mod settings;

use crate::error::AppResult;
use crate::models::*;
use crate::platform::TrackpadBackend;

pub struct LinuxBackend {
    config_dir: std::path::PathBuf,
}

impl LinuxBackend {
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("magicpad-companion");
        let _ = std::fs::create_dir_all(&config_dir);
        Self { config_dir }
    }
}

impl TrackpadBackend for LinuxBackend {
    fn platform_info(&self) -> PlatformInfo {
        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .ok()
            .or_else(|| std::env::var("DESKTOP_SESSION").ok());
        let os_name = read_os_pretty_name().unwrap_or_else(|| "Linux".into());
        PlatformInfo {
            kind: PlatformKind::Linux,
            os_name,
            os_version: read_os_version(),
            arch: std::env::consts::ARCH.into(),
            desktop,
            capabilities: vec![
                "device_detect".into(),
                "battery".into(),
                "libinput_settings".into(),
                "udev_helpers".into(),
                "input_remapper".into(),
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
        settings::load(&self.config_dir)
    }

    fn set_settings(&self, s: &TrackpadSettings) -> AppResult<()> {
        settings::save(&self.config_dir, s)?;
        settings::apply_libinput(s)
    }

    fn get_gestures(&self) -> AppResult<GestureMap> {
        gestures::load(&self.config_dir)
    }

    fn set_gestures(&self, g: &GestureMap) -> AppResult<()> {
        gestures::save(&self.config_dir, g)?;
        gestures::export_input_remapper(g)
    }

    fn driver_status(&self) -> AppResult<DriverStatus> {
        helpers::status()
    }

    fn install_driver(&self) -> AppResult<DriverActionResult> {
        // On Linux "driver" == kernel module + udev/helpers
        self.install_system_helpers()
    }

    fn uninstall_driver(&self) -> AppResult<DriverActionResult> {
        helpers::uninstall()
    }

    fn install_system_helpers(&self) -> AppResult<DriverActionResult> {
        helpers::install()
    }
}

fn read_os_pretty_name() -> Option<String> {
    let text = std::fs::read_to_string("/etc/os-release").ok()?;
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("PRETTY_NAME=") {
            return Some(v.trim_matches('"').to_string());
        }
    }
    None
}

fn read_os_version() -> Option<String> {
    let text = std::fs::read_to_string("/etc/os-release").ok()?;
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("VERSION_ID=") {
            return Some(v.trim_matches('"').to_string());
        }
    }
    None
}
