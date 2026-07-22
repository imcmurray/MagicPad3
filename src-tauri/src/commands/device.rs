use tauri::State;

use crate::error::AppResult;
use crate::models::{BatteryInfo, DeviceInfo};
use crate::state::AppState;

#[tauri::command]
pub fn list_devices(state: State<'_, AppState>) -> AppResult<Vec<DeviceInfo>> {
    let devices = state.backend.list_devices()?;
    state.push_log(
        "info",
        "device",
        format!("Enumerated {} device(s)", devices.len()),
    );
    Ok(devices)
}

#[tauri::command]
pub fn get_battery(state: State<'_, AppState>, device_id: String) -> AppResult<Option<BatteryInfo>> {
    state.backend.battery(&device_id)
}

#[tauri::command]
pub fn refresh_devices(state: State<'_, AppState>) -> AppResult<Vec<DeviceInfo>> {
    list_devices(state)
}
