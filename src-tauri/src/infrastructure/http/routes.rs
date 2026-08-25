use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};

use super::state::HttpState;
use super::ws;
use crate::infrastructure::error::CommandError;

async fn serve_overlay(
    State(state): State<Arc<HttpState>>,
    Path(path): Path<String>,
) -> Result<Response, StatusCode> {
    let overlay_dir = state.overlays_dir.get();
    let file_path = overlay_dir.join(&path);

    let canonical_file = dunce::canonicalize(&file_path).map_err(|_| StatusCode::FORBIDDEN)?;
    let canonical_dir = dunce::canonicalize(&overlay_dir).map_err(|_| StatusCode::FORBIDDEN)?;
    if !canonical_file.starts_with(&canonical_dir) {
        return Err(StatusCode::FORBIDDEN);
    }

    let content = tokio::fs::read(&file_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let mime = match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        _ => "application/octet-stream",
    };

    Ok(([(header::CONTENT_TYPE, mime)], content).into_response())
}

async fn list_templates(State(state): State<Arc<HttpState>>) -> Response {
    match state.catalog.manifest() {
        Ok(manifest) => Json(manifest).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CommandError::from(e)),
        )
            .into_response(),
    }
}

pub fn build_router(state: Arc<HttpState>) -> Router {
    Router::new()
        .route("/overlay/{*path}", get(serve_overlay))
        .route("/api/templates", get(list_templates))
        .route("/ws", get(ws::ws_handler))
        .with_state(state)
}
