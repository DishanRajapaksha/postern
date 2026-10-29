//! The HTTP gate runs before MCP parses or dispatches any message.
use super::{AuthError, Authenticator};
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

pub(crate) async fn require_auth(
    State(auth): State<Arc<Authenticator>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    match auth.authenticate(request.headers()).await {
        Ok(principal) => {
            // Carry our checked identity internally; never take it from JSON arguments.
            request.extensions_mut().insert(principal);
            next.run(request).await
        }
        Err(AuthError::Unauthorised) => {
            (StatusCode::UNAUTHORIZED, [("www-authenticate", "Bearer")]).into_response()
        }
        Err(AuthError::KeycloakUnavailable) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
