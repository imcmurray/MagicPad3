//! MagicPad Companion — Tauri application library.

mod commands;
mod error;
mod models;
mod platform;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // WebKitGTK DMA-BUF can crash Wayland on some Arch/EndeavourOS + Budgie/labwc combos.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // SAFETY: before other threads start.
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }

    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();

    let app_state = AppState::new();
    app_state.push_log("info", "app", "MagicPad Companion starting");
    app_state.push_log(
        "info",
        "app",
        format!(
            "Platform: {} ({})",
            app_state.backend.platform_info().os_name,
            app_state.backend.platform_info().arch
        ),
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::system::get_platform,
            commands::system::get_snapshot,
            commands::system::app_version,
            commands::device::list_devices,
            commands::device::get_battery,
            commands::device::refresh_devices,
            commands::settings::get_settings,
            commands::settings::set_settings,
            commands::settings::reset_settings,
            commands::gestures::get_gestures,
            commands::gestures::set_gestures,
            commands::driver::get_driver_status,
            commands::driver::install_driver,
            commands::driver::uninstall_driver,
            commands::driver::install_system_helpers,
            commands::logs::get_logs,
            commands::logs::clear_logs,
            commands::logs::append_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MagicPad Companion");
}
