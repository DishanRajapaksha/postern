//! Capacity must recover after overload or cancellation, without waiting in a queue.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::{Limits, protect};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::post,
};
use std::sync::Arc;
use tower::ServiceExt;

fn request() -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("host", "127.0.0.1:8765")
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn overload_is_rejected_and_released_capacity_is_reusable() {
    let limits = Arc::new(Limits::default());
    let app = Router::new()
        .route("/mcp", post(|| async { "ok" }))
        .layer(middleware::from_fn_with_state(Arc::clone(&limits), protect));
    let held = limits.slots.acquire_many(32).await.unwrap();
    let response = app.clone().oneshot(request()).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    drop(held);
    assert_eq!(
        app.oneshot(request()).await.unwrap().status(),
        StatusCode::OK
    );
    assert_eq!(limits.slots.available_permits(), 32);
}

#[tokio::test(start_paused = true)]
async fn deadline_cancels_work_and_releases_its_capacity() {
    let limits = Arc::new(Limits::default());
    let app = Router::new()
        .route(
            "/mcp",
            post(|| async {
                // Simulate a handler that never responds. Tokio advances the test clock.
                std::future::pending::<()>().await;
                "unreachable"
            }),
        )
        .layer(middleware::from_fn_with_state(Arc::clone(&limits), protect));
    let response = app.oneshot(request()).await.unwrap();
    assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
    assert_eq!(limits.slots.available_permits(), 32);
    assert_eq!(response.headers()["cache-control"], "no-store");
}
