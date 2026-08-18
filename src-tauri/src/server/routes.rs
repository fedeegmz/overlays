use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;

use super::state::AppState;
use super::ws;

async fn serve_overlay(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Response, StatusCode> {
    let overlay_dir = state.overlay_dir.lock().unwrap().clone();
    let file_path = overlay_dir.join(&path);

    if !file_path.starts_with(&overlay_dir) {
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

    Ok((
        [(header::CONTENT_TYPE, mime)],
        content,
    )
        .into_response())
}

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/overlay/{*path}", get(serve_overlay))
        .route("/api/templates", get(crate::templates::list_templates))
        .route("/ws", get(ws::ws_handler))
        .with_state(state)
}
