//! Issuer errors never become anonymous access or bypass startup validation.
use super::{SECRET, environment, issuer::Issuer};
use crate::support::{
    client::Mcp,
    process::{Postern, command},
};
use anyhow::{Result, ensure};
use serde_json::json;

pub async fn run(issuer: &Issuer) -> Result<()> {
    let bob = issuer.token("bob", "postern-test", true).await?;
    for (name, value) in [
        ("POSTERN_KEYCLOAK_CA_FILE", None),
        (
            "POSTERN_KEYCLOAK_CLIENT_SECRET",
            Some("incorrect-fixture-secret"),
        ),
        (
            "POSTERN_KEYCLOAK_ISSUER",
            Some("https://localhost:8444/realms/postern-test"),
        ),
    ] {
        let mut env = environment::configured();
        if let Some(value) = value {
            env.insert(name.into(), value.into());
        } else {
            env.remove(name);
        }
        let mut server = Postern::start(&env).await?;
        Mcp::new()?
            .call(&bob.access_token, "tools/list", json!({}), 503)
            .await?;
        server.assert_private_logs(&[SECRET, &bob.access_token])?;
    }
    for (name, value) in [
        ("POSTERN_KEYCLOAK_CLIENT_SECRET", ""),
        (
            "POSTERN_KEYCLOAK_ISSUER",
            "http://localhost:8443/realms/postern-test",
        ),
        ("POSTERN_CONFIG", "/nonexistent-inventory.json"),
        ("POSTERN_OAUTH_RESOURCE", "http://postern.example/mcp"),
        ("POSTERN_OAUTH_RESOURCE", "https://postern.example/mcp?query=1"),
    ] {
        let mut env = environment::configured();
        env.insert(name.into(), value.into());
        let mut command = tokio::process::Command::from(command(&env));
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            command.kill_on_drop(true).output(),
        )
        .await??;
        ensure!(
            !output.status.success(),
            "invalid configuration was accepted"
        );
        ensure!(!String::from_utf8_lossy(&output.stderr).contains(SECRET));
    }
    // OAuth discovery must be configured before the server can listen.
    let mut env = environment::configured();
    env.remove("POSTERN_OAUTH_RESOURCE");
    let mut child = tokio::process::Command::from(command(&env));
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(10), child.kill_on_drop(true).output(),
    ).await??;
    ensure!(!output.status.success(), "missing OAuth resource was accepted");
    Ok(())
}
