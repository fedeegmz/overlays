use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use crate::application::checklist_service::ChecklistService;
use crate::application::config_service::ConfigService;
use crate::application::group_preset_service::GroupPresetService;
use crate::application::ports::OverlayBus;
use crate::application::preset_service::PresetService;
use crate::application::template_catalog::TemplateCatalog;
use crate::domain::checklist::{Checklist, ChecklistItem};
use crate::domain::config::AppConfig;
use crate::domain::error::DomainError;
use crate::domain::group_preset::{GroupPreset, GroupPresetEntry};
use crate::domain::overlay::{OverlayAction, OverlayPayload};
use crate::domain::preset::Preset;
use crate::domain::template::Manifest;
use crate::infrastructure::error::CommandError;
use crate::infrastructure::fs_template_source;
use crate::infrastructure::http::state::HttpState;
use crate::infrastructure::lan;

#[derive(Debug, Clone, Serialize)]
pub struct ServerStatus {
    pub running: bool,
    pub port: u16,
    /// Upload page URLs reachable from the LAN; empty when no LAN address is found.
    pub lan_urls: Vec<String>,
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
    let lan_urls = port.map(lan::lan_upload_urls).unwrap_or_default();
    ServerStatus {
        running: port.is_some(),
        port: port.unwrap_or(0),
        lan_urls,
    }
}

#[tauri::command]
pub fn resolve_overlay_asset_path(
    http: State<'_, Arc<HttpState>>,
    template_id: String,
    absolute_path: String,
) -> Result<String, CommandError> {
    let overlays_dir = http.overlays_dir.get();
    let template_dir = http
        .catalog
        .template_dir(&template_id)
        .map_err(CommandError::from)?
        .ok_or_else(|| {
            CommandError::from(DomainError::AssetResolveFailed {
                path: absolute_path.clone(),
            })
        })?;
    fs_template_source::resolve_asset_path(&overlays_dir, &template_dir, &absolute_path)
        .map_err(CommandError::from)
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
pub fn list_group_presets(
    groups: State<'_, Arc<GroupPresetService>>,
) -> Result<Vec<GroupPreset>, CommandError> {
    groups.list().map_err(CommandError::from)
}

#[tauri::command]
pub fn save_group_preset(
    groups: State<'_, Arc<GroupPresetService>>,
    name: String,
    entries: Vec<GroupPresetEntry>,
) -> Result<Vec<GroupPreset>, CommandError> {
    groups.save(name, entries).map_err(CommandError::from)
}

#[tauri::command]
pub fn delete_group_preset(
    groups: State<'_, Arc<GroupPresetService>>,
    name: String,
) -> Result<Vec<GroupPreset>, CommandError> {
    groups.delete(&name).map_err(CommandError::from)
}

#[tauri::command]
pub fn list_checklists(
    checklists: State<'_, Arc<ChecklistService>>,
) -> Result<Vec<Checklist>, CommandError> {
    checklists.list().map_err(CommandError::from)
}

#[tauri::command]
pub fn save_checklist(
    checklists: State<'_, Arc<ChecklistService>>,
    id: String,
    name: String,
    items: Vec<ChecklistItem>,
) -> Result<Vec<Checklist>, CommandError> {
    checklists.save(id, name, items).map_err(CommandError::from)
}

#[tauri::command]
pub fn delete_checklist(
    checklists: State<'_, Arc<ChecklistService>>,
    id: String,
) -> Result<Vec<Checklist>, CommandError> {
    checklists.delete(&id).map_err(CommandError::from)
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

#[tauri::command]
pub fn set_uploads_enabled(
    config: State<'_, Arc<ConfigService>>,
    enabled: bool,
) -> Result<AppConfig, CommandError> {
    config
        .set_uploads_enabled(enabled)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn set_upload_dir(
    config: State<'_, Arc<ConfigService>>,
    template: Option<String>,
) -> Result<AppConfig, CommandError> {
    config.set_upload_dir(template).map_err(CommandError::from)
}
