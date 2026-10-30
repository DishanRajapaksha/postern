//! Real public route + genuine issuer + non-root SSH; always remove temporary grants.
use super::{
    admin::Admin,
    clients, environment,
    state::{ISSUER, ROOT},
};
use crate::support::client::Mcp;
use anyhow::{Context, Result, ensure};
use reqwest::Method;
use serde_json::{Value, json};

pub async fn run() -> Result<()> {
    let resource = environment::resource()?;
    let admin = Admin::new().await?;
    let scopes = admin.get("/client-scopes").await?;
    let scope = scopes
        .as_array()
        .context("scopes")?
        .iter()
        .find(|s| s["name"] == "postern:use")
        .context("scope")?["id"]
        .as_str()
        .context("scope id")?;
    let diagnostic = clients::diagnostic(&admin, "postern-execution-acceptance", scope).await?;
    let path = format!("{ROOT}/config/hosts.json");
    let original = std::fs::read(&path)?;
    let result = async {
        let mut inventory: Value = serde_json::from_slice(&original)?;
        inventory["hosts"][0]["allowed_subjects"].as_array_mut().context("grants")?.push(json!(diagnostic.subject));
        std::fs::write(&path,serde_json::to_vec(&inventory)?)?;
        let response = reqwest::Client::builder().no_proxy().build()?
            .post(format!("{ISSUER}/protocol/openid-connect/token"))
            .basic_auth(&diagnostic.client,Some(&diagnostic.secret))
            .form(&[("grant_type","client_credentials"),("scope","postern:use")]).send().await?;
        ensure!(response.status().is_success(),"diagnostic token issuance failed");
        let response: Value = response.json().await?;
        let token = response["access_token"].as_str().context("access token")?;
        let mcp = Mcp::public(&resource)?;
        let tools = mcp.call(token,"tools/list",json!({}),200).await?;
        ensure!(tools["result"]["tools"].as_array().context("tools")?.len()==5);
        let session = call(&mcp,token,"open_session",json!({"host_id":"gont"})).await?;
        let session_id = session["session_id"].as_str().context("session id")?;
        let result = call(&mcp,token,"exec",json!({"session_id":session_id,"command":"id -un; id -u; test ! -r /srv/services/postern/audit/audit.sqlite3"})).await?;
        ensure!(result["record"]["outcome"]=="completed" && result["record"]["exit_code"]==0);
        ensure!(result["stdout"].as_str().context("stdout")?.starts_with("postern-ssh\n"));
        ensure!(result["audit_degraded"]==false);
        let audit = call(&mcp,token,"audit",json!({})).await?;
        ensure!(audit["records"].as_array().context("audit")?.len()==1);
        call(&mcp,token,"close_session",json!({"session_id":session_id})).await?;
        println!("PASS: public TLS MCP, real issuer, five tools, non-root SSH and durable private audit");
        Ok::<_,anyhow::Error>(token.to_owned())
    }.await;
    let restored = std::fs::write(&path, original);
    let deleted = admin
        .call(
            Method::DELETE,
            &format!("/clients/{}", diagnostic.id),
            Value::Null,
        )
        .await;
    restored?;
    deleted?;
    let token = result?;
    Mcp::public(&resource)?
        .call(&token, "tools/list", json!({}), 401)
        .await?;
    println!("PASS: diagnostic grant/client removed and token immediately revoked");
    Ok(())
}

async fn call(mcp: &Mcp, token: &str, name: &str, args: Value) -> Result<Value> {
    let result = mcp
        .call(
            token,
            "tools/call",
            json!({"name":name,"arguments":args}),
            200,
        )
        .await?;
    ensure!(
        result["result"]["isError"] != true,
        "tool operation rejected"
    );
    Ok(serde_json::from_str(
        result["result"]["content"][0]["text"]
            .as_str()
            .context("tool result")?,
    )?)
}
