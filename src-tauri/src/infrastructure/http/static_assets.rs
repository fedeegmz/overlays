//! The phone app's page and static files under `/remote/`.
//!
//! These need only the remote toggle (403 while off), not a token: the page
//! must load before the device has paired. The files come from the
//! `StaticAssets` port, so nothing here touches the filesystem.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::Router;

use super::remote_auth::remote_enabled;
use super::state::HttpState;

const PAGE: &str = "remote.html";

pub fn routes(state: Arc<HttpState>) -> Router<Arc<HttpState>> {
    Router::new()
        // Relative asset URLs only resolve under a trailing slash.
        .route("/remote", get(|| async { Redirect::temporary("/remote/") }))
        .route("/remote/", get(page))
        .route("/remote/assets/{*path}", get(asset))
        .layer(from_fn_with_state(state, remote_enabled))
}

fn file_response(state: &HttpState, path: &str) -> Response {
    match state.assets.get(path) {
        Some((bytes, mime)) => (
            [
                (header::CONTENT_TYPE, mime),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_string()),
            ],
            bytes,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn page(State(state): State<Arc<HttpState>>) -> Response {
    file_response(&state, PAGE)
}

/// Rejects anything that could climb out of `assets/` (the dev asset reader
/// joins this path onto a directory).
fn is_safe_asset_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', '\0'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

async fn asset(State(state): State<Arc<HttpState>>, Path(path): Path<String>) -> Response {
    if !is_safe_asset_path(&path) {
        return StatusCode::NOT_FOUND.into_response();
    }
    file_response(&state, &format!("assets/{path}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::upload_service::{UploadSettings, UploadSettingsHandle};
    use crate::infrastructure::test_util::MemAssets;
    use axum::body::Body;
    use axum::extract::connect_info::MockConnectInfo;
    use axum::http::{header, Request, StatusCode};
    use tower::ServiceExt;

    const LAN: &str = "192.168.1.50";

    fn state(enabled: bool) -> Arc<HttpState> {
        let mut state = HttpState::for_test(UploadSettingsHandle::new(UploadSettings {
            enabled: true,
            dir_template: None,
        }));
        state.assets = Arc::new(
            MemAssets::default()
                .with("remote.html", "<html>remote</html>", "text/html")
                .with("assets/app.js", "console.log(1)", "text/javascript")
                .with("assets/my app.css", "body{}", "text/css"),
        );
        let state = Arc::new(state);
        state.remote.set_enabled(enabled).unwrap();
        state
    }

    async fn get(
        state: &Arc<HttpState>,
        peer: &str,
        uri: &str,
    ) -> (StatusCode, Option<String>, Vec<u8>) {
        let res = routes(state.clone())
            .with_state(state.clone())
            .layer(MockConnectInfo(std::net::SocketAddr::new(
                peer.parse().unwrap(),
                5000,
            )))
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let content_type = res
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|v| v.to_str().unwrap().to_string());
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, content_type, body.to_vec())
    }

    #[tokio::test]
    async fn remote_root_serves_remote_html() {
        let state = state(true);
        let (status, content_type, body) = get(&state, LAN, "/remote/").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type.as_deref(), Some("text/html"));
        assert_eq!(body, b"<html>remote</html>");
    }

    #[tokio::test]
    async fn remote_without_trailing_slash_redirects_to_root() {
        let state = state(true);
        let res = routes(state.clone())
            .with_state(state.clone())
            .layer(MockConnectInfo(std::net::SocketAddr::new(
                LAN.parse().unwrap(),
                5000,
            )))
            .oneshot(
                Request::builder()
                    .uri("/remote")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(res.status().is_redirection());
        assert_eq!(res.headers()[header::LOCATION], "/remote/");
    }

    #[tokio::test]
    async fn assets_path_maps_to_assets() {
        let state = state(true);
        let (status, content_type, body) = get(&state, LAN, "/remote/assets/app.js").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type.as_deref(), Some("text/javascript"));
        assert_eq!(body, b"console.log(1)");

        let (status, _, body) = get(&state, LAN, "/remote/assets/my%20app.css").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, b"body{}");
    }

    #[tokio::test]
    async fn missing_asset_404() {
        let state = state(true);
        let (status, _, _) = get(&state, LAN, "/remote/assets/nope.js").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn traversal_segments_are_404() {
        let state = state(true);
        for uri in [
            "/remote/assets/../remote.html",
            "/remote/assets/%2e%2e/remote.html",
            "/remote/assets/a/%2e%2e/app.js",
            "/remote/assets/..%5Capp.js",
        ] {
            let (status, _, _) = get(&state, LAN, uri).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        }
    }

    #[tokio::test]
    async fn page_requires_only_toggle_not_token() {
        let state = state(true);
        for uri in ["/remote/", "/remote/assets/app.js"] {
            let (status, _, _) = get(&state, LAN, uri).await;
            assert_eq!(status, StatusCode::OK, "{uri}");
        }
    }

    #[tokio::test]
    async fn page_403_when_disabled() {
        let state = state(false);
        for peer in [LAN, "127.0.0.1"] {
            for uri in ["/remote/", "/remote/assets/app.js", "/remote"] {
                let (status, _, body) = get(&state, peer, uri).await;
                assert_eq!(status, StatusCode::FORBIDDEN, "{peer} {uri}");
                let text = String::from_utf8(body).unwrap();
                assert!(text.contains("remote.disabled"), "{text}");
                assert!(!text.contains("remote</html>"));
            }
        }
    }
}
