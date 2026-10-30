//! Measure the native HTTP boundary, with audit IDs and remote overlap evidence.
use crate::{ssh_scenarios::tool, support::client::Mcp};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{collections::HashSet, time::Instant};

async fn execute(mcp: &Mcp, session: &str) -> Result<Value> {
    let result = tool(
        mcp,
        "alice",
        "exec",
        json!({
            "session_id":session,
            "command":"date +%s%N; sleep 1; date +%s%N"
        }),
    )
    .await?;
    ensure!(result["record"]["exit_code"] == 0);
    ensure!(result["record"]["termination_confirmed"] == true);
    Ok(result)
}

fn interval(result: &Value) -> Result<(u64, u64)> {
    let mut lines = result["stdout"].as_str().context("timestamps")?.lines();
    let start = lines.next().context("start")?.parse()?;
    let end = lines.next().context("end")?.parse()?;
    ensure!(end > start && lines.next().is_none());
    Ok((start, end))
}

pub async fn run(mcp: &Mcp, session: &str) -> Result<()> {
    let opened = tool(mcp, "alice", "open_session", json!({"host_id":"lab"})).await?;
    let other = opened["session_id"].as_str().context("benchmark session")?;
    let mut samples = Vec::new();
    let mut ids = HashSet::new();
    for _ in 0..3 {
        let start = Instant::now();
        let first = execute(mcp, session).await?;
        let second = execute(mcp, other).await?;
        let sequential = start.elapsed().as_secs_f64();
        ensure!(interval(&first)?.1 <= interval(&second)?.0);
        let start = Instant::now();
        let (a, b) = tokio::try_join!(execute(mcp, session), execute(mcp, other))?;
        let parallel = start.elapsed().as_secs_f64();
        let (a_start, a_end) = interval(&a)?;
        let (b_start, b_end) = interval(&b)?;
        let overlap = a_end.min(b_end).saturating_sub(a_start.max(b_start));
        // Remote timestamps prove overlap even when a busy CI host adds HTTP delay.
        ensure!(overlap > 500_000_000, "native execution did not overlap");
        for result in [&first, &second, &a, &b] {
            let id = result["record"]["command_id"]
                .as_str()
                .context("command ID")?;
            ensure!(ids.insert(id.to_owned()));
            let audit = tool(mcp, "alice", "audit", json!({"command_id":id})).await?;
            ensure!(audit["records"][0] == result["record"]);
        }
        samples.push(json!({"sequential_seconds":sequential,
            "parallel_seconds":parallel,"overlap_seconds":overlap as f64 / 1e9}));
    }
    println!(
        "NATIVE_BENCHMARK {}",
        json!({"samples":samples,"unique_commands":ids.len()})
    );
    tool(mcp, "alice", "close_session", json!({"session_id":other})).await?;
    Ok(())
}
