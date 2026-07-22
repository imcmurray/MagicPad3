mod traits;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
mod stub;

pub use traits::TrackpadBackend;

/// Construct the compile-time backend for the host OS.
pub fn create_backend() -> Box<dyn TrackpadBackend> {
    #[cfg(target_os = "windows")]
    {
        return Box::new(windows::WindowsBackend::new());
    }
    #[cfg(target_os = "linux")]
    {
        return Box::new(linux::LinuxBackend::new());
    }
    #[cfg(target_os = "macos")]
    {
        return Box::new(macos::MacosBackend::new());
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        return Box::new(stub::StubBackend::new());
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
mod stub {
    use super::TrackpadBackend;
    use crate::error::{AppError, AppResult};
    use crate::models::*;

    pub struct StubBackend;

    impl StubBackend {
        pub fn new() -> Self {
            Self
        }
    }

    impl TrackpadBackend for StubBackend {
        fn platform_info(&self) -> PlatformInfo {
            PlatformInfo {
                kind: PlatformKind::Other,
                os_name: std::env::consts::OS.into(),
                os_version: None,
                arch: std::env::consts::ARCH.into(),
                desktop: None,
                capabilities: vec!["status_only".into()],
            }
        }

        fn list_devices(&self) -> AppResult<Vec<DeviceInfo>> {
            Ok(vec![])
        }

        fn battery(&self, _: &str) -> AppResult<Option<BatteryInfo>> {
            Ok(None)
        }

        fn get_settings(&self) -> AppResult<TrackpadSettings> {
            Ok(TrackpadSettings::default())
        }

        fn set_settings(&self, _: &TrackpadSettings) -> AppResult<()> {
            Err(AppError::Unsupported(
                "settings not available on this OS".into(),
            ))
        }

        fn get_gestures(&self) -> AppResult<GestureMap> {
            Ok(GestureMap::default())
        }

        fn set_gestures(&self, _: &GestureMap) -> AppResult<()> {
            Err(AppError::Unsupported(
                "gestures not available on this OS".into(),
            ))
        }

        fn driver_status(&self) -> AppResult<DriverStatus> {
            Ok(DriverStatus {
                state: DriverState::NotApplicable,
                name: "N/A".into(),
                version: None,
                detail: "No driver management on this platform.".into(),
                can_install: false,
                can_uninstall: false,
                recommended_source: None,
                conflicts: vec![],
            })
        }

        fn install_driver(&self) -> AppResult<DriverActionResult> {
            Err(AppError::Unsupported("driver install N/A".into()))
        }

        fn uninstall_driver(&self) -> AppResult<DriverActionResult> {
            Err(AppError::Unsupported("driver uninstall N/A".into()))
        }
    }
}
