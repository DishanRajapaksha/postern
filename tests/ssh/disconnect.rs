//! Losing SSH cannot cancel the target timer or certify termination.
use crate::{ssh_fixture::Fixture, ssh_scenarios::tool, support::client::Mcp};
use anyhow::{Context, Result, ensure};
use serde_json::json;
use std::{collections::HashSet, fs, time::Duration};

pub async fn run(fixture: &Fixture, mcp: &Mcp) -> Result<()> {
    let root = fixture.directory.path();
    let paths = || -> Result<HashSet<_>> {
        Ok(fs::read_dir(root.join("run"))?
            .map(|e| e.map(|e| e.path()))
            .collect::<std::io::Result<_>>()?)
    };
    let before = paths()?;
    let opened = tool(mcp, "alice", "open_session", json!({"host_id":"lab"})).await?;
    let session = opened["session_id"]
        .as_str()
        .context("disconnect session")?;
    let directory = paths()?
        .difference(&before)
        .next()
        .context("socket directory")?
        .clone();
    let started = root.join("started");
    let marker = root.join("abandoned-marker");
    let command = format!(
        "touch {}; sleep 4; touch {}",
        started.display(),
        marker.display()
    );
    let execute = tool(
        mcp,
        "alice",
        "exec",
        json!({"session_id":session,"command":command,"timeout_seconds":1}),
    );
    let disconnect = async {
        for _ in 0..100 {
            if started.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        ensure!(started.exists(), "remote command did not start");
        let status = tokio::process::Command::new("/usr/bin/ssh")
            .args(["-F", "/dev/null", "-S"])
            .arg(directory.join("c"))
            .args(["-O", "exit", "127.0.0.1"])
            .output()
            .await?;
        ensure!(status.status.success(), "master shutdown failed");
        Ok::<_, anyhow::Error>(())
    };
    let (result, disconnected) = tokio::join!(execute, disconnect);
    disconnected?;
    let result = result?;
    ensure!(result["record"]["outcome"] == "unknown");
    ensure!(result["record"]["termination_confirmed"] == false);
    tokio::time::sleep(Duration::from_secs(5)).await;
    ensure!(!marker.exists(), "SSH loss abandoned unsupervised work");
    tool(mcp, "alice", "close_session", json!({"session_id":session})).await?;
    Ok(())
}
