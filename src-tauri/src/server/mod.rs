pub mod routes;
pub mod state;
pub mod ws;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;

use state::AppState;

pub async fn start_server(state: Arc<AppState>) -> anyhow::Result<()> {
    let router = routes::build_router(state.clone());

    let mut last_err: Option<std::io::Error> = None;
    for port in 4848..=4851 {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        match TcpListener::bind(addr).await {
            Ok(listener) => {
                *state.port.lock().unwrap() = Some(port);
                eprintln!("[overlays] server listening on 127.0.0.1:{port}");
                axum::serve(listener, router.clone()).await?;
                return Ok(());
            }
            Err(e) => {
                eprintln!("[overlays] port {port} occupied, trying next");
                last_err = Some(e);
            }
        }
    }

    Err(anyhow::anyhow!(
        "no free port in range 4848-4851: {:?}",
        last_err
    ))
}

pub fn new_state(overlay_dir: PathBuf, presets_path: PathBuf, config_path: PathBuf) -> AppState {
    let (tx, _rx) = tokio::sync::broadcast::channel(128);
    AppState {
        tx,
        current: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        connected: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        port: Arc::new(std::sync::Mutex::new(None)),
        overlay_dir: Arc::new(std::sync::Mutex::new(overlay_dir)),
        presets_path,
        config_path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use futures_util::StreamExt;
    use std::sync::atomic::Ordering;
    use tower::ServiceExt;

    fn test_state() -> Arc<AppState> {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("examples");
        let presets_path = std::env::temp_dir().join("overlays-test-presets.json");
        let config_path = std::env::temp_dir().join("overlays-test-config.json");
        Arc::new(new_state(dir, presets_path, config_path))
    }

    #[tokio::test]
    async fn templates_endpoint_returns_manifest() {
        let state = test_state();
        let router = routes::build_router(state);
        let res = router
            .oneshot(
                Request::builder()
                    .uri("/api/templates")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), 200);
        let body = axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["templates"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn static_overlay_is_served() {
        let state = test_state();
        let router = routes::build_router(state);
        let res = router
            .oneshot(
                Request::builder()
                    .uri("/overlay/lower-third-basico/index.html")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), 200);
        let body = axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("script.js"));
    }

    #[tokio::test]
    async fn ws_receives_broadcast_payload() {
        let state = test_state();
        let router = routes::build_router(state.clone());

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
            .await
            .unwrap();

        let payload = serde_json::json!({
            "instance_id": "test-1",
            "template": "lower-third-basico",
            "action": "show",
            "fields": { "titulo": "Fede", "subtitulo": "Dev" }
        });
        state.tx.send(payload.to_string()).unwrap();

        let msg = ws.next().await.unwrap().unwrap();
        let text = match msg {
            tokio_tungstenite::tungstenite::Message::Text(t) => t.to_string(),
            other => panic!("expected Text, got {other:?}"),
        };

        let received: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(received["instance_id"], "test-1");
        assert_eq!(received["template"], "lower-third-basico");
        assert_eq!(received["fields"]["titulo"], "Fede");
    }

    #[tokio::test]
    async fn ws_counts_connections() {
        let state = test_state();
        let router = routes::build_router(state.clone());

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        let (_ws1, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert_eq!(state.connected.load(Ordering::SeqCst), 1);

        drop(_ws1);
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert_eq!(state.connected.load(Ordering::SeqCst), 0);
    }
}
