use tauri::State;

use crate::error::AppResult;
use crate::models::{DriverActionResult, DriverStatus};
use crate::state::AppState;

#[tauri::command]
pub fn get_driver_status(state: State<'_, AppState>) -> AppResult<DriverStatus> {
    state.backend.driver_status()
}

#[tauri::command]
pub fn install_driver(state: State<'_, AppState>) -> AppResult<DriverActionResult> {
    state.push_log("info", "driver", "Install requested");
    let result = state.backend.install_driver()?;
    for line in &result.log_lines {
        state.push_log(if result.success { "info" } else { "warn" }, "driver", line);
    }
    Ok(result)
}

#[tauri::command]
pub fn uninstall_driver(state: State<'_, AppState>) -> AppResult<DriverActionResult> {
    state.push_log("info", "driver", "Uninstall requested");
    let result = state.backend.uninstall_driver()?;
    for line in &result.log_lines {
        state.push_log(if result.success { "info" } else { "warn" }, "driver", line);
    }
    Ok(result)
}

#[tauri::command]
pub fn install_system_helpers(state: State<'_, AppState>) -> AppResult<DriverActionResult> {
    state.push_log("info", "driver", "System helpers install requested");
    let result = state.backend.install_system_helpers()?;
    for line in &result.log_lines {
        state.push_log(if result.success { "info" } else { "warn" }, "driver", line);
    }
    Ok(result)
}
