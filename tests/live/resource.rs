//! Reconcile only our resource client, preserving credentials and unrelated settings.
use super::admin::Admin;
use anyhow::{Context, Result, ensure};
use reqwest::Method;
use serde_json::json;

pub(super) async fn harden(admin: &Admin) -> Result<()> {
    let found = admin.get("/clients?clientId=postern-resource").await?;
    let id = found[0]["id"].as_str().context("missing resource client")?;
    let path = format!("/clients/{id}");
    let mut resource = admin.get(&path).await?;
    // A readback may redact credentials; never send that field back as a new secret.
    resource
        .as_object_mut()
        .context("invalid client representation")?
        .remove("secret");
    ensure!(
        resource["attributes"]["postern.managed"] == "inventory-preview",
        "unmanaged resource client"
    );
    resource["fullScopeAllowed"] = json!(false);
    let mappers = resource["protocolMappers"]
        .as_array_mut()
        .context("missing resource mappers")?;
    let audience = mappers
        .iter_mut()
        .find(|v| v["name"] == "postern-audience")
        .context("missing audience mapper")?;
    audience["config"]
        .as_object_mut()
        .context("invalid audience mapper")?
        .remove("included.client.audience");
    if !mappers
        .iter()
        .any(|v| v["name"] == "postern-introspection-audience")
    {
        mappers.push(super::clients::introspection_audience());
    }
    admin.call(Method::PUT, &path, resource).await?;
    Ok(())
}
