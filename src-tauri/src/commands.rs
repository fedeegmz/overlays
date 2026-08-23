use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use crate::config::AppConfig;
use crate::error::CommandError;
use crate::server::state::{AppState, OverlayPayload};
use crate::storage::Preset;
use crate::templates::Manifest;

#[derive(Debug, Clone, Serialize)]
pub struct ServerStatus {
    pub running: bool,
    pub port: u16,
}

#[tauri::command]
pub fn send_overlay_update(
    state: State<'_, Arc<AppState>>,
    instance_id: String,
    template: String,
    action: String,
    fields: HashMap<String, String>,
) -> Result<(), CommandError> {
    let payload = OverlayPayload {
        instance_id: instance_id.clone(),
        template,
        action,
        fields,
    };

    state
        .current
        .lock()
        .unwrap()
        .insert(instance_id, payload.clone());
    let json = serde_json::to_string(&payload)
        .map_err(|_| CommandError::new(CommandError::INTERNAL))?;
    let _ = state.tx.send(json);

    Ok(())
}

#[tauri::command]
pub async fn list_templates(
    state: State<'_, Arc<AppState>>,
) -> Result<Manifest, CommandError> {
    let overlay_dir = state.overlay_dir.lock().unwrap().clone();
    crate::templates::discover_overlays(&overlay_dir).await
}

#[tauri::command]
pub fn get_server_status(state: State<'_, Arc<AppState>>) -> ServerStatus {
    let port = *state.port.lock().unwrap();
    ServerStatus {
        running: port.is_some(),
        port: port.unwrap_or(0),
    }
}

#[tauri::command]
pub fn save_preset(
    state: State<'_, Arc<AppState>>,
    nombre: String,
    template: String,
    fields: HashMap<String, String>,
) -> Result<Vec<Preset>, CommandError> {
    let nombre = nombre.trim().to_string();
    if nombre.is_empty() {
        return Err(CommandError::new(CommandError::PRESET_EMPTY_NAME));
    }

    let path = state.presets_path.clone();
    let mut presets = crate::storage::load_presets(&path);

    let preset = Preset {
        nombre: nombre.clone(),
        template,
        fields,
    };

    if let Some(existing) = presets.iter_mut().find(|p| p.nombre == nombre) {
        *existing = preset;
    } else {
        presets.push(preset);
    }

    crate::storage::save_presets(&path, &presets)?;
    Ok(presets)
}

#[tauri::command]
pub fn list_presets(state: State<'_, Arc<AppState>>) -> Result<Vec<Preset>, CommandError> {
    Ok(crate::storage::load_presets(&state.presets_path))
}

#[tauri::command]
pub fn delete_preset(
    state: State<'_, Arc<AppState>>,
    nombre: String,
) -> Result<Vec<Preset>, CommandError> {
    let path = state.presets_path.clone();
    let mut presets = crate::storage::load_presets(&path);
    presets.retain(|p| p.nombre != nombre);
    crate::storage::save_presets(&path, &presets)?;
    Ok(presets)
}

#[tauri::command]
pub fn get_config(state: State<'_, Arc<AppState>>) -> Result<AppConfig, CommandError> {
    Ok(crate::config::load_config(&state.config_path))
}

#[tauri::command]
pub fn set_language(
    state: State<'_, Arc<AppState>>,
    lang: String,
) -> Result<AppConfig, CommandError> {
    const SUPPORTED_LANGUAGES: [&str; 2] = ["es", "en"];
    if !SUPPORTED_LANGUAGES.contains(&lang.as_str()) {
        return Err(CommandError::new(CommandError::LANGUAGE_UNSUPPORTED).param("lang", lang));
    }

    let mut app_config = crate::config::load_config(&state.config_path);
    app_config.set_language(&lang);
    crate::config::save_config(&state.config_path, &app_config)?;

    Ok(app_config)
}

#[tauri::command]
pub fn set_overlays_dir(
    state: State<'_, Arc<AppState>>,
    path: String,
) -> Result<AppConfig, CommandError> {
    let dir = PathBuf::from(&path);
    if !dir.is_dir() {
        return Err(CommandError::new(CommandError::OVERLAYS_DIR_INVALID).param("path", path));
    }

    let mut app_config = crate::config::load_config(&state.config_path);
    app_config.overlays_dir = Some(dir.clone());
    crate::config::save_config(&state.config_path, &app_config)?;

    *state.overlay_dir.lock().unwrap() = dir;

    Ok(app_config)
}
