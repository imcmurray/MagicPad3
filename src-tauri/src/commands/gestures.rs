use tauri::State;

use crate::error::AppResult;
use crate::models::GestureMap;
use crate::state::AppState;

#[tauri::command]
pub fn get_gestures(state: State<'_, AppState>) -> AppResult<GestureMap> {
    state.backend.get_gestures()
}

#[tauri::command]
pub fn set_gestures(state: State<'_, AppState>, gestures: GestureMap) -> AppResult<()> {
    state.backend.set_gestures(&gestures)?;
    state.push_log("info", "gestures", "Gesture map saved");
    Ok(())
}
