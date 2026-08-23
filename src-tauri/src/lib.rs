mod commands;
mod config;
mod server;
mod storage;
mod templates;

use std::path::PathBuf;
use std::sync::Arc;

use server::{new_state, start_server};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let config_path = app
                .path()
                .app_data_dir()
                .map(|dir| dir.join("config.json"))
                .unwrap_or_else(|e| {
                    eprintln!("[overlays] could not resolve app_data_dir: {e}");
                    PathBuf::from("config.json")
                });

            let presets_path = app
                .path()
                .app_data_dir()
                .map(|dir| dir.join("presets.json"))
                .unwrap_or_else(|e| {
                    eprintln!("[overlays] could not resolve app_data_dir: {e}");
                    PathBuf::from("presets.json")
                });

            let app_config = config::load_config(&config_path);
            let overlay_dir = app_config.overlays_dir.unwrap_or_default();

            eprintln!("[overlays] config_path = {config_path:?}");
            eprintln!("[overlays] overlay_dir = {overlay_dir:?}");
            eprintln!("[overlays] presets_path = {presets_path:?}");

            let state = Arc::new(new_state(overlay_dir, presets_path, config_path));
            let server_state = state.clone();
            app.manage(state);

            tauri::async_runtime::spawn(async move {
                if let Err(e) = start_server(server_state).await {
                    eprintln!("[overlays] server error: {e}");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::send_overlay_update,
            commands::list_templates,
            commands::get_server_status,
            commands::save_preset,
            commands::list_presets,
            commands::delete_preset,
            commands::get_config,
            commands::set_overlays_dir,
            commands::set_language,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
