//! Closing a busy session blocks new work while the dispatched cgroup drains.
use crate::{ssh_fixture::Fixture, ssh_scenarios::tool, support::client::Mcp};
use anyhow::{Context, Result, ensure};
use serde_json::json;
use std::{path::Path, time::Duration};

pub async fn started(marker: &Path) -> Result<()> {
    for _ in 0..200 {
        if marker.exists() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    anyhow::bail!("remote command did not start")
}

pub async fn run(fixture: &Fixture, mcp: &Mcp) -> Result<()> {
    let opened = tool(mcp, "alice", "open_session", json!({"host_id":"lab"})).await?;
    let session = opened["session_id"].as_str().context("race session")?;
    let root = fixture.directory.path();
    let ready = root.join("race-started");
    let marker = root.join("race-survivor");
    let command = format!(
        "touch {}; trap '' TERM; sleep 4; touch {}",
        ready.display(),
        marker.display()
    );
    let execute = tool(
        mcp,
        "alice",
        "exec",
        json!({
            "session_id":session,"command":command,"timeout_seconds":1
        }),
    );
    let close = async {
        started(&ready).await?;
        tool(mcp, "alice", "close_session", json!({"session_id":session})).await?;
        let denied = tool(
            mcp,
            "alice",
            "exec",
            json!({
                "session_id":session,"command":format!("touch {}", marker.display())
            }),
        )
        .await?;
        ensure!(denied["denied"] == true);
        Ok::<_, anyhow::Error>(())
    };
    let (result, ()) = tokio::try_join!(execute, close)?;
    ensure!(result["record"]["outcome"] == "timed_out");
    ensure!(result["record"]["termination_confirmed"] == true);
    tokio::time::sleep(Duration::from_secs(4)).await;
    ensure!(!marker.exists(), "closed session left a surviving command");
    let audit = tool(
        mcp,
        "alice",
        "audit",
        json!({
            "command_id":result["record"]["command_id"]
        }),
    )
    .await?;
    ensure!(audit["records"][0] == result["record"]);
    let opened = tool(mcp, "alice", "open_session", json!({"host_id":"lab"})).await?;
    let session = opened["session_id"]
        .as_str()
        .context("deadline race session")?;
    for delay in ["0.8", "1.0", "1.2"] {
        let result = tool(
            mcp,
            "alice",
            "exec",
            json!({
                "session_id":session,"command":format!("sleep {delay}; exit 23"),
                "timeout_seconds":1
            }),
        )
        .await?;
        let record = &result["record"];
        ensure!(record["termination_confirmed"] == true);
        match record["outcome"].as_str() {
            Some("completed") => ensure!(record["exit_code"] == 23),
            Some("timed_out") => {
                ensure!(record["exit_code"].is_null() && record["supervisor_result"] == "timeout")
            }
            _ => anyhow::bail!("deadline race fabricated an outcome: {record}"),
        }
    }
    tool(mcp, "alice", "close_session", json!({"session_id":session})).await?;
    Ok(())
}
