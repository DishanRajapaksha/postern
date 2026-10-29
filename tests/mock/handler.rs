//! Change claims by token name; these values are public fixture inputs only.
use super::State;
use axum::{
    Form,
    extract::State as Extract,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub async fn redirected(Extract(state): Extract<Arc<State>>) -> StatusCode {
    if let Ok(mut counts) = state.counts.lock() {
        *counts.entry("redirect-followed".into()).or_default() += 1;
    }
    StatusCode::INTERNAL_SERVER_ERROR
}

pub async fn introspect(
    Extract(state): Extract<Arc<State>>,
    headers: HeaderMap,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Response, StatusCode> {
    let expected = "Basic cG9zdGVybi10ZXN0OmZpeHR1cmUtc2VjcmV0";
    if headers.get("authorization").is_none_or(|v| v != expected) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let token = form.get("token").map(String::as_str).unwrap_or("");
    let count = {
        let mut counts = state
            .counts
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let count = counts.entry(token.into()).or_default();
        *count += 1;
        *count
    };
    if token == "outage" {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    if token == "redirect" {
        let location = format!("{}/redirect", state.issuer.trim_end_matches("/realms/test"));
        return Ok((StatusCode::FOUND, [("location", location)]).into_response());
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .as_secs();
    let active = matches!(
        token,
        "alice"
            | "bob"
            | "expired"
            | "wrong-aud"
            | "wrong-iss"
            | "wrong-scope"
            | "revoked-during-call"
            | "switching-subject"
            | "oversized"
    ) && !(token == "revoked-during-call" && count > 1);
    let mut claims = json!({"active":active, "token_type":"Bearer",
        "sub":if token == "bob" || (token == "switching-subject" && count > 1) { "other-user" }
            else { "00000000-0000-0000-0000-000000000001" },
        "iss":if token == "wrong-iss" { "https://malicious.example/realms/test" } else { &state.issuer },
        "aud":[if token == "wrong-aud" { "another-service" } else { "postern" }],
        "scope":if token == "wrong-scope" { "other" } else { "postern:use" },
        "exp":if token == "expired" { now - 10 } else { now + 600 }, "nbf":now - 10});
    if token == "oversized" {
        claims["padding"] = json!("x".repeat(16384));
    }
    let data = if token == "malformed" {
        "not-json".into()
    } else {
        claims.to_string()
    };
    Ok(([("content-type", "application/json")], data).into_response())
}
