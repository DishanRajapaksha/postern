//! Restore final owner-only grants and remove diagnostic clients, even after a failed check.
use super::{
    admin::Admin,
    exercise, files,
    state::{ROOT, State},
};
use anyhow::{Context, Result, ensure};
use reqwest::Method;
use serde_json::{Value, json};

pub async fn run() -> Result<()> {
    let mcp = super::environment::mcp()?;
    let state: State = serde_json::from_slice(&std::fs::read(format!(
        "{ROOT}/private/verification-state.json"
    ))?)?;
    ensure!(state.diagnostics.len() == 3, "incomplete diagnostic setup");
    let admin = Admin::new().await?;
    let result = exercise::run(&state).await;
    // Do not stop cleanup after the first error; make every diagnostic credential unusable.
    let mut failures = vec![];
    for diagnostic in &state.diagnostics {
        if admin
            .call(
                Method::DELETE,
                &format!("/clients/{}", diagnostic.id),
                Value::Null,
            )
            .await
            .is_err()
        {
            failures.push("diagnostic client deletion failed");
        }
    }
    if files::inventory(&state.owner, None).is_err() {
        failures.push("final inventory restore failed");
    }
    if files::restart().is_err() {
        failures.push("service restart failed");
    }
    ensure!(
        failures.is_empty(),
        "cleanup needs attention: {}",
        failures.join(", ")
    );
    let tokens = result?;
    let realm = admin.get("").await?;
    ensure!(
        realm["browserFlow"]
            .as_str()
            .context("missing realm policy")?
            == state.browser_flow
    );
    for token in &tokens {
        mcp.call(token, "tools/list", json!({}), 401).await?;
    }
    std::fs::remove_file(format!("{ROOT}/private/verification-state.json"))?;
    println!(
        "PASS: deployed MCP, real-issuer introspection, isolated host grants and immediate revocation"
    );
    println!(
        "PASS: diagnostic clients removed, owner-only inventory restored, passkey flow unchanged"
    );
    Ok(())
}
