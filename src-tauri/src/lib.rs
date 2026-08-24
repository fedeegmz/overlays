mod application;
mod domain;
mod infrastructure;

use std::path::PathBuf;
use std::sync::Arc;

use application::config_service::ConfigService;
use application::ports::ConfigRepository;
use application::preset_service::PresetService;
use application::template_catalog::{OverlaysDirHandle, TemplateCatalog};
use infrastructure::fs_template_source::FsTemplateSource;
use infrastructure::http::{start_server, state::HttpState};
use infrastructure::json_store::{JsonConfigRepository, JsonPresetRepository};
use infrastructure::overlay_bus::BroadcastOverlayBus;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = match app.path().app_data_dir() {
                Ok(dir) => dir,
                Err(e) => {
                    eprintln!("[overlays] could not resolve app_data_dir: {e}");
                    PathBuf::from(".")
                }
            };
            let config_path = data_dir.join("config.json");
            let presets_path = data_dir.join("presets.json");

            eprintln!("[overlays] config_path = {config_path:?}");
            eprintln!("[overlays] presets_path = {presets_path:?}");

            let config_repo = Arc::new(JsonConfigRepository::new(config_path));
            let preset_repo = Arc::new(JsonPresetRepository::new(presets_path));

            let app_config = config_repo.load();
            let overlays_dir =
                OverlaysDirHandle::new(app_config.overlays_dir.clone().unwrap_or_default());

            eprintln!("[overlays] overlays_dir = {:?}", overlays_dir.get());

            let bus = Arc::new(BroadcastOverlayBus::new());
            let catalog = Arc::new(TemplateCatalog::new(
                Arc::new(FsTemplateSource),
                overlays_dir.clone(),
            ));
            let presets = Arc::new(PresetService::new(preset_repo));
            let config_service = Arc::new(ConfigService::new(config_repo, overlays_dir.clone()));
            let http_state = Arc::new(HttpState::new(
                bus,
                catalog.clone(),
                overlays_dir.clone(),
            ));

            app.manage(http_state.clone());
            app.manage(catalog);
            app.manage(presets);
            app.manage(config_service);

            tauri::async_runtime::spawn(async move {
                if let Err(e) = start_server(http_state).await {
                    eprintln!("[overlays] server error: {e}");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            infrastructure::tauri::commands::send_overlay_update,
            infrastructure::tauri::commands::list_templates,
            infrastructure::tauri::commands::get_server_status,
            infrastructure::tauri::commands::save_preset,
            infrastructure::tauri::commands::list_presets,
            infrastructure::tauri::commands::delete_preset,
            infrastructure::tauri::commands::get_config,
            infrastructure::tauri::commands::set_overlays_dir,
            infrastructure::tauri::commands::set_language,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
