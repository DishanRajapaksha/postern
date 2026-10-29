//! Provision a fail-closed runtime and separate short-lived diagnostic accounts.
use super::{
    admin::Admin,
    clients, files,
    state::{ISSUER, State},
};
use anyhow::{Context, Result, ensure};
use reqwest::Method;
use serde_json::json;

pub async fn run() -> Result<()> {
    let admin = Admin::new().await?;
    let realm = admin.get("").await?;
    let browser_flow = realm["browserFlow"]
        .as_str()
        .context("missing browser flow")?
        .to_owned();
    ensure!(
        browser_flow == "gont-passkeys",
        "unexpected owner authentication policy"
    );
    let users = admin.get("/users?username=dish&exact=true").await?;
    ensure!(users.as_array().is_some_and(|v| v.len() == 1) && users[0]["enabled"] == true);
    let owner = users[0]["id"]
        .as_str()
        .context("missing owner subject")?
        .to_owned();
    let scopes = admin.get("/client-scopes").await?;
    let mut scope = scopes
        .as_array()
        .context("invalid scope list")?
        .iter()
        .find(|v| v["name"] == "postern:use")
        .cloned();
    if scope.is_none() {
        admin
            .call(
                Method::POST,
                "/client-scopes",
                json!({"name":"postern:use", "protocol":"openid-connect",
            "attributes":{"include.in.token.scope":"true"}}),
            )
            .await?;
        let scopes = admin.get("/client-scopes").await?;
        scope = scopes
            .as_array()
            .context("invalid scope list")?
            .iter()
            .find(|v| v["name"] == "postern:use")
            .cloned();
    }
    let scope = scope.context("missing Postern scope")?;
    let scope_id = scope["id"].as_str().context("missing scope ID")?;
    let resource = clients::create(&admin, "postern-resource", false).await?;
    let secret = clients::secret(&admin, &resource).await?;
    files::private("environment", format!("POSTERN_CONFIG=/srv/services/postern/config/hosts.json\nPOSTERN_KEYCLOAK_ISSUER={ISSUER}\nPOSTERN_KEYCLOAK_AUDIENCE=postern\nPOSTERN_KEYCLOAK_SCOPE=postern:use\nPOSTERN_KEYCLOAK_CLIENT_ID=postern-resource\nPOSTERN_KEYCLOAK_CLIENT_SECRET={secret}\n").as_bytes())?;
    let mut state = State {
        owner,
        browser_flow,
        diagnostics: vec![],
    };
    files::private("verification-state.json", &serde_json::to_vec(&state)?)?;
    for suffix in ["alpha", "beta", "denied"] {
        state.diagnostics.push(
            clients::diagnostic(&admin, &format!("postern-diagnostic-{suffix}"), scope_id).await?,
        );
        files::private("verification-state.json", &serde_json::to_vec(&state)?)?;
    }
    files::inventory(&state.owner, None)?;
    println!(
        "Configured dedicated Postern resource, owner-only inventory and temporary diagnostics"
    );
    Ok(())
}

pub async fn diagnostics() -> Result<()> {
    use super::state::ROOT;
    let admin = Admin::new().await?;
    super::resource::harden(&admin).await?;
    let path = format!("{ROOT}/private/verification-state.json");
    let mut state: State = serde_json::from_slice(&std::fs::read(path)?)?;
    let scopes = admin.get("/client-scopes").await?;
    let scope = scopes
        .as_array()
        .context("invalid scope list")?
        .iter()
        .find(|v| v["name"] == "postern:use")
        .context("missing Postern scope")?["id"]
        .as_str()
        .context("missing scope ID")?;
    state.diagnostics.clear();
    for suffix in ["alpha", "beta", "denied"] {
        state.diagnostics.push(
            clients::diagnostic(&admin, &format!("postern-diagnostic-{suffix}"), scope).await?,
        );
        files::private("verification-state.json", &serde_json::to_vec(&state)?)?;
    }
    Ok(())
}
