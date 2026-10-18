use std::sync::Arc;

use axum::extract::multipart::MultipartError;
use axum::extract::{DefaultBodyLimit, FromRequest, Multipart, Request, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};

use super::state::HttpState;
use crate::domain::error::{DomainError, DomainResult};
use crate::infrastructure::error::CommandError;
use crate::infrastructure::upload_sink::PendingUpload;

/// Extra room above the file limit for the multipart envelope (boundaries, headers, name field).
const ENVELOPE_SLACK_BYTES: u64 = 1024 * 1024;

/// Upper bound for the optional name field.
const MAX_NAME_FIELD_BYTES: usize = 4096;

const UPLOAD_PAGE: &str = include_str!("upload.html");

/// Routes open to every peer on the network.
pub fn routes(max_file_bytes: u64) -> Router<Arc<HttpState>> {
    let body_limit =
        usize::try_from(max_file_bytes.saturating_add(ENVELOPE_SLACK_BYTES)).unwrap_or(usize::MAX);
    Router::new()
        .route("/upload", get(page))
        .route("/upload/status", get(status))
        .route(
            "/upload",
            post(upload).layer(DefaultBodyLimit::max(body_limit)),
        )
}

async fn page() -> Html<&'static str> {
    Html(UPLOAD_PAGE)
}

async fn status(State(state): State<Arc<HttpState>>) -> Response {
    Json(serde_json::json!({ "enabled": state.upload.is_enabled() })).into_response()
}

async fn upload(State(state): State<Arc<HttpState>>, request: Request) -> Response {
    match receive(&state, request).await {
        Ok(file_name) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "file_name": file_name })),
        )
            .into_response(),
        Err(e) => error_response(e),
    }
}

async fn receive(state: &Arc<HttpState>, request: Request) -> DomainResult<String> {
    // Checked per request and before touching the body, so a disabled server reads nothing.
    if !state.upload.is_enabled() {
        return Err(DomainError::UploadDisabled);
    }

    let mut multipart = Multipart::from_request(request, state).await.map_err(|e| {
        DomainError::UploadBadRequest {
            detail: e.body_text(),
        }
    })?;

    let mut typed_name: Option<String> = None;
    while let Some(mut field) = multipart.next_field().await.map_err(multipart_error)? {
        match field.name() {
            Some("name") => typed_name = Some(read_name(&mut field).await?),
            Some("file") => {
                let original = field.file_name().unwrap_or("").to_string();
                let target = state.upload.plan(&original, typed_name.as_deref())?;
                let mut pending = PendingUpload::create(&target.dir, &target.base).await?;

                let mut written: u64 = 0;
                while let Some(chunk) = field.chunk().await.map_err(multipart_error)? {
                    written += chunk.len() as u64;
                    if written > state.max_upload_bytes {
                        return Err(DomainError::UploadTooLarge);
                    }
                    pending.write(&chunk).await?;
                }
                return pending.commit().await;
            }
            _ => {}
        }
    }
    Err(DomainError::UploadMissingFile)
}

/// Reads the optional name field with a small cap so it cannot be used to buffer a huge body.
async fn read_name(field: &mut axum::extract::multipart::Field<'_>) -> DomainResult<String> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field.chunk().await.map_err(multipart_error)? {
        if bytes.len() + chunk.len() > MAX_NAME_FIELD_BYTES {
            return Err(DomainError::UploadBadRequest {
                detail: "name field too long".into(),
            });
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|_| DomainError::UploadBadRequest {
        detail: "name field is not valid UTF-8".into(),
    })
}

fn multipart_error(e: MultipartError) -> DomainError {
    if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
        DomainError::UploadTooLarge
    } else {
        DomainError::UploadBadRequest {
            detail: e.body_text(),
        }
    }
}

fn status_for(e: &DomainError) -> StatusCode {
    match e {
        DomainError::UploadDisabled => StatusCode::SERVICE_UNAVAILABLE,
        DomainError::UploadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        DomainError::UploadMissingFile | DomainError::UploadBadRequest { .. } => {
            StatusCode::BAD_REQUEST
        }
        DomainError::UploadNameExhausted => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn error_response(e: DomainError) -> Response {
    (status_for(&e), Json(CommandError::from(e))).into_response()
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::path::{Path, PathBuf};

    use axum::body::Body;
    use axum::extract::connect_info::MockConnectInfo;
    use axum::http::{header, Request};
    use tower::ServiceExt;

    use super::*;
    use crate::application::upload_service::{UploadSettings, UploadSettingsHandle};
    use crate::infrastructure::http::routes::build_router;
    use crate::infrastructure::test_util::TempDir;

    struct Harness {
        router: Router,
        settings: UploadSettingsHandle,
        dir: PathBuf,
        _tmp: TempDir,
    }

    /// Router with a fixed clock (2026-03-07 09:05:01), a temp destination and a 16-byte limit.
    fn harness(enabled: bool) -> Harness {
        let tmp = TempDir::new();
        let dir = tmp.path().join("dest");
        let settings = UploadSettingsHandle::new(UploadSettings {
            enabled,
            dir_template: Some(dir.to_string_lossy().into_owned()),
        });
        let mut state = HttpState::for_test(settings.clone());
        state.max_upload_bytes = 16;
        let router = build_router(Arc::new(state))
            .layer(MockConnectInfo(SocketAddr::from(([192, 168, 1, 50], 5000))));
        Harness {
            router,
            settings,
            dir,
            _tmp: tmp,
        }
    }

    /// Hand-built multipart body: `(field name, file name, content)`.
    fn multipart(parts: &[(&str, Option<&str>, &[u8])]) -> (String, Vec<u8>) {
        let boundary = "XBOUNDARYX";
        let mut body = Vec::new();
        for (name, file, data) in parts {
            body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
            match file {
                Some(f) => body.extend_from_slice(
                    format!(
                        "Content-Disposition: form-data; name=\"{name}\"; filename=\"{f}\"\r\n\
                         Content-Type: application/octet-stream\r\n\r\n"
                    )
                    .as_bytes(),
                ),
                None => body.extend_from_slice(
                    format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
                ),
            }
            body.extend_from_slice(data);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        (format!("multipart/form-data; boundary={boundary}"), body)
    }

    async fn post_parts(
        h: &Harness,
        parts: &[(&str, Option<&str>, &[u8])],
    ) -> (StatusCode, serde_json::Value) {
        let (ct, body) = multipart(parts);
        let res = h
            .router
            .clone()
            .oneshot(
                Request::post("/upload")
                    .header(header::CONTENT_TYPE, ct)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    fn list(dir: &Path) -> Vec<String> {
        TempDir::list(dir)
    }

    #[tokio::test]
    async fn upload_with_original_name_succeeds() {
        let h = harness(true);
        let (status, json) = post_parts(
            &h,
            &[("name", None, b""), ("file", Some("photo.jpg"), b"abc")],
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(json["file_name"], "photo.jpg");
        assert_eq!(std::fs::read(h.dir.join("photo.jpg")).unwrap(), b"abc");
        assert_eq!(list(&h.dir), vec!["photo.jpg"]);
    }

    #[tokio::test]
    async fn upload_without_name_field_uses_original() {
        let h = harness(true);
        let (status, json) = post_parts(&h, &[("file", Some("a.txt"), b"x")]).await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(json["file_name"], "a.txt");
    }

    #[tokio::test]
    async fn upload_with_custom_expanded_name() {
        let h = harness(true);
        let (status, json) = post_parts(
            &h,
            &[
                ("name", None, b"stage-%hh%mm"),
                ("file", Some("a.jpg"), b"x"),
            ],
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(json["file_name"], "stage-0905.jpg");
        assert!(h.dir.join("stage-0905.jpg").is_file());
    }

    #[tokio::test]
    async fn traversal_name_stays_inside_destination() {
        let h = harness(true);
        let (status, _) = post_parts(
            &h,
            &[
                ("name", None, b"../../etc/passwd"),
                ("file", Some("a.txt"), b"x"),
            ],
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(list(&h.dir), vec!["passwd.txt"]);
        assert_eq!(list(h.dir.parent().unwrap()), vec!["dest"]);
    }

    #[tokio::test]
    async fn original_name_with_windows_path_is_reduced() {
        let h = harness(true);
        let (_, json) = post_parts(&h, &[("file", Some("C:\\Users\\x\\photo.jpg"), b"x")]).await;
        assert_eq!(json["file_name"], "photo.jpg");
    }

    #[tokio::test]
    async fn collision_gets_numbered_suffix() {
        let h = harness(true);
        post_parts(&h, &[("file", Some("photo.jpg"), b"one")]).await;
        let (status, json) = post_parts(&h, &[("file", Some("photo.jpg"), b"two")]).await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(json["file_name"], "photo (1).jpg");
        assert_eq!(std::fs::read(h.dir.join("photo.jpg")).unwrap(), b"one");
        assert_eq!(std::fs::read(h.dir.join("photo (1).jpg")).unwrap(), b"two");
    }

    #[tokio::test]
    async fn disabled_post_returns_503_and_touches_nothing() {
        let h = harness(false);
        let (status, json) = post_parts(&h, &[("file", Some("a.txt"), b"x")]).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(json["code"], "upload.disabled");
        assert!(!h.dir.exists());
    }

    #[tokio::test]
    async fn disabled_is_checked_before_reading_the_body() {
        let h = harness(false);
        // Not even a valid multipart request: the switch must answer first.
        let res = h
            .router
            .clone()
            .oneshot(
                Request::post("/upload")
                    .body(Body::from("garbage"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn live_toggle_applies_to_next_request() {
        let h = harness(false);
        let (status, _) = post_parts(&h, &[("file", Some("a.txt"), b"x")]).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        h.settings.set_enabled(true);
        let (status, _) = post_parts(&h, &[("file", Some("a.txt"), b"x")]).await;
        assert_eq!(status, StatusCode::CREATED);
    }

    #[tokio::test]
    async fn file_over_limit_returns_413_and_leaves_nothing() {
        let h = harness(true);
        let (status, json) = post_parts(&h, &[("file", Some("big.bin"), &[7u8; 17])]).await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(json["code"], "upload.too_large");
        assert!(list(&h.dir).is_empty());
    }

    #[tokio::test]
    async fn file_exactly_at_limit_is_accepted() {
        let h = harness(true);
        let (status, _) = post_parts(&h, &[("file", Some("ok.bin"), &[7u8; 16])]).await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(std::fs::read(h.dir.join("ok.bin")).unwrap().len(), 16);
    }

    #[tokio::test]
    async fn missing_file_returns_400_and_writes_nothing() {
        let h = harness(true);
        let (status, json) = post_parts(&h, &[("name", None, b"clip")]).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["code"], "upload.missing_file");
        assert!(!h.dir.exists());
    }

    #[tokio::test]
    async fn non_multipart_request_returns_400() {
        let h = harness(true);
        let res = h
            .router
            .clone()
            .oneshot(
                Request::post("/upload")
                    .body(Body::from("garbage"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert!(!h.dir.exists());
    }

    #[tokio::test]
    async fn oversized_name_field_returns_400() {
        let h = harness(true);
        let name = vec![b'a'; 5000];
        let (status, _) =
            post_parts(&h, &[("name", None, &name), ("file", Some("a.txt"), b"x")]).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(!h.dir.exists());
    }

    #[tokio::test]
    async fn page_is_served_without_overlays_dir() {
        let h = harness(true);
        let res = h
            .router
            .clone()
            .oneshot(Request::get("/upload").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let ct = res.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .to_string();
        assert!(ct.starts_with("text/html"));
    }

    #[tokio::test]
    async fn page_is_served_when_disabled() {
        let h = harness(false);
        let res = h
            .router
            .clone()
            .oneshot(Request::get("/upload").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn status_reports_live_enabled_flag() {
        let h = harness(false);
        let get_status = || async {
            let res = h
                .router
                .clone()
                .oneshot(Request::get("/upload/status").body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK);
            let bytes = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()
        };
        assert_eq!(get_status().await, serde_json::json!({ "enabled": false }));
        h.settings.set_enabled(true);
        assert_eq!(get_status().await, serde_json::json!({ "enabled": true }));
    }
}
