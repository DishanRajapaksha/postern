//! Decode a real MCP tool result without printing credentials.
use crate::support::client::Mcp;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};

pub(super) async fn call(mcp: &Mcp, token: &str, name: &str, args: Value) -> Result<Value> {
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
