//! Exercise boundary decisions without needing a token or a running issuer.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    middleware,
};
use tower::ServiceExt;

#[tokio::test]
async fn rejects_foreign_origins_hosts_and_large_chunked_bodies() {
    let app = Router::new()
        .route("/mcp", axum::routing::post(|| async { "ok" }))
        .layer(middleware::from_fn_with_state(
            std::sync::Arc::new(super::Limits::default()),
            super::protect,
        ));
    for (host, origin, size, expected) in [
        ("127.0.0.1:8765", None, 0, StatusCode::OK),
        (
            "localhost:8765",
            Some("http://localhost:8765"),
            0,
            StatusCode::OK,
        ),
        ("evil.example:8765", None, 0, StatusCode::FORBIDDEN),
        (
            "127.0.0.1:8765",
            Some("https://evil.example"),
            0,
            StatusCode::FORBIDDEN,
        ),
        ("127.0.0.1:8765", Some("null"), 0, StatusCode::FORBIDDEN),
        ("127.0.0.1:8765", None, 16385, StatusCode::PAYLOAD_TOO_LARGE),
    ] {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("host", host);
        if let Some(value) = origin {
            builder = builder.header("origin", value);
        }
        let request = builder.body(Body::from(vec![b'x'; size])).unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), expected);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
}

#[test]
fn duplicate_headers_cannot_choose_a_more_permissive_origin() {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("host", "localhost:8765".parse().unwrap());
    headers.insert("origin", "http://localhost:8765".parse().unwrap());
    headers.append("origin", "https://evil.example".parse().unwrap());
    assert!(!super::origin::allowed(&headers));
    headers.remove("origin");
    headers.append("host", "localhost:8765".parse().unwrap());
    assert!(!super::origin::allowed(&headers));
}

#[test]
fn public_origin_must_match_explicit_configuration() {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("host", "leyline.example".parse().unwrap());
    headers.insert("origin", "https://leyline.example".parse().unwrap());
    let public = Some("https://leyline.example");
    assert!(super::origin::allowed_public(&headers, public));
    assert!(!super::origin::allowed_public(&headers, None));
    headers.insert("origin", "https://attacker.example".parse().unwrap());
    headers.insert("x-forwarded-host", "leyline.example".parse().unwrap());
    assert!(!super::origin::allowed_public(&headers, public));
}
