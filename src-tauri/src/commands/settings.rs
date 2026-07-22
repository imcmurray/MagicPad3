use tauri::State;

use crate::error::AppResult;
use crate::models::TrackpadSettings;
use crate::state::AppState;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppResult<TrackpadSettings> {
    let s = state.backend.get_settings()?;
    *state.settings_cache.lock() = s.clone();
    Ok(s)
}

#[tauri::command]
pub fn set_settings(state: State<'_, AppState>, settings: TrackpadSettings) -> AppResult<()> {
    state.backend.set_settings(&settings)?;
    *state.settings_cache.lock() = settings;
    state.push_log("info", "settings", "Settings applied");
    Ok(())
}

#[tauri::command]
pub fn reset_settings(state: State<'_, AppState>) -> AppResult<TrackpadSettings> {
    let defaults = TrackpadSettings::default();
    state.backend.set_settings(&defaults)?;
    *state.settings_cache.lock() = defaults.clone();
    state.push_log("info", "settings", "Settings reset to defaults");
    Ok(defaults)
}
