//! Public static metadata still obeys the HTTP boundary and never consults Keycloak.
use anyhow::{Result, ensure};
use reqwest::Method;
use serde_json::json;

pub async fn check(issuer: &str) -> Result<()> {
    let client = reqwest::Client::builder().no_proxy().build()?;
    for path in [
        "/.well-known/oauth-protected-resource",
        "/.well-known/oauth-protected-resource/mcp",
    ] {
        let url = format!("http://127.0.0.1:8765{path}");
        let response = client
            .get(&url)
            .header("authorization", "Bearer outage")
            .header("x-forwarded-host", "attacker.example")
            .header("x-forwarded-proto", "http")
            .send()
            .await?;
        ensure!(response.status() == 200);
        ensure!(response.headers()["cache-control"] == "no-store");
        ensure!(response.headers()["x-content-type-options"] == "nosniff");
        ensure!(response.headers()["content-type"] == "application/json");
        ensure!(response.headers().get("www-authenticate").is_none());
        ensure!(
            response.json::<serde_json::Value>().await?
                == json!({
            "resource":"https://postern.example/mcp", "authorization_servers":[issuer],
            "scopes_supported":["postern:use"], "bearer_methods_supported":["header"]})
        );
        for (header, value) in [
            ("host", "attacker.example"),
            ("origin", "https://attacker.example"),
        ] {
            ensure!(
                client
                    .get(&url)
                    .header(header, value)
                    .send()
                    .await?
                    .status()
                    == 403
            );
        }
        ensure!(
            client
                .get(&url)
                .body(vec![b'x'; 16385])
                .send()
                .await?
                .status()
                == 413
        );
        for method in [Method::POST, Method::DELETE] {
            ensure!(client.request(method, &url).send().await?.status() == 405);
        }
    }
    Ok(())
}
