//! Validate the operator's intended resource before changing the live realm.
use crate::support::client::Mcp;
use anyhow::{Context, Result, ensure};

pub fn resource() -> Result<String> {
    let resource = std::env::var("POSTERN_OAUTH_RESOURCE")
        .context("set POSTERN_OAUTH_RESOURCE to the intended HTTPS MCP URL")?;
    let url = reqwest::Url::parse(&resource)?;
    ensure!(
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.path().len() > 1
            && !url.path().ends_with('/')
            && url.query().is_none()
            && url.fragment().is_none()
            && url.as_str() == resource,
        "OAuth resource must be a canonical HTTPS resource URL with a nonempty endpoint path"
    );
    Ok(resource)
}

pub fn mcp() -> Result<Mcp> {
    Mcp::for_resource(&resource()?)
}
