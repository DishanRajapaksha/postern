//! Check privacy headers on success and failure before decoding a reply.
use anyhow::{Result, ensure};
use serde_json::Value;

pub async fn check(
    response: reqwest::Response,
    status: u16,
    method: &str,
    challenge: &str,
) -> Result<Value> {
    ensure!(
        response.status().as_u16() == status,
        "{method}: expected {status}, received {}; content-type={:?}, cf-mitigated={:?}, content-length={:?}",
        response.status(),
        response.headers().get("content-type"),
        response.headers().get("cf-mitigated"),
        response.headers().get("content-length")
    );
    ensure!(
        response.headers().get("mcp-session-id").is_none(),
        "unexpected stateful session"
    );
    ensure!(
        response
            .headers()
            .get("cache-control")
            .is_some_and(|v| v == "no-store")
    );
    ensure!(
        response
            .headers()
            .get("x-content-type-options")
            .is_some_and(|v| v == "nosniff")
    );
    if status == 401 {
        ensure!(
            response
                .headers()
                .get("www-authenticate")
                .is_some_and(|v| v == challenge)
        );
    }
    if status != 200 {
        return Ok(Value::Null);
    }
    let value: Value = response.json().await?;
    ensure!(value["jsonrpc"] == "2.0" && value["id"] == 1);
    Ok(value)
}
