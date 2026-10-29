//! Expiry and real Keycloak logout must affect the very next MCP request.
use super::{
    isolation::inventory,
    issuer::{Issuer, Tokens},
};
use crate::support::client::Mcp;
use anyhow::{Result, ensure};
use serde_json::json;

pub async fn run(issuer: &Issuer, mcp: &Mcp, alice: &Tokens, bob: &Tokens) -> Result<()> {
    let wrong_audience = issuer.token("alice", "wrong-audience", true).await?;
    let missing_scope = issuer.token("alice", "postern-test", false).await?;
    for invalid in [
        &alice.refresh_token,
        &wrong_audience.access_token,
        &missing_scope.access_token,
    ] {
        mcp.call(invalid, "tools/list", json!({}), 401).await?;
    }
    issuer
        .post(
            "logout",
            "postern-test",
            &[("refresh_token", &alice.refresh_token)],
        )
        .await?;
    let state = issuer
        .post(
            "token/introspect",
            "postern-test",
            &[("token", &alice.access_token)],
        )
        .await?;
    ensure!(state["active"] == false);
    mcp.call(
        &alice.access_token,
        "tools/call",
        json!({"name":"hosts","arguments":{}}),
        401,
    )
    .await?;
    mcp.hosts(&bob.access_token, inventory("bob")).await?;
    let expiring = issuer.token("bob", "short-lived", true).await?;
    mcp.hosts(&expiring.access_token, inventory("bob")).await?;
    tokio::time::sleep(std::time::Duration::from_secs(expiring.expires_in + 1)).await;
    mcp.call(&expiring.access_token, "tools/list", json!({}), 401)
        .await?;
    Ok(())
}
