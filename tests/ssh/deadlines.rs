//! Delayed side effects and escaped process groups must die with the cgroup.
use crate::{ssh_fixture::Fixture, ssh_scenarios::tool, support::client::Mcp};
use anyhow::{Context, Result, ensure};
use serde_json::json;
use std::time::{Duration, Instant};

pub async fn run(fixture: &Fixture, mcp: &Mcp, session: &str) -> Result<()> {
    let root = fixture.directory.path();
    let marker = root.join("deadline-marker");
    let commands = [
        format!("sleep 3; touch {}", marker.display()),
        format!("trap '' TERM; sleep 4; touch {}", marker.display()),
        format!(
            "setsid sh -c 'trap \"\" TERM; sleep 4; touch {}' & wait",
            marker.display()
        ),
        // ExitType=cgroup retains even a background child after its parent exits.
        format!("(sleep 3; touch {}) >/dev/null 2>&1 &", marker.display()),
    ];
    for command in commands {
        let result = tool(
            mcp,
            "alice",
            "exec",
            json!({
                "session_id":session,"command":command,"timeout_seconds":1
            }),
        )
        .await?;
        ensure!(result["record"]["outcome"] == "timed_out", "{result}");
        ensure!(result["record"]["termination_confirmed"] == true);
        ensure!(result["record"]["supervisor_result"] == "timeout");
        tokio::time::sleep(Duration::from_secs(4)).await;
        ensure!(!marker.exists(), "timed-out descendant wrote a marker");
    }
    for status in [0, 23, 255] {
        let result = tool(
            mcp,
            "alice",
            "exec",
            json!({
                "session_id":session,"command":format!("exit {status}")
            }),
        )
        .await?;
        ensure!(result["record"]["outcome"] == "completed");
        ensure!(result["record"]["exit_code"] == status);
    }
    let second = tool(mcp, "alice", "open_session", json!({"host_id":"lab"})).await?;
    let other = second["session_id"].as_str().context("second session")?;
    let start = Instant::now();
    let command = |session| json!({"session_id":session,"command":"sleep 3"});
    let (a, b) = tokio::join!(
        tool(mcp, "alice", "exec", command(session)),
        tool(mcp, "alice", "exec", command(other))
    );
    ensure!(a?["record"]["exit_code"] == 0 && b?["record"]["exit_code"] == 0);
    ensure!(
        start.elapsed() < Duration::from_secs(5),
        "backend serialized independent sessions"
    );
    tool(mcp, "alice", "close_session", json!({"session_id":other})).await?;
    Ok(())
}
