use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::StreamExt;

use super::state::HttpState;
use crate::application::ports::OverlayBus;

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<HttpState>>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<HttpState>) {
    state.connected.fetch_add(1, Ordering::SeqCst);

    let mut rx = state.bus.subscribe();

    for payload in state.bus.snapshot() {
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
                        for payload in state.bus.snapshot() {
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
