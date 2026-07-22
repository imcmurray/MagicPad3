use tauri::State;

use crate::error::AppResult;
use crate::models::{AppSnapshot, PlatformInfo};
use crate::state::AppState;

#[tauri::command]
pub fn get_platform(state: State<'_, AppState>) -> PlatformInfo {
    state.backend.platform_info()
}

#[tauri::command]
pub fn get_snapshot(state: State<'_, AppState>) -> AppResult<AppSnapshot> {
    let platform = state.backend.platform_info();
    let devices = state.backend.list_devices().unwrap_or_default();
    let settings = state
        .backend
        .get_settings()
        .unwrap_or_else(|_| state.settings_cache.lock().clone());
    let gestures = state.backend.get_gestures().unwrap_or_default();
    let driver = state.backend.driver_status()?;
    Ok(AppSnapshot {
        platform,
        devices,
        settings,
        gestures,
        driver,
    })
}

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
