use std::net::{IpAddr, SocketAddr};

use axum::extract::rejection::ExtensionRejection as ConnectInfoRejection;
use axum::extract::{ConnectInfo, Request};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// True for IPv4 127/8, IPv6 `::1` and IPv4-mapped loopback (`::ffff:127.x.x.x`).
pub fn is_loopback(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback(),
        IpAddr::V6(v6) => {
            v6.is_loopback() || v6.to_ipv4_mapped().is_some_and(|v4| v4.is_loopback())
        }
    }
}

/// Rejects every peer that is not loopback with an empty `403`. A request without
/// `ConnectInfo` is denied too (fail closed).
pub async fn loopback_only(
    peer: Result<ConnectInfo<SocketAddr>, ConnectInfoRejection>,
    req: Request,
    next: Next,
) -> Response {
    let allowed = peer.is_ok_and(|ConnectInfo(addr)| is_loopback(addr.ip()));
    if allowed {
        next.run(req).await
    } else {
        StatusCode::FORBIDDEN.into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::extract::connect_info::MockConnectInfo;
    use axum::http::Request;
    use axum::middleware::from_fn;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn loopback_addresses_are_allowed() {
        for a in ["127.0.0.1", "127.5.6.7", "::1", "::ffff:127.0.0.1"] {
            assert!(is_loopback(ip(a)), "{a} should be loopback");
        }
    }

    #[test]
    fn non_loopback_addresses_are_rejected() {
        for a in [
            "192.168.1.50",
            "10.0.0.1",
            "fe80::1",
            "2001:db8::1",
            "::ffff:192.168.1.50",
            "::ffff:10.0.0.1",
        ] {
            assert!(!is_loopback(ip(a)), "{a} should not be loopback");
        }
    }

    fn guarded() -> Router {
        Router::new()
            .route("/x", get(|| async { "ok" }))
            .route_layer(from_fn(loopback_only))
    }

    async fn status_for(router: Router) -> (StatusCode, Vec<u8>) {
        let res = router
            .oneshot(Request::builder().uri("/x").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let body = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
        (status, body.to_vec())
    }

    #[tokio::test]
    async fn middleware_passes_loopback_peer() {
        let addr = SocketAddr::new(ip("127.0.0.1"), 5000);
        let (status, body) = status_for(guarded().layer(MockConnectInfo(addr))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, b"ok");
    }

    #[tokio::test]
    async fn middleware_blocks_lan_peer_with_empty_body() {
        let addr = SocketAddr::new(ip("192.168.1.50"), 5000);
        let (status, body) = status_for(guarded().layer(MockConnectInfo(addr))).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn middleware_blocks_when_connect_info_missing() {
        let (status, body) = status_for(guarded()).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(body.is_empty());
    }
}
