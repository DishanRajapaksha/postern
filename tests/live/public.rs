//! Pre-register one narrowly configured client; no realm policy or anonymous DCR changes.
use super::{admin::Admin, environment};
use anyhow::{Context, Result, ensure};
use reqwest::Method;
use serde_json::{Value, json};

pub async fn configure() -> Result<()> {
    let resource = environment::resource()?;
    let admin = Admin::new().await?;
    let realm = admin.get("").await?;
    ensure!(
        realm["browserFlow"] == "gont-passkeys",
        "unexpected browser policy"
    );
    let mut client: Value = serde_json::from_str(include_str!(
        "../../deploy/gont/chatgpt-client.example.json"
    ))?;
    if let Ok(callback) = std::env::var("POSTERN_CHATGPT_CALLBACK") {
        let url = reqwest::Url::parse(&callback)?;
        ensure!(
            url.scheme() == "https"
                && url.host_str() == Some("chatgpt.com")
                && url.username().is_empty()
                && url.password().is_none()
                && url.fragment().is_none()
                && !callback.contains('*'),
            "invalid ChatGPT callback"
        );
        client["redirectUris"] = json!([callback]);
        client["enabled"] = json!(true);
    } else {
        client["redirectUris"] = json!([]);
    }
    client["protocolMappers"][0]["config"]["included.custom.audience"] = json!(resource);
    let existing = admin.get("/clients?clientId=postern-chatgpt").await?;
    if let Some(id) = existing[0]["id"].as_str() {
        let current = admin.get(&format!("/clients/{id}")).await?;
        ensure!(
            current["attributes"]["postern.managed"] == "public-ssh",
            "unmanaged login client"
        );
        // A check without a callback must never disable an accepted existing client.
        ensure!(
            std::env::var("POSTERN_CHATGPT_CALLBACK").is_ok(),
            "callback required to update existing client"
        );
        admin
            .call(Method::PUT, &format!("/clients/{id}"), client)
            .await?;
    } else {
        admin.call(Method::POST, "/clients", client).await?;
    }
    let created = admin.get("/clients?clientId=postern-chatgpt").await?;
    let id = created[0]["id"].as_str().context("missing login client")?;
    let scopes = admin.get("/client-scopes").await?;
    for name in ["basic", "postern:use"] {
        let scope = scopes
            .as_array()
            .context("scopes")?
            .iter()
            .find(|s| s["name"] == name)
            .context("missing scope")?;
        admin
            .call(
                Method::PUT,
                &format!(
                    "/clients/{id}/default-client-scopes/{}",
                    scope["id"].as_str().context("scope id")?
                ),
                Value::Null,
            )
            .await?;
    }
    println!(
        "Configured dedicated Postern login client; enabled={}",
        created[0]["enabled"]
    );
    Ok(())
}
