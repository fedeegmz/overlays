//! The `/remote/api` surface: REST control of existing instances plus a
//! one-way state WebSocket. Mutations are REST only; the WebSocket never acts
//! on what clients send. Access control lives in `remote_auth`.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use axum::extract::rejection::ExtensionRejection as ConnectInfoRejection;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::error::RecvError;

use super::remote_auth::{revoked_close, secure};
use super::state::HttpState;
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::instance::{EventSource, InstancesSnapshot};
use crate::infrastructure::error::CommandError;
use crate::infrastructure::tauri::commands::StartGroupResult;

/// Request bodies above this are rejected with 413.
const MAX_BODY_BYTES: usize = 64 * 1024;

/// Incoming WebSocket frames above this close the session; smaller ones are ignored.
const MAX_WS_INCOMING_BYTES: usize = 4 * 1024;

/// Longest accepted `X-Client-Id`; keeps event payloads bounded.
const MAX_CLIENT_ID_CHARS: usize = 64;

#[derive(Deserialize)]
struct PairBody {
    pin: String,
}

#[derive(Deserialize, Default)]
struct FieldsBody {
    #[serde(default)]
    fields: HashMap<String, String>,
}

#[derive(Deserialize)]
struct PresetBody {
    name: String,
}

#[derive(Deserialize)]
struct PresetQuery {
    template: Option<String>,
}

/// Routes of the remote surface, already wrapped with the access layers.
pub fn routes(state: Arc<HttpState>) -> Router<Arc<HttpState>> {
    let open = Router::new().route("/remote/api/pair", post(pair));
    let protected = Router::new()
        .route("/remote/api/state", get(get_state))
        .route("/remote/api/templates", get(get_templates))
        .route("/remote/api/presets", get(get_presets))
        .route("/remote/api/groups", get(get_groups))
        .route("/remote/api/instances/{id}/show", post(show))
        .route("/remote/api/instances/{id}/update", post(update))
        .route("/remote/api/instances/{id}/hide", post(hide))
        .route("/remote/api/instances/{id}/preset", post(apply_preset))
        .route("/remote/api/groups/{name}/start", post(start_group))
        .route("/remote/api/ws", get(ws_handler));
    secure(state, open, protected).layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
}

fn status_for(error: &DomainError) -> StatusCode {
    match error {
        DomainError::InstanceNotFound { .. }
        | DomainError::PresetNotFound { .. }
        | DomainError::GroupPresetNotFound { .. }
        | DomainError::TemplateNotFound { .. } => StatusCode::NOT_FOUND,
        DomainError::InstanceIdTaken { .. } => StatusCode::CONFLICT,
        DomainError::RemoteDisabled => StatusCode::FORBIDDEN,
        DomainError::RemoteUnauthorized => StatusCode::UNAUTHORIZED,
        DomainError::RemotePairingRateLimited => StatusCode::TOO_MANY_REQUESTS,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn respond<T: Serialize>(result: DomainResult<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(e) => (status_for(&e), Json(CommandError::from(e))).into_response(),
    }
}

/// Who is acting: the `X-Client-Id` of the tab, so it can tell its own echo.
fn source(headers: &HeaderMap) -> EventSource {
    let id = headers
        .get("x-client-id")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.chars().take(MAX_CLIENT_ID_CHARS).collect::<String>())
        .unwrap_or_else(|| "remote".to_string());
    EventSource::remote(id)
}

async fn pair(
    State(state): State<Arc<HttpState>>,
    peer: Result<ConnectInfo<SocketAddr>, ConnectInfoRejection>,
    Json(body): Json<PairBody>,
) -> Response {
    // Without a peer address every request shares one rate-limit bucket.
    let ip = peer
        .map(|ConnectInfo(addr)| addr.ip().to_canonical())
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    respond(
        state
            .remote
            .pair(ip, &body.pin)
            .map(|token| serde_json::json!({ "token": token })),
    )
}

async fn get_state(State(state): State<Arc<HttpState>>, headers: HeaderMap) -> Response {
    Json(state.instances.list(source(&headers))).into_response()
}

async fn get_templates(State(state): State<Arc<HttpState>>) -> Response {
    respond(state.catalog.manifest())
}

async fn get_presets(
    State(state): State<Arc<HttpState>>,
    Query(query): Query<PresetQuery>,
) -> Response {
    let mut presets = state.presets.list();
    if let Some(template) = query.template {
        presets.retain(|p| p.template == template);
    }
    Json(presets).into_response()
}

async fn get_groups(State(state): State<Arc<HttpState>>) -> Response {
    respond(state.group_presets.list())
}

async fn show(
    State(state): State<Arc<HttpState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Option<Json<FieldsBody>>,
) -> Response {
    let fields = body.map(|Json(b)| b.fields).unwrap_or_default();
    respond(state.instances.show_with(&id, fields, source(&headers)))
}

async fn update(
    State(state): State<Arc<HttpState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Option<Json<FieldsBody>>,
) -> Response {
    let fields = body.map(|Json(b)| b.fields).unwrap_or_default();
    respond(state.instances.update_with(&id, fields, source(&headers)))
}

async fn hide(
    State(state): State<Arc<HttpState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    respond(state.instances.hide(&id, source(&headers)))
}

async fn apply_preset(
    State(state): State<Arc<HttpState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<PresetBody>,
) -> Response {
    respond(
        state
            .instances
            .apply_preset(&id, &body.name, source(&headers)),
    )
}

/// The `{name}` segment arrives percent-decoded, so names with spaces or `/` work.
async fn start_group(
    State(state): State<Arc<HttpState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> Response {
    respond(
        state
            .instances
            .start_group(&name, source(&headers))
            .map(|(results, snapshot)| StartGroupResult { results, snapshot }),
    )
}

#[derive(Serialize)]
struct StateMessage<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    #[serde(flatten)]
    snapshot: &'a InstancesSnapshot,
}

async fn send_state(socket: &mut WebSocket, snapshot: &InstancesSnapshot) -> bool {
    let message = StateMessage {
        kind: "state",
        snapshot,
    };
    match serde_json::to_string(&message) {
        Ok(json) => socket.send(Message::Text(json.into())).await.is_ok(),
        Err(_) => false,
    }
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<HttpState>>) -> Response {
    ws.max_message_size(MAX_WS_INCOMING_BYTES)
        .max_frame_size(MAX_WS_INCOMING_BYTES)
        .on_upgrade(move |socket| session(socket, state))
}

/// One remote viewer: the current snapshot, then every change, until the
/// client leaves or access is revoked or switched off (close code 4001).
async fn session(mut socket: WebSocket, state: Arc<HttpState>) {
    // Subscribe before reading the snapshot so no change falls in the gap.
    let mut revoked = state.sessions.subscribe();
    let mut events = state.events.subscribe();
    let initial = state.instances.list(EventSource::desktop("server"));
    let mut last_sent = initial.revision;
    if !send_state(&mut socket, &initial).await {
        return;
    }

    loop {
        tokio::select! {
            changed = revoked.changed() => {
                if changed.is_ok() {
                    let _ = socket.send(revoked_close()).await;
                }
                break;
            }
            event = events.recv() => {
                let snapshot = match event {
                    Ok(snapshot) => snapshot,
                    Err(RecvError::Lagged(_)) => match state.events.latest() {
                        Some(latest) => latest,
                        None => continue,
                    },
                    Err(RecvError::Closed) => break,
                };
                if snapshot.revision <= last_sent {
                    continue;
                }
                last_sent = snapshot.revision;
                if !send_state(&mut socket, &snapshot).await {
                    break;
                }
            }
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                // The socket is one-way: anything else the client sends is ignored.
                Some(Ok(_)) => {}
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::upload_service::{UploadSettings, UploadSettingsHandle};
    use crate::domain::group_preset::GroupPresetEntry;
    use crate::domain::instance::EventSource;
    use axum::body::Body;
    use axum::extract::connect_info::MockConnectInfo;
    use axum::http::{Request, StatusCode};
    use futures_util::{SinkExt, StreamExt};
    use std::collections::HashMap;
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message as WsMessage;
    use tower::ServiceExt;

    const LAN: &str = "192.168.1.50";
    const TEMPLATE: &str = "lower-third-basico";

    fn remote_state() -> Arc<HttpState> {
        let state = Arc::new(HttpState::for_test(UploadSettingsHandle::new(
            UploadSettings {
                enabled: true,
                dir_template: None,
            },
        )));
        state.remote.set_enabled(true).unwrap();
        state
    }

    fn router(state: &Arc<HttpState>) -> Router {
        routes(state.clone()).with_state(state.clone())
    }

    fn token(state: &Arc<HttpState>) -> String {
        let offer = state.remote.start_pairing().unwrap();
        state.remote.pair(LAN.parse().unwrap(), &offer.pin).unwrap()
    }

    /// One request from a LAN peer carrying `token` (when given).
    async fn send(
        state: &Arc<HttpState>,
        token: Option<&str>,
        method: &str,
        uri: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(token) = token {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        let body = match body {
            Some(json) => {
                builder = builder.header("content-type", "application/json");
                Body::from(json.to_string())
            }
            None => Body::empty(),
        };
        let res = router(state)
            .layer(MockConnectInfo(std::net::SocketAddr::new(
                LAN.parse().unwrap(),
                5000,
            )))
            .oneshot(builder.body(body).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    fn create_instance(state: &Arc<HttpState>) -> String {
        state
            .instances
            .create(TEMPLATE, None, None, EventSource::desktop("desk"))
            .unwrap()
            .0
    }

    fn entry(template: &str) -> GroupPresetEntry {
        GroupPresetEntry {
            template: template.into(),
            fields: HashMap::new(),
            instance_id: None,
            custom_name: None,
        }
    }

    #[tokio::test]
    async fn pair_with_valid_pin_returns_token() {
        let state = remote_state();
        let offer = state.remote.start_pairing().unwrap();

        let (status, body) = send(
            &state,
            None,
            "POST",
            "/remote/api/pair",
            Some(serde_json::json!({ "pin": offer.pin })),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        let token = body["token"].as_str().unwrap();
        assert!(state.remote.verify_token(token));
    }

    #[tokio::test]
    async fn pair_with_wrong_pin_is_401_and_floods_are_429() {
        let state = remote_state();
        state.remote.start_pairing().unwrap();
        let wrong = serde_json::json!({ "pin": "abcdef" });

        let (status, body) = send(
            &state,
            None,
            "POST",
            "/remote/api/pair",
            Some(wrong.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "remote.unauthorized");

        for _ in 0..9 {
            send(
                &state,
                None,
                "POST",
                "/remote/api/pair",
                Some(wrong.clone()),
            )
            .await;
        }
        let (status, body) = send(&state, None, "POST", "/remote/api/pair", Some(wrong)).await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(body["code"], "remote.pairing_rate_limited");
    }

    #[tokio::test]
    async fn pair_while_disabled_is_403() {
        let state = remote_state();
        state.remote.set_enabled(false).unwrap();
        let (status, body) = send(
            &state,
            None,
            "POST",
            "/remote/api/pair",
            Some(serde_json::json!({ "pin": "123456" })),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "remote.disabled");
    }

    #[tokio::test]
    async fn control_routes_require_a_token() {
        let state = remote_state();
        for (method, uri) in [
            ("GET", "/remote/api/state"),
            ("GET", "/remote/api/templates"),
            ("GET", "/remote/api/presets"),
            ("GET", "/remote/api/groups"),
            ("POST", "/remote/api/instances/x/show"),
            ("POST", "/remote/api/groups/g/start"),
        ] {
            let (status, body) = send(&state, None, method, uri, None).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri}");
            assert_eq!(body["code"], "remote.unauthorized");
        }
    }

    #[tokio::test]
    async fn get_state_returns_snapshot() {
        let state = remote_state();
        let token = token(&state);
        let id = create_instance(&state);

        let (status, body) = send(&state, Some(&token), "GET", "/remote/api/state", None).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["revision"], 1);
        assert_eq!(body["instances"][0]["id"], id.as_str());
        assert_eq!(body["instances"][0]["templateId"], TEMPLATE);
        assert!(body["instances"][0]["displayName"].is_string());
    }

    #[tokio::test]
    async fn get_templates_returns_manifest_with_folders() {
        let state = remote_state();
        let token = token(&state);

        let (status, body) = send(&state, Some(&token), "GET", "/remote/api/templates", None).await;

        assert_eq!(status, StatusCode::OK);
        let templates = body["templates"].as_array().unwrap();
        assert!(!templates.is_empty());
        assert!(templates.iter().all(|t| t["folder"].is_string()));
        assert!(body["folders"].is_array());
    }

    #[tokio::test]
    async fn get_presets_and_groups() {
        let state = remote_state();
        let token = token(&state);
        let fields = HashMap::from([("titulo".to_string(), "A".to_string())]);
        state
            .presets
            .save("P1".into(), TEMPLATE.into(), fields.clone())
            .unwrap();
        state
            .presets
            .save("P2".into(), "titulo-centrado".into(), fields)
            .unwrap();
        state
            .group_presets
            .save("Show".into(), vec![entry(TEMPLATE)])
            .unwrap();

        let (_, all) = send(&state, Some(&token), "GET", "/remote/api/presets", None).await;
        assert_eq!(all.as_array().unwrap().len(), 2);

        let (status, one) = send(
            &state,
            Some(&token),
            "GET",
            &format!("/remote/api/presets?template={TEMPLATE}"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(one.as_array().unwrap().len(), 1);
        assert_eq!(one[0]["name"], "P1");

        let (status, groups) = send(&state, Some(&token), "GET", "/remote/api/groups", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(groups[0]["name"], "Show");
        assert_eq!(groups[0]["entries"][0]["template"], TEMPLATE);
    }

    #[tokio::test]
    async fn show_update_hide_emit_overlay_payload() {
        let state = remote_state();
        let token = token(&state);
        let id = create_instance(&state);
        let mut rx = state.bus.subscribe();

        for (action, body) in [
            ("show", serde_json::json!({ "fields": { "titulo": "Hi" } })),
            (
                "update",
                serde_json::json!({ "fields": { "titulo": "Yo" } }),
            ),
            ("hide", serde_json::json!({})),
        ] {
            let (status, snap) = send(
                &state,
                Some(&token),
                "POST",
                &format!("/remote/api/instances/{id}/{action}"),
                Some(body),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{action}");
            assert_eq!(snap["source"]["kind"], "remote");

            let published: serde_json::Value =
                serde_json::from_str(&rx.recv().await.unwrap()).unwrap();
            assert_eq!(published["action"], action);
            assert_eq!(published["instance_id"], id.as_str());
            assert_eq!(published["template"], TEMPLATE);
            if action == "update" {
                assert_eq!(published["fields"]["titulo"], "Yo");
            }
        }
    }

    #[tokio::test]
    async fn show_and_hide_work_without_a_body() {
        let state = remote_state();
        let token = token(&state);
        let id = create_instance(&state);
        let (status, snap) = send(
            &state,
            Some(&token),
            "POST",
            &format!("/remote/api/instances/{id}/show"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(snap["instances"][0]["visible"], true);
    }

    #[tokio::test]
    async fn update_drops_file_type_and_unknown_keys_when_remote() {
        let state = remote_state();
        let token = token(&state);
        let id = create_instance(&state);

        let (status, snap) = send(
            &state,
            Some(&token),
            "POST",
            &format!("/remote/api/instances/{id}/update"),
            Some(serde_json::json!({ "fields": { "titulo": "Ok", "ghost": "x" } })),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        let fields = &snap["instances"][0]["fields"];
        assert_eq!(fields["titulo"], "Ok");
        assert!(fields.get("ghost").is_none());
    }

    #[tokio::test]
    async fn client_id_header_becomes_the_remote_source() {
        let state = remote_state();
        let token = token(&state);
        let id = create_instance(&state);
        let res = router(&state)
            .layer(MockConnectInfo(std::net::SocketAddr::new(
                LAN.parse().unwrap(),
                5000,
            )))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/remote/api/instances/{id}/hide"))
                    .header("authorization", format!("Bearer {token}"))
                    .header("x-client-id", "phone-7")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let snap: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(snap["source"]["clientId"], "phone-7");
        assert_eq!(snap["source"]["kind"], "remote");
    }

    #[tokio::test]
    async fn instance_unknown_returns_404_instance_not_found() {
        let state = remote_state();
        let token = token(&state);
        for action in ["show", "update", "hide"] {
            let (status, body) = send(
                &state,
                Some(&token),
                "POST",
                &format!("/remote/api/instances/nope/{action}"),
                None,
            )
            .await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{action}");
            assert_eq!(body["code"], "instance.not_found");
        }
    }

    #[tokio::test]
    async fn preset_applies_and_unknown_returns_404_preset_not_found() {
        let state = remote_state();
        let token = token(&state);
        let id = create_instance(&state);
        state
            .presets
            .save(
                "P1".into(),
                TEMPLATE.into(),
                HashMap::from([("titulo".to_string(), "FromPreset".to_string())]),
            )
            .unwrap();

        let (status, snap) = send(
            &state,
            Some(&token),
            "POST",
            &format!("/remote/api/instances/{id}/preset"),
            Some(serde_json::json!({ "name": "P1" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(snap["instances"][0]["fields"]["titulo"], "FromPreset");

        let (status, body) = send(
            &state,
            Some(&token),
            "POST",
            &format!("/remote/api/instances/{id}/preset"),
            Some(serde_json::json!({ "name": "missing" })),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["code"], "preset.not_found");
    }

    #[tokio::test]
    async fn group_unknown_returns_404_group_preset_not_found() {
        let state = remote_state();
        let token = token(&state);
        let (status, body) = send(
            &state,
            Some(&token),
            "POST",
            "/remote/api/groups/nope/start",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["code"], "group_preset.not_found");
    }

    #[tokio::test]
    async fn group_start_returns_results() {
        let state = remote_state();
        let token = token(&state);
        state
            .group_presets
            .save("Show".into(), vec![entry(TEMPLATE)])
            .unwrap();

        let (status, body) = send(
            &state,
            Some(&token),
            "POST",
            "/remote/api/groups/Show/start",
            None,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["results"][0]["ok"], true);
        assert_eq!(body["results"][0]["template"], TEMPLATE);
        assert_eq!(body["snapshot"]["instances"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn group_start_decodes_name_with_spaces_and_slash() {
        let state = remote_state();
        let token = token(&state);
        state
            .group_presets
            .save("Intro / Day 1".into(), vec![entry(TEMPLATE)])
            .unwrap();

        let (status, body) = send(
            &state,
            Some(&token),
            "POST",
            "/remote/api/groups/Intro%20%2F%20Day%201/start",
            None,
        )
        .await;

        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["results"][0]["ok"], true);
    }

    #[tokio::test]
    async fn no_create_remove_rename_move_routes_404_405() {
        let state = remote_state();
        let token = token(&state);
        let id = create_instance(&state);
        for (method, uri) in [
            ("POST", "/remote/api/instances".to_string()),
            ("PUT", "/remote/api/instances".to_string()),
            ("DELETE", format!("/remote/api/instances/{id}")),
            ("POST", format!("/remote/api/instances/{id}")),
            ("POST", format!("/remote/api/instances/{id}/rename")),
            ("POST", format!("/remote/api/instances/{id}/move")),
            ("POST", "/remote/api/presets".to_string()),
            ("DELETE", "/remote/api/presets".to_string()),
            ("POST", "/remote/api/groups".to_string()),
            ("DELETE", "/remote/api/groups/Show".to_string()),
        ] {
            let (status, _) = send(&state, Some(&token), method, &uri, None).await;
            assert!(
                status == StatusCode::NOT_FOUND || status == StatusCode::METHOD_NOT_ALLOWED,
                "{method} {uri} -> {status}"
            );
        }
        assert_eq!(state.instances.list(EventSource::desktop("d")).revision, 1);
    }

    #[tokio::test]
    async fn body_over_64kib_rejected() {
        let state = remote_state();
        let token = token(&state);
        let id = create_instance(&state);
        let big = "x".repeat(65 * 1024);

        let (status, _) = send(
            &state,
            Some(&token),
            "POST",
            &format!("/remote/api/instances/{id}/update"),
            Some(serde_json::json!({ "fields": { "titulo": big } })),
        )
        .await;

        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    }

    type Ws = tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >;

    /// Real socket on loopback (allowed without a token while remote is on).
    async fn connect(state: &Arc<HttpState>) -> Ws {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = router(state);
        tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .await
            .unwrap();
        });
        let (ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/remote/api/ws"))
            .await
            .unwrap();
        ws
    }

    async fn next_json(ws: &mut Ws) -> serde_json::Value {
        let msg = tokio::time::timeout(std::time::Duration::from_secs(2), ws.next())
            .await
            .expect("expected a message")
            .unwrap()
            .unwrap();
        match msg {
            WsMessage::Text(t) => serde_json::from_str(t.as_str()).unwrap(),
            other => panic!("expected text, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ws_sends_snapshot_on_connect() {
        let state = remote_state();
        let id = create_instance(&state);
        let mut ws = connect(&state).await;

        let first = next_json(&mut ws).await;

        assert_eq!(first["type"], "state");
        assert_eq!(first["revision"], 1);
        assert_eq!(first["instances"][0]["id"], id.as_str());
    }

    #[tokio::test]
    async fn ws_receives_change_events_with_source_and_revision() {
        let state = remote_state();
        let mut ws = connect(&state).await;
        let first = next_json(&mut ws).await;
        assert_eq!(first["revision"], 0);

        state
            .instances
            .create(TEMPLATE, None, None, EventSource::desktop("desk-1"))
            .unwrap();
        let created = next_json(&mut ws).await;
        assert_eq!(created["type"], "state");
        assert_eq!(created["revision"], 1);
        assert_eq!(created["source"]["kind"], "desktop");
        assert_eq!(created["source"]["clientId"], "desk-1");
        assert_eq!(created["instances"].as_array().unwrap().len(), 1);

        let id = created["instances"][0]["id"].as_str().unwrap().to_string();
        state
            .instances
            .hide(&id, EventSource::remote("phone-2"))
            .unwrap();
        let hidden = next_json(&mut ws).await;
        assert_eq!(hidden["revision"], 2);
        assert_eq!(hidden["source"]["kind"], "remote");
        assert_eq!(hidden["source"]["clientId"], "phone-2");
    }

    #[tokio::test]
    async fn ws_ignores_incoming_and_limits_4kib() {
        let state = remote_state();
        let mut ws = connect(&state).await;
        next_json(&mut ws).await;

        // Small frames are ignored: no reply, the session stays usable.
        ws.send(WsMessage::Text("{\"type\":\"hide\"}".into()))
            .await
            .unwrap();
        state
            .instances
            .create(TEMPLATE, None, None, EventSource::desktop("d"))
            .unwrap();
        assert_eq!(next_json(&mut ws).await["revision"], 1);

        // A frame above 4 KiB ends the session.
        ws.send(WsMessage::Text("x".repeat(5000).into()))
            .await
            .unwrap();
        let end = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                match ws.next().await {
                    None | Some(Err(_)) | Some(Ok(WsMessage::Close(_))) => break,
                    Some(Ok(_)) => {}
                }
            }
        })
        .await;
        assert!(end.is_ok(), "oversized frame must close the socket");
    }

    #[tokio::test]
    async fn ws_closes_4001_on_revoke() {
        let state = remote_state();
        let mut ws = connect(&state).await;
        next_json(&mut ws).await;

        state.remote.revoke_all().unwrap();

        let msg = tokio::time::timeout(std::time::Duration::from_secs(2), ws.next())
            .await
            .expect("socket should close promptly")
            .unwrap()
            .unwrap();
        match msg {
            WsMessage::Close(Some(frame)) => assert_eq!(u16::from(frame.code), 4001),
            other => panic!("expected close 4001, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ws_refuses_peers_without_a_valid_token() {
        let state = remote_state();
        let (status, _) = send(&state, None, "GET", "/remote/api/ws", None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn remote_routes_mounted_in_the_main_router() {
        let state = remote_state();
        let token = token(&state);
        let res = crate::infrastructure::http::routes::build_router(state.clone())
            .layer(MockConnectInfo(std::net::SocketAddr::new(
                LAN.parse().unwrap(),
                5000,
            )))
            .oneshot(
                Request::builder()
                    .uri("/remote/api/state")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}
