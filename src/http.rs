//! Protect the local HTTP boundary before spending work on authentication.
#[cfg(test)]
mod limit_tests;
mod origin;
#[cfg(test)]
mod tests;

use axum::{
    body::{Body, to_bytes},
    extract::State,
    http::{HeaderValue, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::{sync::Arc, time::Duration};
use tokio::{sync::Semaphore, time::timeout};

const MAX_BODY: usize = 16 * 1024;
/// One shared budget for the server. Requests cannot create extra capacity.
pub(crate) struct Limits {
    slots: Semaphore,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            slots: Semaphore::new(32),
        }
    }
}

pub(crate) async fn protect(
    State(limits): State<Arc<Limits>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let mut response =
        match timeout(Duration::from_secs(15), dispatch(request, next, &limits)).await {
            Ok(response) => response,
            Err(_) => StatusCode::REQUEST_TIMEOUT.into_response(),
        };
    // Neither browsers nor intermediaries should store private host inventories.
    response
        .headers_mut()
        .insert("cache-control", HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
}

async fn dispatch(request: Request<Body>, next: Next, limits: &Limits) -> Response {
    if !origin::allowed(request.headers()) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Ok(_permit) = limits.slots.try_acquire() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let (parts, body) = request.into_parts();
    // Read under both a byte limit and the outer deadline, including chunked bodies.
    let Ok(bytes) = to_bytes(body, MAX_BODY).await else {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    };
    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}
