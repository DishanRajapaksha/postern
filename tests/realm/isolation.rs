//! Exact response comparisons catch leaked grants as well as cross-user hosts.
use crate::support::client::Mcp;
use anyhow::Result;
use reqwest::Method;
use serde_json::{Value, json};

pub fn inventory(user: &str) -> Value {
    json!({"hosts":[{"id":user,"description":format!("{} only", if user == "alice" { "Alice" } else { "Bob" })},
        {"id":"shared","description":"Shared"}]})
}

pub async fn run(mcp: &Mcp, alice: &str, bob: &str, charlie: &str) -> Result<()> {
    for _ in 0..2 {
        mcp.hosts(alice, inventory("alice")).await?;
        mcp.hosts(bob, inventory("bob")).await?;
        mcp.hosts(charlie, json!({"hosts":[]})).await?;
    }
    tokio::try_join!(
        mcp.hosts(alice, inventory("alice")),
        mcp.hosts(bob, inventory("bob")),
        mcp.hosts(charlie, json!({"hosts":[]})),
        mcp.hosts(alice, inventory("alice"))
    )?;
    let result = mcp
        .request(
            Some(bob),
            Method::POST,
            "tools/call",
            json!({"name":"hosts","arguments":{}}),
            200,
            &[("mcp-session-id", "attacker-supplied-session")],
        )
        .await?;
    let actual: Value = serde_json::from_str(
        result["result"]["content"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing host response"))?,
    )?;
    anyhow::ensure!(actual == inventory("bob"));
    Ok(())
}
