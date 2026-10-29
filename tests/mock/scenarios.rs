//! Fresh HTTP and tool checks must both succeed without caching or identity changes.
use super::Mock;
use crate::support::client::Mcp;
use anyhow::{Result, ensure};
use reqwest::Method;
use serde_json::json;

pub async fn run(mcp: &Mcp, issuer: &Mock) -> Result<()> {
    mcp.request(None, Method::POST, "initialize", json!({}), 401, &[])
        .await?;
    for token in [
        "inactive",
        "expired",
        "wrong-aud",
        "wrong-iss",
        "wrong-scope",
    ] {
        mcp.call(token, "tools/list", json!({}), 401).await?;
    }
    for token in ["outage", "redirect", "malformed", "oversized"] {
        mcp.call(token, "tools/list", json!({}), 503).await?;
    }
    issuer.assert_no_redirects()?;
    for method in [Method::GET, Method::DELETE] {
        mcp.request(None, method, "tools/list", json!({}), 401, &[])
            .await?;
    }
    let init = mcp
        .call(
            "alice",
            "initialize",
            json!({"protocolVersion":"2025-11-25", "capabilities":{},
        "clientInfo":{"name":"rust-mock-integration","version":"1"}}),
            200,
        )
        .await?;
    ensure!(init["result"].get("capabilities").is_some());
    let tools = mcp.call("bob", "tools/list", json!({}), 200).await?;
    ensure!(
        tools["result"]["tools"]
            .as_array()
            .is_some_and(|v| v.len() == 1 && v[0]["name"] == "hosts")
    );
    let before = issuer.count("alice")?;
    mcp.hosts(
        "alice",
        json!({"hosts":[{"id":"lab", "description":"Disposable test host (illustrative only)"}]}),
    )
    .await?;
    ensure!(
        issuer.count("alice")? == before + 2,
        "tool must introspect twice"
    );
    mcp.hosts("bob", json!({"hosts":[]})).await?;
    mcp.call(
        "inactive",
        "tools/call",
        json!({"name":"hosts","arguments":{}}),
        401,
    )
    .await?;
    for token in ["revoked-during-call", "switching-subject"] {
        let result = mcp
            .call(
                token,
                "tools/call",
                json!({"name":"hosts","arguments":{}}),
                200,
            )
            .await?;
        ensure!(result.get("error").is_some() || result["result"]["isError"] == true);
        ensure!(issuer.count(token)? == 2);
    }
    Ok(())
}
