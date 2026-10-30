//! Kill the gateway after remote dispatch, restore a live snapshot and reject old handles.
use crate::{
    ssh_fixture::Fixture,
    ssh_races::started,
    ssh_scenarios::tool,
    support::{client::Mcp, process::Postern},
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{fs, process::Command, time::Duration};

pub async fn run(fixture: &mut Fixture, mcp: &Mcp) -> Result<()> {
    let root = fixture.directory.path().to_path_buf();
    let opened = tool(mcp, "alice", "open_session", json!({"host_id":"lab"})).await?;
    let session = opened["session_id"].as_str().context("recovery session")?;
    let ready = root.join("crash-started");
    let marker = root.join("crash-survivor");
    let command = format!(
        "touch {}; sleep 5; touch {}",
        ready.display(),
        marker.display()
    );
    let execute = tool(
        mcp,
        "alice",
        "exec",
        json!({
            "session_id":session,"command":command,"timeout_seconds":2
        }),
    );
    let snapshot = root.join("snapshot.sqlite3");
    let crash = async {
        started(&ready).await?;
        let status = Command::new(env!("CARGO_BIN_EXE_postern-audit"))
            .arg("backup")
            .arg(root.join("audit/audit.sqlite3"))
            .arg(&snapshot)
            .status()?;
        ensure!(status.success(), "live audit backup failed");
        fixture._server.stop()?;
        Ok::<_, anyhow::Error>(())
    };
    let (_lost_response, crashed) = tokio::join!(execute, crash);
    crashed?;
    fixture._server = Postern::start(&fixture._env).await?;
    let fresh = Mcp::new()?;
    verify(&fresh, session).await?;
    ensure!(
        fs::read_dir(root.join("run"))?
            .all(|e| e.is_ok_and(|e| !e.file_name().to_string_lossy().starts_with("s-"))),
        "orphan runtime directories remain"
    );
    fixture._server.stop()?;
    let restored = root.join("restored");
    fs::create_dir(&restored)?;
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&restored, fs::Permissions::from_mode(0o700))?;
    ensure!(
        Command::new(env!("CARGO_BIN_EXE_postern-audit"))
            .arg("backup")
            .arg(&snapshot)
            .arg(restored.join("audit.sqlite3"))
            .status()?
            .success()
    );
    let path = root.join("execution.json");
    let mut config: Value = serde_json::from_slice(&fs::read(&path)?)?;
    config["audit_dir"] = json!(restored);
    fs::write(path, serde_json::to_vec(&config)?)?;
    fixture._server = Postern::start(&fixture._env).await?;
    verify(&Mcp::new()?, session).await?;
    tokio::time::sleep(Duration::from_secs(5)).await;
    ensure!(
        !marker.exists(),
        "gateway crash abandoned the remote deadline"
    );
    Ok(())
}

async fn verify(mcp: &Mcp, session: &str) -> Result<()> {
    let denied = tool(
        mcp,
        "alice",
        "exec",
        json!({"session_id":session,"command":"true"}),
    )
    .await?;
    ensure!(denied["denied"] == true, "old session survived restart");
    let audit = tool(mcp, "alice", "audit", json!({"session_id":session})).await?;
    let record = &audit["records"][0];
    ensure!(record["session_id"] == session && record["outcome"] == "unknown");
    ensure!(record["termination_confirmed"] == false);
    Ok(())
}
