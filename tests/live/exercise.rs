//! Exercise the installed systemd service with credentials issued by the actual realm.
use super::state::{ISSUER, State};
use crate::support::client::Mcp;
use anyhow::{Context, Result};
use reqwest::{Client, Method};
use serde_json::{Value, json};

pub async fn run(state: &State) -> Result<Vec<String>> {
    let client = Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let mut tokens = vec![];
    for diagnostic in &state.diagnostics {
        let response = client
            .post(format!("{ISSUER}/protocol/openid-connect/token"))
            .basic_auth(&diagnostic.client, Some(&diagnostic.secret))
            .form(&[
                ("grant_type", "client_credentials"),
                ("scope", "postern:use"),
            ])
            .send()
            .await?;
        anyhow::ensure!(
            response.status().is_success(),
            "diagnostic token issuance failed: {}",
            response.status()
        );
        let response: Value = response.json().await?;
        tokens.push(
            response["access_token"]
                .as_str()
                .context("missing diagnostic access token")?
                .to_owned(),
        );
    }
    let mcp = Mcp::new()?;
    mcp.request(None, Method::POST, "tools/list", json!({}), 401, &[])
        .await?;
    mcp.call("invalid-token", "tools/list", json!({}), 401)
        .await?;
    mcp.request(
        Some(&tokens[0]),
        Method::POST,
        "tools/list",
        json!({}),
        403,
        &[("origin", "https://attacker.example")],
    )
    .await?;
    mcp.request(
        Some(&tokens[0]),
        Method::POST,
        "tools/list",
        json!({}),
        403,
        &[("host", "attacker.example:8765")],
    )
    .await?;
    mcp.call(
        &tokens[0],
        "tools/list",
        json!({"padding":"x".repeat(16385)}),
        413,
    )
    .await?;
    let init = mcp
        .call(
            &tokens[0],
            "initialize",
            json!({"protocolVersion":"2025-11-25","capabilities":{},
        "clientInfo":{"name":"gont-native-acceptance","version":"1"}}),
            200,
        )
        .await?;
    anyhow::ensure!(init["result"].get("capabilities").is_some());
    let tools = mcp.call(&tokens[0], "tools/list", json!({}), 200).await?;
    anyhow::ensure!(
        tools["result"]["tools"]
            .as_array()
            .is_some_and(|v| v.len() == 1 && v[0]["name"] == "hosts")
    );
    super::host_probe::run(state, &tokens).await?;
    Ok(tokens)
}
