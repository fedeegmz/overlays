use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use crate::application::config_service::ConfigService;
use crate::application::key_service::KeyService;
use crate::application::ports::OverlayBus;
use crate::application::preset_service::PresetService;
use crate::application::template_catalog::TemplateCatalog;
use crate::domain::ai::ApiKeyPresence;
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

/// Presence metadata for configured providers plus a keyring availability
/// probe for the generate gate. Probes only when provider keys exist.
#[derive(Debug, Clone, Serialize)]
pub struct ConfiguredProviders {
    pub providers: Vec<ApiKeyPresence>,
    pub keyring_available: bool,
}

#[tauri::command]
pub async fn list_configured_providers(
    keys: State<'_, Arc<KeyService>>,
) -> Result<ConfiguredProviders, CommandError> {
    let keys = keys.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let providers = keys.list();
        let keyring_available = if providers.is_empty() {
            true // nothing configured — the gate short-circuits before probing
        } else {
            providers.iter().any(|p| keys.probe(&p.provider))
        };
        ConfiguredProviders {
            providers,
            keyring_available,
        }
    })
    .await
    .map_err(|e| CommandError::new(CommandError::COMMON_INTERNAL).param("reason", e.to_string()))
}

#[tauri::command]
pub async fn add_provider_key(
    keys: State<'_, Arc<KeyService>>,
    provider: String,
    key: String,
) -> Result<Vec<ApiKeyPresence>, CommandError> {
    let keys = keys.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        keys.add(&provider, &key).map_err(CommandError::from)
    })
    .await
    .map_err(|e| CommandError::new(CommandError::COMMON_INTERNAL).param("reason", e.to_string()))?
}

#[tauri::command]
pub async fn delete_provider_key(
    keys: State<'_, Arc<KeyService>>,
    provider: String,
) -> Result<Vec<ApiKeyPresence>, CommandError> {
    let keys = keys.inner().clone();
    tauri::async_runtime::spawn_blocking(move || keys.delete(&provider).map_err(CommandError::from))
        .await
        .map_err(|e| {
            CommandError::new(CommandError::COMMON_INTERNAL).param("reason", e.to_string())
        })?
}
