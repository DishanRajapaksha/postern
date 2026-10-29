//! Test final owner isolation, then private/shared/denied diagnostic grants.
use super::{files, state::State};
use crate::support::client::Mcp;
use anyhow::Result;
use serde_json::json;

pub async fn run(state: &State, tokens: &[String]) -> Result<()> {
    let mcp = Mcp::new()?;
    // Diagnostic credentials cannot inherit the owner's only host grant.
    mcp.hosts(&tokens[0], json!({"hosts":[]})).await?;
    files::inventory(
        &state.owner,
        Some((&state.diagnostics[0].subject, &state.diagnostics[1].subject)),
    )?;
    files::restart()?;
    // The old pool contains connections to the stopped process. Use a new client.
    let mcp = Mcp::new()?;
    let shared = json!({"id":"diagnostic-shared","description":"Temporary shared test"});
    tokio::try_join!(
        mcp.hosts(&tokens[0], json!({"hosts":[{"id":"diagnostic-private","description":"Temporary private test"},shared]})),
        mcp.hosts(&tokens[1], json!({"hosts":[shared]})),
        mcp.hosts(&tokens[2], json!({"hosts":[]})))?;
    Ok(())
}
