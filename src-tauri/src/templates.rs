use std::path::Path;
use std::sync::Arc;

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::server::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayMeta {
    pub id: Option<String>,
    pub name: String,
    pub fields: Vec<OverlayField>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub fields: Vec<OverlayField>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Manifest {
    pub templates: Vec<TemplateInfo>,
}

pub async fn discover_overlays(overlay_dir: &Path) -> Result<Manifest, String> {
    if !overlay_dir.is_dir() {
        return Ok(Manifest {
            templates: Vec::new(),
        });
    }

    let mut templates = Vec::new();

    let mut entries = tokio::fs::read_dir(overlay_dir)
        .await
        .map_err(|e| format!("could not read {overlay_dir:?}: {e}"))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| format!("could not read {overlay_dir:?}: {e}"))?
    {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let meta_path = path.join("overlay.json");
        if !tokio::fs::try_exists(&meta_path).await.unwrap_or(false) {
            continue;
        }

        let html_path = path.join("index.html");
        if !tokio::fs::try_exists(&html_path).await.unwrap_or(false) {
            eprintln!(
                "[overlays] skipping {:?}: no index.html found",
                path.file_name()
            );
            continue;
        }

        let content = match tokio::fs::read_to_string(&meta_path).await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[overlays] could not read {:?}: {e}", meta_path);
                continue;
            }
        };

        let meta: OverlayMeta = match serde_json::from_str(&content) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[overlays] could not parse {:?}: {e}", meta_path);
                continue;
            }
        };

        let folder_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        let id = meta.id.unwrap_or(folder_name.clone());
        let relative_path = format!("{}/index.html", folder_name);

        templates.push(TemplateInfo {
            id,
            name: meta.name,
            path: relative_path,
            fields: meta.fields,
        });
    }

    templates.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(Manifest { templates })
}

pub async fn list_templates(State(state): State<Arc<AppState>>) -> Response {
    let overlay_dir = state.overlay_dir.lock().unwrap().clone();
    match discover_overlays(&overlay_dir).await {
        Ok(manifest) => Json(manifest).into_response(),
        Err(e) => {
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e).into_response()
        }
    }
}
