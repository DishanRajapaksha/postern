//! Create dedicated, labelled clients without editing any existing application or realm flow.
use super::{admin::Admin, state::Diagnostic};
use anyhow::{Context, Result, ensure};
use reqwest::Method;
use serde_json::{Value, json};

pub async fn create(admin: &Admin, name: &str, diagnostic: bool) -> Result<String> {
    let existing = admin.get(&format!("/clients?clientId={name}")).await?;
    ensure!(
        existing.as_array().is_some_and(Vec::is_empty),
        "Postern client already exists; inspect before provisioning"
    );
    admin.call(Method::POST, "/clients", json!({"clientId":name, "enabled":true,
        "protocol":"openid-connect", "publicClient":false, "fullScopeAllowed":false, "standardFlowEnabled":false,
        "directAccessGrantsEnabled":false, "serviceAccountsEnabled":diagnostic,
        "attributes":{"postern.managed":"inventory-preview","access.token.lifespan":"120"}, "protocolMappers":[
            {"name":"postern-subject", "protocol":"openid-connect", "protocolMapper":"oidc-sub-mapper",
             "config":{"access.token.claim":"true","introspection.token.claim":"true"}},
            {"name":"postern-audience", "protocol":"openid-connect", "protocolMapper":"oidc-audience-mapper",
             "config":{"included.custom.audience":"postern",
                "access.token.claim":"true","introspection.token.claim":"true"}}, introspection_audience()]
    })).await?;
    let created = admin.get(&format!("/clients?clientId={name}")).await?;
    Ok(created[0]["id"]
        .as_str()
        .context("missing created client ID")?
        .into())
}

pub async fn secret(admin: &Admin, id: &str) -> Result<String> {
    let response = admin.get(&format!("/clients/{id}/client-secret")).await?;
    Ok(response["value"]
        .as_str()
        .context("missing client secret")?
        .into())
}

pub(super) fn introspection_audience() -> Value {
    // A single Keycloak audience mapper selects one value; two audiences need two mappers.
    json!({"name":"postern-introspection-audience","protocol":"openid-connect",
        "protocolMapper":"oidc-audience-mapper","config":{
            "included.client.audience":"postern-resource","access.token.claim":"true",
            "introspection.token.claim":"true","id.token.claim":"false"}})
}

pub async fn diagnostic(admin: &Admin, name: &str, scope: &str) -> Result<Diagnostic> {
    let id = create(admin, name, true).await?;
    admin
        .call(
            Method::PUT,
            &format!("/clients/{id}/optional-client-scopes/{scope}"),
            Value::Null,
        )
        .await?;
    let account = admin
        .get(&format!("/clients/{id}/service-account-user"))
        .await?;
    Ok(Diagnostic {
        secret: secret(admin, &id).await?,
        id,
        client: name.into(),
        subject: account["id"]
            .as_str()
            .context("missing diagnostic subject")?
            .into(),
    })
}
