use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use crate::application::config_service::ConfigService;
use crate::application::ports::OverlayBus;
use crate::application::preset_service::PresetService;
use crate::application::template_catalog::TemplateCatalog;
use crate::domain::config::AppConfig;
use crate::domain::overlay::{OverlayAction, OverlayPayload};
use crate::domain::preset::Preset;
use crate::domain::template::Manifest;
use crate::infrastructure::error::CommandError;
use crate::infrastructure::http::state::HttpState;

#[derive(Debug, Clone, Serialize)]
pub struct ServerStatus {
    pub running: bool,
    pub port: u16,
}

#[tauri::command]
pub fn send_overlay_update(
    http: State<'_, Arc<HttpState>>,
    instance_id: String,
    template: String,
    action: OverlayAction,
    fields: HashMap<String, String>,
) -> Result<(), CommandError> {
    http.bus.publish(&OverlayPayload {
        instance_id,
        template,
        action,
        fields,
    });
    Ok(())
}

#[tauri::command]
pub fn list_templates(
    templates: State<'_, Arc<TemplateCatalog>>,
) -> Result<Manifest, CommandError> {
    templates.manifest().map_err(CommandError::from)
}

#[tauri::command]
pub fn get_server_status(http: State<'_, Arc<HttpState>>) -> ServerStatus {
    let port = *http.port.lock().unwrap();
    ServerStatus {
        running: port.is_some(),
        port: port.unwrap_or(0),
    }
}

#[tauri::command]
pub fn save_preset(
    presets: State<'_, Arc<PresetService>>,
    name: String,
    template: String,
    fields: HashMap<String, String>,
) -> Result<Vec<Preset>, CommandError> {
    presets
        .save(name, template, fields)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn list_presets(presets: State<'_, Arc<PresetService>>) -> Result<Vec<Preset>, CommandError> {
    Ok(presets.list())
}

#[tauri::command]
pub fn delete_preset(
    presets: State<'_, Arc<PresetService>>,
    name: String,
) -> Result<Vec<Preset>, CommandError> {
    presets.delete(&name).map_err(CommandError::from)
}

#[tauri::command]
pub fn get_config(config: State<'_, Arc<ConfigService>>) -> Result<AppConfig, CommandError> {
    Ok(config.get())
}

#[tauri::command]
pub fn set_language(
    config: State<'_, Arc<ConfigService>>,
    lang: String,
) -> Result<AppConfig, CommandError> {
    config.set_language(lang).map_err(CommandError::from)
}

#[tauri::command]
pub fn set_overlays_dir(
    config: State<'_, Arc<ConfigService>>,
    path: String,
) -> Result<AppConfig, CommandError> {
    config.set_overlays_dir(path).map_err(CommandError::from)
}
