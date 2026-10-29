//! Treat the Authorization header as untrusted text, including duplicate headers.
use super::AuthError;
use axum::http::{HeaderMap, header::AUTHORIZATION};

pub(super) fn read(headers: &HeaderMap) -> Result<&str, AuthError> {
    let mut values = headers.get_all(AUTHORIZATION).iter();
    let first = values.next().ok_or(AuthError::Unauthorised)?;
    if values.next().is_some() {
        return Err(AuthError::Unauthorised);
    }
    let text = first.to_str().map_err(|_| AuthError::Unauthorised)?;
    let (scheme, token) = text.split_once(' ').ok_or(AuthError::Unauthorised)?;
    // OAuth schemes are case insensitive; token bytes are not.
    if !scheme.eq_ignore_ascii_case("Bearer")
        || token.is_empty()
        || token.len() > 8192
        || !token
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-._~+/=".contains(&c))
    {
        return Err(AuthError::Unauthorised);
    }
    Ok(token)
}
