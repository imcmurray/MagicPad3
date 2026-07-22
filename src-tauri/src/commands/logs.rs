use tauri::State;

use crate::models::LogEntry;
use crate::state::AppState;

#[tauri::command]
pub fn get_logs(state: State<'_, AppState>) -> Vec<LogEntry> {
    state.logs_snapshot()
}

#[tauri::command]
pub fn clear_logs(state: State<'_, AppState>) {
    state.clear_logs();
    state.push_log("info", "logs", "Log buffer cleared");
}

#[tauri::command]
pub fn append_log(state: State<'_, AppState>, level: String, source: String, message: String) {
    state.push_log(&level, &source, message);
}
