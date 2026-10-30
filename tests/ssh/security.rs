//! A genuine host-key mismatch and changed grants block new work.
use crate::{ssh_fixture::Fixture, ssh_scenarios::tool, support::client::Mcp};
use anyhow::{Result, ensure};
use serde_json::json;

pub async fn run(fixture: &Fixture, mcp: &Mcp, session: &str) -> Result<()> {
    let socket = reqwest::Client::builder()
        .no_proxy()
        .unix_socket(fixture.directory.path().join("http.sock"))
        .build()?;
    let response = socket
        .get("http://postern.example/.well-known/oauth-protected-resource/mcp")
        .send()
        .await?;
    ensure!(
        response.status() == 200,
        "public Host through private socket rejected"
    );
    ensure!(
        socket
            .post("http://postern.example/mcp")
            .send()
            .await?
            .status()
            == 401
    );
    ensure!(
        socket
            .get("http://attacker.example/.well-known/oauth-protected-resource/mcp")
            .send()
            .await?
            .status()
            == 403
    );
    ensure!(
        tool(mcp, "bob", "close_session", json!({"session_id":session})).await?["denied"] == true
    );
    let path = fixture.directory.path().join("known_hosts");
    let correct = std::fs::read(&path)?;
    let wrong = std::fs::read_to_string(fixture.directory.path().join("identity.pub"))?;
    std::fs::write(&path, format!("[127.0.0.1]:2222 {wrong}"))?;
    ensure!(tool(mcp, "alice", "open_session", json!({"host_id":"lab"})).await?["denied"] == true);
    std::fs::write(&path, correct)?;
    let grants = fixture.directory.path().join("hosts.json");
    let saved = std::fs::read(&grants)?;
    std::fs::write(&grants, b"{\"hosts\":[]}")?;
    ensure!(
        tool(
            mcp,
            "alice",
            "exec",
            json!({"session_id":session,"command":"true"})
        )
        .await?["denied"]
            == true
    );
    std::fs::write(grants, saved)?;
    Ok(())
}
