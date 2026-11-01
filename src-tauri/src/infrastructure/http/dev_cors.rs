//! Development-only CORS for `/remote/api`.
//!
//! In `pnpm tauri dev` the phone app is served by Vite on port 1420, so its
//! calls to the backend are cross-origin. This module is compiled out of
//! release builds (`cfg(debug_assertions)` at the module declaration), where
//! the page and the API share an origin.

use axum::extract::Request;
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

const DEV_PORT_SUFFIX: &str = ":1420";

/// `http://<host>:1420` and nothing else.
fn is_dev_origin(origin: &str) -> bool {
    origin
        .strip_prefix("http://")
        .and_then(|rest| rest.strip_suffix(DEV_PORT_SUFFIX))
        .is_some_and(|host| !host.is_empty() && !host.contains(['/', '@', '?', '#', ' ', '\\']))
}

fn allow(response: &mut Response, origin: HeaderValue) {
    let headers = response.headers_mut();
    headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
    headers.insert(header::VARY, HeaderValue::from_static("Origin"));
}

/// Answers preflights and tags responses for the Vite dev origin. It sits
/// outside the access layers so a preflight (which carries no token) is not
/// rejected by them.
pub async fn dev_cors(req: Request, next: Next) -> Response {
    let origin = req
        .headers()
        .get(header::ORIGIN)
        .filter(|o| o.to_str().is_ok_and(is_dev_origin))
        .cloned();
    let Some(origin) = origin else {
        return next.run(req).await;
    };
    if req.method() == Method::OPTIONS {
        let mut response = StatusCode::NO_CONTENT.into_response();
        allow(&mut response, origin);
        let headers = response.headers_mut();
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static("GET, POST, OPTIONS"),
        );
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static("authorization, content-type, x-client-id"),
        );
        return response;
    }
    let mut response = next.run(req).await;
    allow(&mut response, origin);
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::upload_service::{UploadSettings, UploadSettingsHandle};
    use crate::infrastructure::http::remote;
    use crate::infrastructure::http::state::HttpState;
    use axum::body::Body;
    use axum::extract::connect_info::MockConnectInfo;
    use axum::http::Request as HttpRequest;
    use std::sync::Arc;
    use tower::ServiceExt;

    async fn call(method: &str, origin: Option<&str>) -> Response {
        let state = Arc::new(HttpState::for_test(UploadSettingsHandle::new(
            UploadSettings {
                enabled: true,
                dir_template: None,
            },
        )));
        state.remote.set_enabled(true).unwrap();
        let mut builder = HttpRequest::builder()
            .method(method)
            .uri("/remote/api/state");
        if let Some(origin) = origin {
            builder = builder.header(header::ORIGIN, origin);
        }
        remote::routes(state.clone())
            .with_state(state)
            .layer(MockConnectInfo(std::net::SocketAddr::new(
                "192.168.1.50".parse().unwrap(),
                5000,
            )))
            .oneshot(builder.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    #[test]
    fn only_http_origins_on_the_vite_port_match() {
        assert!(is_dev_origin("http://192.168.1.5:1420"));
        assert!(is_dev_origin("http://localhost:1420"));
        assert!(!is_dev_origin("https://192.168.1.5:1420"));
        assert!(!is_dev_origin("http://192.168.1.5:3000"));
        assert!(!is_dev_origin("http://:1420"));
        assert!(!is_dev_origin("http://evil.com/x:1420"));
        assert!(!is_dev_origin("http://a@evil.com:1420"));
    }

    #[tokio::test]
    async fn preflight_is_answered_without_a_token() {
        let res = call("OPTIONS", Some("http://192.168.1.5:1420")).await;
        assert_eq!(res.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            res.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
            "http://192.168.1.5:1420"
        );
        let allowed = res.headers()[header::ACCESS_CONTROL_ALLOW_HEADERS]
            .to_str()
            .unwrap();
        assert!(allowed.contains("authorization") && allowed.contains("x-client-id"));
    }

    #[tokio::test]
    async fn dev_origin_responses_carry_cors_even_when_rejected() {
        // No token from a LAN peer: 401, but the page must still be able to read it.
        let res = call("GET", Some("http://192.168.1.5:1420")).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            res.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
            "http://192.168.1.5:1420"
        );
    }

    #[tokio::test]
    async fn other_origins_get_no_cors_headers() {
        for origin in [Some("http://evil.example:3000"), None] {
            let res = call("GET", origin).await;
            assert!(res
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .is_none());
        }
    }
}
