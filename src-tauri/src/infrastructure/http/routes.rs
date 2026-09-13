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

    // Per-segment deny on the decoded path: ".", ".." and any dot-prefixed
    // segment (".staging", ".hidden") are forbidden (D12). The canonicalize
    // check below would reject most escapes, but a staged dir canonicalizes
    // INSIDE overlay_dir — only the explicit prefix rule blocks it.
    if path
        .split('/')
        .any(|seg| seg == "." || seg == ".." || seg.starts_with('.'))
    {
        return Err(StatusCode::FORBIDDEN);
    }

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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    use super::build_router;
    use crate::application::template_catalog::{OverlaysDirHandle, TemplateCatalog};
    use crate::infrastructure::fs_template_source::FsTemplateSource;
    use crate::infrastructure::http::state::HttpState;
    use crate::infrastructure::overlay_bus::BroadcastOverlayBus;

    fn deny_state(root: PathBuf) -> Arc<HttpState> {
        HttpState::new(
            Arc::new(BroadcastOverlayBus::new()),
            Arc::new(TemplateCatalog::new(
                Arc::new(FsTemplateSource),
                OverlaysDirHandle::default(),
            )),
            OverlaysDirHandle::new(root),
        )
        .into()
    }
    /// Unique root per test tag — parallel tests must never share a path.
    fn seeded_root(tag: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("overlays-route-deny-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".staging/generated")).unwrap();
        std::fs::create_dir_all(root.join("lower-third")).unwrap();
        std::fs::write(
            root.join(".staging/generated/index.html"),
            "<html>staged</html>",
        )
        .unwrap();
        std::fs::write(root.join("lower-third/index.html"), "<html>ok</html>").unwrap();
        std::fs::write(
            root.join("lower-third/overlay.json"),
            r#"{"name":"Lower","fields":[]}"#,
        )
        .unwrap();
        root
    }

    #[tokio::test]
    async fn denies_dot_prefixed_segments_even_when_canonical() {
        let root = seeded_root("dot");
        let router = build_router(deny_state(root.clone()));

        let res = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/overlay/.staging/generated/index.html")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), 403);

        let res = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/overlay/lower-third/index.html")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), 200);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn denies_traversal_and_encoded_dot_segments() {
        let root = seeded_root("traversal");
        let router = build_router(deny_state(root.clone()));
        for uri in [
            "/overlay/../lower-third/index.html",
            "/overlay/%2e%2e/lower-third/index.html",
            "/overlay/.hidden/x",
            "/overlay/lower-third/..",
        ] {
            let res = router
                .clone()
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(res.status(), 403, "uri {uri} should be denied");
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}
