//! Anthropic Messages API adapter (blocking). Raw HTTP errors map onto the
//! `AiError` transport vocabulary; normalization happens in the service.
//!
//! The endpoint is injectable so unit tests run against a local TCP server —
//! no network access in tests.

use serde_json::Value;

use crate::application::ports::AiProvider;
use crate::domain::ai::{AiError, AiText, ProviderKind};

const DEFAULT_ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 4096;
const DEFAULT_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// Default generation model (Anthropic naming; overridable per request via
/// the `model` argument of `generate`).
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-5";

pub struct AnthropicProvider {
    endpoint: String,
    client: reqwest::blocking::Client,
}

impl AnthropicProvider {
    pub fn new() -> Self {
        Self::with_endpoint_and_timeout(DEFAULT_ENDPOINT.to_string(), DEFAULT_REQUEST_TIMEOUT)
    }

    /// Test seam: point at a local server with a tight timeout.
    pub fn with_endpoint_and_timeout(endpoint: String, timeout: std::time::Duration) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .no_proxy()
            .build()
            .expect("reqwest client build must not fail");
        Self { endpoint, client }
    }
}

impl Default for AnthropicProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AiProvider for AnthropicProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Anthropic
    }

    fn models(&self) -> Vec<String> {
        vec![DEFAULT_MODEL.to_string()]
    }

    fn generate(
        &self,
        model: &str,
        prompt: &str,
        system: &str,
        key: &str,
    ) -> Result<AiText, AiError> {
        let response = self
            .client
            .post(&self.endpoint)
            .header("x-api-key", key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": model,
                "max_tokens": DEFAULT_MAX_TOKENS,
                "system": system,
                "messages": [{ "role": "user", "content": prompt }],
            }))
            .send()
            .map_err(map_transport_error)?;

        match response.status().as_u16() {
            200 => {}
            401 => return Err(AiError::Unauthorized),
            429 => return Err(AiError::RateLimited),
            status => {
                return Err(AiError::InvalidResponse {
                    detail: format!("unexpected HTTP status {status}"),
                })
            }
        }

        let body: Value = response.json().map_err(|e| AiError::InvalidResponse {
            detail: format!("response body is not JSON: {e}"),
        })?;

        let text = body["content"][0]["text"]
            .as_str()
            .ok_or_else(|| AiError::InvalidResponse {
                detail: "missing content[0].text in response".into(),
            })?
            .to_string();
        let truncated = body["stop_reason"].as_str() == Some("max_tokens");

        Ok(AiText { text, truncated })
    }
}

fn map_transport_error(e: reqwest::Error) -> AiError {
    if e.is_timeout() {
        AiError::Timeout
    } else {
        AiError::Network {
            detail: e.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    /// Minimal single-request HTTP server; returns (port, request receiver).
    fn start_server(status: u16, body: String) -> (u16, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut buf = [0u8; 8192];
            let n = stream.read(&mut buf).unwrap();
            let _ = tx.send(String::from_utf8_lossy(&buf[..n]).to_string());
            let head = format!(
                "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(body.as_bytes());
        });
        (port, rx)
    }

    fn provider(endpoint: String, timeout_ms: u64) -> AnthropicProvider {
        AnthropicProvider::with_endpoint_and_timeout(
            endpoint,
            std::time::Duration::from_millis(timeout_ms),
        )
    }

    fn valid_body(stop_reason: &str) -> String {
        format!(
            r#"{{"id":"msg_1","type":"message","content":[{{"type":"text","text":"SALIDA JSON"}}],"stop_reason":"{stop_reason}"}}"#
        )
    }

    #[test]
    fn sends_contract_headers_and_parses_text() {
        let (port, rx) = start_server(200, valid_body("end_turn"));
        let p = provider(format!("http://127.0.0.1:{port}/v1/messages"), 2000);

        let result = p.generate("model-x", "mi prompt", "system rules", "sk-test-1234");

        let req = rx.recv().unwrap();
        assert!(
            req.contains("x-api-key: sk-test-1234"),
            "missing key header"
        );
        assert!(req.contains("anthropic-version: 2023-06-01"));
        assert!(req.contains("model-x"));
        assert!(req.contains("mi prompt"));
        assert!(req.contains("system rules"));
        let text = result.expect("valid response should parse");
        assert_eq!(text.text, "SALIDA JSON");
        assert!(!text.truncated);
    }

    #[test]
    fn max_tokens_stop_reason_marks_truncated() {
        let (port, _rx) = start_server(200, valid_body("max_tokens"));
        let p = provider(format!("http://127.0.0.1:{port}/v1/messages"), 2000);

        let text = p.generate("m", "p", "s", "k").unwrap();
        assert!(text.truncated);
    }

    #[test]
    fn unauthorized_maps_to_401() {
        let (port, _rx) = start_server(
            401,
            r#"{"type":"error","error":{"type":"authentication_error"}}"#.to_string(),
        );
        let p = provider(format!("http://127.0.0.1:{port}/v1/messages"), 2000);

        assert!(matches!(
            p.generate("m", "p", "s", "wrong-key"),
            Err(AiError::Unauthorized)
        ));
    }

    #[test]
    fn rate_limited_maps_to_429() {
        let (port, _rx) = start_server(
            429,
            r#"{"type":"error","error":{"type":"rate_limit_error"}}"#.to_string(),
        );
        let p = provider(format!("http://127.0.0.1:{port}/v1/messages"), 2000);

        assert!(matches!(
            p.generate("m", "p", "s", "k"),
            Err(AiError::RateLimited)
        ));
    }

    #[test]
    fn malformed_body_is_invalid_response() {
        let (port, _rx) = start_server(200, "not json at all".to_string());
        let p = provider(format!("http://127.0.0.1:{port}/v1/messages"), 2000);

        assert!(matches!(
            p.generate("m", "p", "s", "k"),
            Err(AiError::InvalidResponse { .. })
        ));
    }

    #[test]
    fn missing_text_field_is_invalid_response() {
        let (port, _rx) = start_server(
            200,
            r#"{"content":[],"stop_reason":"end_turn"}"#.to_string(),
        );
        let p = provider(format!("http://127.0.0.1:{port}/v1/messages"), 2000);

        assert!(matches!(
            p.generate("m", "p", "s", "k"),
            Err(AiError::InvalidResponse { .. })
        ));
    }

    #[test]
    fn slow_server_maps_to_timeout() {
        // Server sleeps longer than the client timeout.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                thread::sleep(std::time::Duration::from_millis(1200));
                let _ = stream.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n");
            }
        });
        let p = provider(format!("http://127.0.0.1:{port}/v1/messages"), 100);

        assert!(matches!(
            p.generate("m", "p", "s", "k"),
            Err(AiError::Timeout)
        ));
    }

    #[test]
    fn connection_refused_maps_to_network() {
        // Bind a port, note it, drop the listener, then connect — refused.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let p = provider(format!("http://127.0.0.1:{port}/v1/messages"), 1000);

        assert!(matches!(
            p.generate("m", "p", "s", "k"),
            Err(AiError::Network { .. })
        ));
    }
}
