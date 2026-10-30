//! Exact private inventory assertions shared by issuer tests.
use super::client::Mcp;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
impl Mcp {
    pub async fn hosts(&self, token: &str, expected: Value) -> Result<()> {
        let result = self
            .call(
                token,
                "tools/call",
                json!({"name":"hosts","arguments":{}}),
                200,
            )
            .await?;
        ensure!(
            result.get("error").is_none() && result["result"]["isError"] != true,
            "hosts failed"
        );
        let text = result["result"]["content"][0]["text"]
            .as_str()
            .context("missing hosts text")?;
        ensure!(
            serde_json::from_str::<Value>(text)? == expected,
            "unexpected host inventory"
        );
        Ok(())
    }
}
