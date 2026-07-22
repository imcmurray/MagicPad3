//! Persist companion settings and best-effort apply to Windows PTP stack.

use std::path::Path;

use crate::error::{AppError, AppResult};
use crate::models::{GestureMap, TrackpadSettings};

pub fn load(path: &Path) -> AppResult<TrackpadSettings> {
    if !path.exists() {
        return Ok(TrackpadSettings::default());
    }
    let text = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

pub fn save(path: &Path, settings: &TrackpadSettings) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(settings)?;
    std::fs::write(path, text)?;
    Ok(())
}

pub fn save_gestures(settings_path: &Path, gestures: &GestureMap) -> AppResult<()> {
    let path = settings_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("gestures.json");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(gestures)?)?;
    Ok(())
}

/// Apply pointer/touchpad settings via registry where documented for PTP.
///
/// Full coverage depends on the installed Precision driver. We write the
/// common `HKCU\Software\Microsoft\Windows\CurrentVersion\PrecisionTouchPad`
/// values when available and keep companion JSON as the source of truth.
pub fn apply_system(settings: &TrackpadSettings) -> AppResult<()> {
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu.create_subkey(
            r"Software\Microsoft\Windows\CurrentVersion\PrecisionTouchPad",
        )?;

        // AAPThreshold / tap — values are driver-dependent; store companion-normalized copies.
        let _ = key.set_value("MagicPad_TapToClick", &(settings.tap_to_click as u32));
        let _ = key.set_value(
            "MagicPad_NaturalScroll",
            &(settings.natural_scroll as u32),
        );
        let _ = key.set_value(
            "MagicPad_PointerSpeed",
            &((settings.pointer_speed * 100.0) as u32),
        );
        let _ = key.set_value(
            "MagicPad_ForceThreshold",
            &((settings.force_threshold * 100.0) as u32),
        );

        // Classic mouse speed (1-20)
        let speed = (settings.pointer_speed * 19.0).round() as u32 + 1;
        if let Ok((mouse, _)) = hkcu.create_subkey(r"Control Panel\Mouse") {
            let _ = mouse.set_value("MouseSensitivity", &speed.to_string());
        }

        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = settings;
        Err(AppError::Unsupported("Windows settings only".into()))
    }
}
