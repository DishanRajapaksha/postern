//! Discovery requires current credentials and exposes only the implemented hosts tool.
use super::ISSUER;
use crate::support::client::Mcp;
use anyhow::{Result, ensure};
use reqwest::Method;
use serde_json::json;

pub async fn run(mcp: &Mcp, alice: &str, bob: &str) -> Result<()> {
    crate::support::discovery::check(ISSUER).await?;
    for verb in [Method::POST, Method::GET, Method::DELETE] {
        mcp.request(None, verb.clone(), "tools/list", json!({}), 401, &[])
            .await?;
        mcp.request(
            Some("invalid-token"),
            verb,
            "tools/list",
            json!({}),
            401,
            &[],
        )
        .await?;
    }
    let client = reqwest::Client::builder().no_proxy().build()?;
    for path in [
        "/.well-known/oauth-protected-resource",
        "/.well-known/oauth-protected-resource/mcp",
    ] {
        let result = client
            .get(format!("http://127.0.0.1:8765{path}"))
            .send()
            .await?;
        ensure!(result.status() == 200 && result.headers()["cache-control"] == "no-store");
        let metadata: serde_json::Value = result.json().await?;
        ensure!(metadata["authorization_servers"][0] == ISSUER);
        ensure!(metadata["resource"] == "https://postern.example/mcp");
    }
    let duplicate = format!("Bearer {bob}");
    mcp.request(
        Some(alice),
        Method::POST,
        "tools/list",
        json!({}),
        401,
        &[("authorization", &duplicate)],
    )
    .await?;
    let init = mcp
        .call(
            alice,
            "initialize",
            json!({"protocolVersion":"2025-11-25", "capabilities":{},
        "clientInfo":{"name":"rust-docker-integration", "version":"1"}}),
            200,
        )
        .await?;
    ensure!(init["result"]["capabilities"].get("tools").is_some());
    let list = mcp.call(bob, "tools/list", json!({}), 200).await?;
    ensure!(
        list["result"]["tools"]
            .as_array()
            .is_some_and(|v| v.len() == 1 && v[0]["name"] == "hosts")
    );
    for name in ["exec", "open_session", "close_session", "audit"] {
        let result = mcp
            .call(
                alice,
                "tools/call",
                json!({"name":name, "arguments":{}}),
                200,
            )
            .await?;
        ensure!(result.get("error").is_some() || result["result"]["isError"] == true);
    }
    Ok(())
}
