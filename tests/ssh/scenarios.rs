//! Actual SSH side effects prove dispatch ordering and resource limits.
use crate::{ssh_fixture::Fixture, support::client::Mcp};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};

pub async fn tool(mcp: &Mcp, actor: &str, name: &str, args: Value) -> Result<Value> {
    let result = mcp
        .call(
            actor,
            "tools/call",
            json!({"name":name,"arguments":args}),
            200,
        )
        .await?;
    if result["result"]["isError"] == true {
        return Ok(json!({"denied":true}));
    }
    Ok(serde_json::from_str(
        result["result"]["content"][0]["text"]
            .as_str()
            .context("missing result")?,
    )?)
}

pub async fn run(fixture: &Fixture) -> Result<()> {
    let mcp = Mcp::new()?;
    let tools = mcp.call("alice", "tools/list", json!({}), 200).await?;
    ensure!(tools["result"]["tools"].as_array().context("tools")?.len() == 5);
    let opened = tool(&mcp, "alice", "open_session", json!({"host_id":"lab"})).await?;
    let session = opened["session_id"].as_str().context("SSH did not open")?;
    let exec = |command: &str| json!({"session_id":session,"command":command});
    let result = tool(
        &mcp,
        "alice",
        "exec",
        exec("printf '%s' 'a;b|c'; printf err >&2; exit 7"),
    )
    .await?;
    ensure!(result["record"]["outcome"] == "completed" && result["record"]["exit_code"] == 7);
    ensure!(result["stdout"] == "a;b|c" && result["stderr"] == "err");
    ensure!(tool(&mcp, "bob", "exec", exec("touch /tmp/should-not-run")).await?["denied"] == true);
    let audit = tool(&mcp, "bob", "audit", json!({})).await?;
    ensure!(audit["records"].as_array().context("audit")?.is_empty());
    let large = tool(
        &mcp,
        "alice",
        "exec",
        exec("head -c 200000 /dev/zero | tr '\\0' x"),
    )
    .await?;
    ensure!(large["record"]["stdout_truncated"] == true);
    ensure!(large["stdout"].as_str().context("output")?.len() == 65536);
    let timed = tool(
        &mcp,
        "alice",
        "exec",
        json!({"session_id":session,"command":"sleep 4","timeout_seconds":1}),
    )
    .await?;
    ensure!(timed["record"]["outcome"] == "timed_out");
    let audit = tool(&mcp, "alice", "audit", json!({})).await?;
    ensure!(audit["records"].as_array().context("audit")?.len() == 3);
    crate::ssh_runtime::run(fixture, &mcp, session).await?;
    crate::ssh_deadlines::run(fixture, &mcp, session).await?;
    crate::ssh_security::run(fixture, &mcp, session).await?;
    crate::ssh_disconnect::run(fixture, &mcp).await?;
    crate::ssh_races::run(fixture, &mcp).await?;
    crate::ssh_benchmark::run(&mcp, session).await?;
    // The trigger simulates storage rejecting writes while reads still work.
    let database =
        rusqlite::Connection::open(fixture.directory.path().join("audit/audit.sqlite3"))?;
    database.execute_batch("CREATE TRIGGER reject_writes BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT,'fixture storage failure'); END;")?;
    let marker = fixture.directory.path().join("must-not-exist");
    let failed = tool(
        &mcp,
        "alice",
        "exec",
        exec(&format!("touch {}", marker.display())),
    )
    .await?;
    ensure!(failed["denied"] == true && !marker.exists());
    tool(
        &mcp,
        "alice",
        "close_session",
        json!({"session_id":session}),
    )
    .await?;
    ensure!(tool(&mcp, "alice", "exec", exec("true")).await?["denied"] == true);
    database.execute_batch("DROP TRIGGER reject_writes;")?;
    Ok(())
}
