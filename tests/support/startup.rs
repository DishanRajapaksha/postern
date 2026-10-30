//! Mandatory discovery configuration fails before binding, even outside Docker.
use super::process::{Environment, command};
use anyhow::{Result, ensure};

pub async fn check(configured: &Environment) -> Result<()> {
    for value in [
        None,
        Some(""),
        Some("http://postern.example/mcp"),
        Some("https://postern.example/mcp?query=1"),
        Some("https://postern.example/mcp#fragment"),
    ] {
        let mut env = configured.clone();
        env.remove("POSTERN_OAUTH_RESOURCE");
        if let Some(value) = value {
            env.insert("POSTERN_OAUTH_RESOURCE".into(), value.into());
        }
        let mut child = tokio::process::Command::from(command(&env));
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            child.kill_on_drop(true).output(),
        )
        .await??;
        ensure!(
            !output.status.success(),
            "accepted invalid OAuth configuration"
        );
        ensure!(!String::from_utf8_lossy(&output.stderr).contains("fixture-secret"));
    }
    Ok(())
}
