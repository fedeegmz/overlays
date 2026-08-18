use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::StreamExt;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::state::{AppState, OverlayPayload};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

fn current_payloads(state: &AppState) -> Vec<OverlayPayload> {
    state.current.lock().unwrap().values().cloned().collect()
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    state.connected.fetch_add(1, Ordering::SeqCst);

    let mut rx = state.tx.subscribe();

    for payload in current_payloads(&state) {
        if let Ok(json) = serde_json::to_string(&payload) {
            if socket.send(Message::Text(json.into())).await.is_err() {
                state.connected.fetch_sub(1, Ordering::SeqCst);
                return;
            }
        }
    }

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(text) => {
                        if socket.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        for payload in current_payloads(&state) {
                            if let Ok(json) = serde_json::to_string(&payload) {
                                if socket.send(Message::Text(json.into())).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            incoming = socket.next() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }

    state.connected.fetch_sub(1, Ordering::SeqCst);
}
