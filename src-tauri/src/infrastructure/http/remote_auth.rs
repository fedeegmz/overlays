//! Access control for the `/remote*` surface.
//!
//! Order of checks: the toggle (403 `remote.disabled`, loopback included),
//! then loopback-or-token (401 `remote.unauthorized`). `/pair` skips only the
//! token step. `/overlay`, `/ws` and `/api/*` are untouched by this module and
//! stay loopback-only no matter what token a peer presents.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::rejection::ExtensionRejection as ConnectInfoRejection;
use axum::extract::ws::{CloseFrame, Message};
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::{from_fn_with_state, Next};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};

use super::guard::is_loopback;
use super::state::HttpState;
use crate::infrastructure::error::CommandError;

/// WebSocket close code sent when a session is revoked or remote access is turned off.
pub const REVOKED_CLOSE_CODE: u16 = 4001;

/// The close frame every remote WebSocket sends on revocation.
pub fn revoked_close() -> Message {
    Message::Close(Some(CloseFrame {
        code: REVOKED_CLOSE_CODE,
        reason: "revoked".into(),
    }))
}

pub fn error_response(status: StatusCode, code: &str) -> Response {
    (status, Json(CommandError::new(code))).into_response()
}

/// Wraps the remote routes: every route (and every unknown path) needs the
/// toggle on; `protected` additionally needs loopback or a valid token;
/// `open` (pairing) needs only the toggle.
pub fn secure(
    state: Arc<HttpState>,
    open: Router<Arc<HttpState>>,
    protected: Router<Arc<HttpState>>,
) -> Router<Arc<HttpState>> {
    protected
        .route_layer(from_fn_with_state(state.clone(), remote_auth))
        .merge(open)
        .layer(from_fn_with_state(state, remote_enabled))
}

/// Rejects every request while remote access is off, loopback included.
pub async fn remote_enabled(
    State(state): State<Arc<HttpState>>,
    req: Request,
    next: Next,
) -> Response {
    if state.remote.is_enabled() {
        next.run(req).await
    } else {
        error_response(StatusCode::FORBIDDEN, CommandError::REMOTE_DISABLED)
    }
}

/// Toggle on, and either a loopback peer or a valid token. A request without
/// `ConnectInfo` is treated as non-loopback (fail closed).
pub async fn remote_auth(
    State(state): State<Arc<HttpState>>,
    peer: Result<ConnectInfo<SocketAddr>, ConnectInfoRejection>,
    req: Request,
    next: Next,
) -> Response {
    if !state.remote.is_enabled() {
        return error_response(StatusCode::FORBIDDEN, CommandError::REMOTE_DISABLED);
    }
    let loopback = peer.is_ok_and(|ConnectInfo(addr)| is_loopback(addr.ip()));
    let authorized =
        loopback || presented_token(&req).is_some_and(|token| state.remote.verify_token(&token));
    if authorized {
        next.run(req).await
    } else {
        error_response(StatusCode::UNAUTHORIZED, CommandError::REMOTE_UNAUTHORIZED)
    }
}

/// `Authorization: Bearer <token>`; the WebSocket endpoint also accepts
/// `?token=` because browsers cannot set headers on a WebSocket. The query
/// form is refused elsewhere so tokens do not end up in ordinary request URLs.
fn presented_token(req: &Request) -> Option<String> {
    if let Some(value) = req.headers().get(header::AUTHORIZATION) {
        let value = value.to_str().ok()?;
        let (scheme, token) = value.split_once(' ')?;
        return (scheme.eq_ignore_ascii_case("bearer") && !token.trim().is_empty())
            .then(|| token.trim().to_string());
    }
    if !req.uri().path().ends_with("/ws") {
        return None;
    }
    req.uri()
        .query()?
        .split('&')
        .find_map(|pair| pair.strip_prefix("token="))
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::upload_service::{UploadSettings, UploadSettingsHandle};
    use crate::infrastructure::http::routes::build_router;
    use axum::body::Body;
    use axum::extract::connect_info::MockConnectInfo;
    use axum::extract::ws::WebSocketUpgrade;
    use axum::http::Request as HttpRequest;
    use axum::routing::{get, post};
    use futures_util::StreamExt;
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message as WsMessage;
    use tower::ServiceExt;

    const LAN: &str = "192.168.1.50";

    fn test_state() -> Arc<HttpState> {
        Arc::new(HttpState::for_test(UploadSettingsHandle::new(
            UploadSettings {
                enabled: true,
                dir_template: None,
            },
        )))
    }

    /// Stand-in WebSocket endpoint built from the shared revocation helpers.
    async fn ws_probe(ws: WebSocketUpgrade, State(state): State<Arc<HttpState>>) -> Response {
        let mut epoch = state.sessions.subscribe();
        ws.on_upgrade(move |mut socket| async move {
            let _ = epoch.changed().await;
            let _ = socket.send(revoked_close()).await;
        })
    }

    fn remote_router(state: &Arc<HttpState>) -> Router {
        let open = Router::new().route("/remote/api/pair", post(|| async { "paired" }));
        let protected = Router::new()
            .route("/remote/api/state", get(|| async { "state" }))
            .route("/remote/api/ws", get(ws_probe));
        secure(state.clone(), open, protected).with_state(state.clone())
    }

    fn pair(state: &Arc<HttpState>, peer: &str) -> String {
        let offer = state.remote.start_pairing().unwrap();
        state
            .remote
            .pair(peer.parse().unwrap(), &offer.pin)
            .unwrap()
    }

    async fn call(
        router: Router,
        peer: &str,
        method: &str,
        uri: &str,
        bearer: Option<&str>,
    ) -> (StatusCode, serde_json::Value) {
        let addr = SocketAddr::new(peer.parse().unwrap(), 5000);
        let mut builder = HttpRequest::builder().method(method).uri(uri);
        if let Some(token) = bearer {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let res = router
            .layer(MockConnectInfo(addr))
            .oneshot(builder.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let body = axum::body::to_bytes(res.into_body(), 64 * 1024)
            .await
            .unwrap();
        let json = serde_json::from_slice(&body)
            .unwrap_or_else(|_| serde_json::Value::String(String::from_utf8_lossy(&body).into()));
        (status, json)
    }

    #[tokio::test]
    async fn disabled_returns_403_even_for_loopback() {
        let state = test_state();
        for peer in ["127.0.0.1", LAN] {
            for (method, uri) in [
                ("GET", "/remote/api/state"),
                ("POST", "/remote/api/pair"),
                ("GET", "/remote/unknown"),
            ] {
                let (status, body) =
                    call(remote_router(&state), peer, method, uri, Some("x")).await;
                assert_eq!(status, StatusCode::FORBIDDEN, "{peer} {uri}");
                assert_eq!(body["code"], "remote.disabled");
            }
        }
    }

    #[tokio::test]
    async fn loopback_enabled_no_token_allowed() {
        let state = test_state();
        state.remote.set_enabled(true).unwrap();
        let (status, body) = call(
            remote_router(&state),
            "127.0.0.1",
            "GET",
            "/remote/api/state",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "state");
    }

    #[tokio::test]
    async fn lan_enabled_no_token_401() {
        let state = test_state();
        state.remote.set_enabled(true).unwrap();
        let (status, body) =
            call(remote_router(&state), LAN, "GET", "/remote/api/state", None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "remote.unauthorized");
    }

    #[tokio::test]
    async fn lan_bad_token_401() {
        let state = test_state();
        state.remote.set_enabled(true).unwrap();
        let _ = pair(&state, LAN);
        let (status, _) = call(
            remote_router(&state),
            LAN,
            "GET",
            "/remote/api/state",
            Some("not-the-token"),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn lan_good_token_ok() {
        let state = test_state();
        state.remote.set_enabled(true).unwrap();
        let token = pair(&state, LAN);
        let (status, _) = call(
            remote_router(&state),
            LAN,
            "GET",
            "/remote/api/state",
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn two_concurrent_tokens_both_valid() {
        let state = test_state();
        state.remote.set_enabled(true).unwrap();
        let t1 = pair(&state, LAN);
        let t2 = pair(&state, "192.168.1.51");
        for token in [&t1, &t2] {
            let (status, _) = call(
                remote_router(&state),
                LAN,
                "GET",
                "/remote/api/state",
                Some(token),
            )
            .await;
            assert_eq!(status, StatusCode::OK);
        }
    }

    #[tokio::test]
    async fn revoke_all_invalidates_old_tokens_401() {
        let state = test_state();
        state.remote.set_enabled(true).unwrap();
        let t1 = pair(&state, LAN);
        let t2 = pair(&state, "192.168.1.51");

        state.remote.revoke_all().unwrap();

        for token in [&t1, &t2] {
            let (status, _) = call(
                remote_router(&state),
                LAN,
                "GET",
                "/remote/api/state",
                Some(token),
            )
            .await;
            assert_eq!(status, StatusCode::UNAUTHORIZED);
        }
    }

    async fn serve(state: &Arc<HttpState>) -> std::net::SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let router = remote_router(state);
        tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        });
        addr
    }

    async fn expect_close_4001(
        ws: &mut tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    ) {
        let next = tokio::time::timeout(std::time::Duration::from_secs(2), ws.next())
            .await
            .expect("socket should be closed promptly")
            .unwrap()
            .unwrap();
        match next {
            WsMessage::Close(Some(frame)) => assert_eq!(u16::from(frame.code), 4001),
            other => panic!("expected close 4001, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn revoke_all_closes_ws_with_4001() {
        let state = test_state();
        state.remote.set_enabled(true).unwrap();
        let addr = serve(&state).await;
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/remote/api/ws"))
            .await
            .unwrap();

        state.remote.revoke_all().unwrap();

        expect_close_4001(&mut ws).await;
    }

    #[tokio::test]
    async fn disable_closes_ws_keeps_hashes_and_tokens_work_on_reenable() {
        let state = test_state();
        state.remote.set_enabled(true).unwrap();
        let token = pair(&state, LAN);
        let addr = serve(&state).await;
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/remote/api/ws"))
            .await
            .unwrap();

        state.remote.set_enabled(false).unwrap();
        expect_close_4001(&mut ws).await;
        let (status, _) = call(
            remote_router(&state),
            LAN,
            "GET",
            "/remote/api/state",
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);

        state.remote.set_enabled(true).unwrap();
        let (status, _) = call(
            remote_router(&state),
            LAN,
            "GET",
            "/remote/api/state",
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn ws_query_token_accepted() {
        let state = test_state();
        state.remote.set_enabled(true).unwrap();
        let token = pair(&state, LAN);

        // A plain GET cannot upgrade, so a passed auth shows up as a non-401.
        let (status, _) = call(
            remote_router(&state),
            LAN,
            "GET",
            &format!("/remote/api/ws?token={token}"),
            None,
        )
        .await;
        assert_ne!(status, StatusCode::UNAUTHORIZED);
        assert_ne!(status, StatusCode::FORBIDDEN);

        let (status, _) = call(
            remote_router(&state),
            LAN,
            "GET",
            "/remote/api/ws?token=bad",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn query_token_is_refused_outside_the_ws_route() {
        let state = test_state();
        state.remote.set_enabled(true).unwrap();
        let token = pair(&state, LAN);
        let (status, _) = call(
            remote_router(&state),
            LAN,
            "GET",
            &format!("/remote/api/state?token={token}"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn pair_route_skips_token_check() {
        let state = test_state();
        state.remote.set_enabled(true).unwrap();
        let (status, body) =
            call(remote_router(&state), LAN, "POST", "/remote/api/pair", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "paired");
    }

    #[tokio::test]
    async fn overlay_ws_templates_stay_loopback_only_even_with_valid_token() {
        let state = test_state();
        state.remote.set_enabled(true).unwrap();
        let token = pair(&state, LAN);
        for uri in [
            "/api/templates",
            "/overlay/lower-third-basico/index.html",
            "/ws",
        ] {
            let (status, _) = call(
                build_router(state.clone()),
                LAN,
                "GET",
                &format!("{uri}?token={token}"),
                Some(&token),
            )
            .await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{uri}");
        }
    }
}
