//! Report acceptance predicates only; keep token values and credentials on Gont.
use super::{
    admin::Admin,
    clients,
    state::{ISSUER, ROOT},
};
use anyhow::{Context, Result, ensure};
use reqwest::{Client, Method};
use serde_json::Value;

pub async fn run() -> Result<()> {
    let admin = Admin::new().await?;
    let scopes = admin.get("/client-scopes").await?;
    let scope = scopes
        .as_array()
        .context("invalid scopes")?
        .iter()
        .find(|s| s["name"] == "postern:use")
        .context("missing scope")?["id"]
        .as_str()
        .context("missing scope ID")?;
    let diagnostic = clients::diagnostic(&admin, "postern-diagnostic-debug", scope).await?;
    let result = inspect(&diagnostic.client, &diagnostic.secret).await;
    admin
        .call(
            Method::DELETE,
            &format!("/clients/{}", diagnostic.id),
            Value::Null,
        )
        .await?;
    result
}

async fn inspect(client_id: &str, issuer_secret: &str) -> Result<()> {
    let client = Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let token = client
        .post(format!("{ISSUER}/protocol/openid-connect/token"))
        .basic_auth(client_id, Some(issuer_secret))
        .form(&[
            ("grant_type", "client_credentials"),
            ("scope", "postern:use"),
        ])
        .send()
        .await?;
    ensure!(token.status().is_success(), "token issuance failed");
    let token: Value = token.json().await?;
    let environment = std::fs::read_to_string(format!("{ROOT}/private/environment"))?;
    let secret = environment
        .lines()
        .find_map(|line| line.strip_prefix("POSTERN_KEYCLOAK_CLIENT_SECRET="))
        .context("missing resource credential")?;
    let response = client
        .post(format!("{ISSUER}/protocol/openid-connect/token/introspect"))
        .basic_auth("postern-resource", Some(secret))
        .form(&[(
            "token",
            token["access_token"].as_str().context("missing token")?,
        )])
        .send()
        .await?;
    ensure!(
        response.status().is_success(),
        "introspection rejected the resource client"
    );
    let claims: Value = response.json().await?;
    println!("Resource-client introspection:");
    super::claim_report::print(&claims)?;
    let response = client
        .post(format!("{ISSUER}/protocol/openid-connect/token/introspect"))
        .basic_auth(client_id, Some(issuer_secret))
        .form(&[(
            "token",
            token["access_token"].as_str().context("missing token")?,
        )])
        .send()
        .await?;
    ensure!(
        response.status().is_success(),
        "issuing-client introspection failed"
    );
    println!("Issuing-client introspection:");
    super::claim_report::print(&response.json().await?)?;
    Ok(())
}
