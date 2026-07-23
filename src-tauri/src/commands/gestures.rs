use tauri::State;

use crate::error::AppResult;
use crate::models::{GestureDaemonStatus, GestureMap};
use crate::state::AppState;

#[tauri::command]
pub fn get_gestures(state: State<'_, AppState>) -> AppResult<GestureMap> {
    state.backend.get_gestures()
}

#[tauri::command]
pub fn set_gestures(state: State<'_, AppState>, gestures: GestureMap) -> AppResult<()> {
    match state.backend.set_gestures(&gestures) {
        Ok(()) => {
            state.push_log("info", "gestures", "Gesture map saved; daemon started");
            Ok(())
        }
        Err(e) => {
            // Still log — partial success (saved file) is common on first setup
            state.push_log("warn", "gestures", e.to_string());
            Err(e)
        }
    }
}

#[tauri::command]
pub fn get_gesture_daemon_status() -> GestureDaemonStatus {
    #[cfg(target_os = "linux")]
    {
        crate::gesture_daemon::daemon_status()
    }
    #[cfg(not(target_os = "linux"))]
    {
        GestureDaemonStatus {
            available: false,
            running: false,
            libinput_ok: false,
            wtype_ok: false,
            input_group: false,
            unit_installed: false,
            message: "Gesture daemon is Linux-only (Windows uses Precision Touchpad)".into(),
        }
    }
}

#[tauri::command]
pub fn start_gesture_daemon(state: State<'_, AppState>) -> AppResult<String> {
    #[cfg(target_os = "linux")]
    {
        let msg =
            crate::gesture_daemon::ensure_daemon_running().map_err(crate::error::AppError::msg)?;
        state.push_log("info", "gestures", &msg);
        Ok(msg)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = state;
        Err(crate::error::AppError::Unsupported(
            "Gesture daemon is Linux-only".into(),
        ))
    }
}
