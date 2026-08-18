use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tauri::State;

use crate::server::state::{AppState, OverlayPayload};
use crate::storage::Preset;

#[tauri::command]
pub fn send_overlay_update(
    state: State<'_, Arc<AppState>>,
    instance_id: String,
    template: String,
    action: String,
    fields: HashMap<String, String>,
) -> Result<(), String> {
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
    let json = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    let _ = state.tx.send(json);

    Ok(())
}

#[tauri::command]
pub fn list_templates(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    let overlay_dir = state.overlay_dir.lock().unwrap().clone();
    let manifest = crate::templates::discover_overlays(&overlay_dir)?;
    serde_json::to_value(&manifest).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_server_status(state: State<'_, Arc<AppState>>) -> serde_json::Value {
    let port = *state.port.lock().unwrap();
    serde_json::json!({ "running": port.is_some(), "port": port.unwrap_or(0) })
}

#[tauri::command]
pub fn save_preset(
    state: State<'_, Arc<AppState>>,
    nombre: String,
    template: String,
    fields: HashMap<String, String>,
) -> Result<Vec<Preset>, String> {
    let nombre = nombre.trim().to_string();
    if nombre.is_empty() {
        return Err("el nombre del preset no puede estar vacío".into());
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
pub fn list_presets(state: State<'_, Arc<AppState>>) -> Vec<Preset> {
    crate::storage::load_presets(&state.presets_path)
}

#[tauri::command]
pub fn delete_preset(
    state: State<'_, Arc<AppState>>,
    nombre: String,
) -> Result<Vec<Preset>, String> {
    let path = state.presets_path.clone();
    let mut presets = crate::storage::load_presets(&path);
    presets.retain(|p| p.nombre != nombre);
    crate::storage::save_presets(&path, &presets)?;
    Ok(presets)
}

#[tauri::command]
pub fn get_config(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    let app_config = crate::config::load_config(&state.config_path);
    serde_json::to_value(&app_config).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_overlays_dir(
    state: State<'_, Arc<AppState>>,
    path: String,
) -> Result<serde_json::Value, String> {
    let dir = PathBuf::from(&path);
    if !dir.is_dir() {
        return Err(format!("{path} is not a valid directory"));
    }

    let mut app_config = crate::config::load_config(&state.config_path);
    app_config.overlays_dir = Some(dir.clone());
    crate::config::save_config(&state.config_path, &app_config)?;

    *state.overlay_dir.lock().unwrap() = dir;

    serde_json::to_value(&app_config).map_err(|e| e.to_string())
}
