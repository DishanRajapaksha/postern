//! A failed final audit append preserves observed output but disables new execution.
use crate::{
    ssh_fixture::Fixture,
    ssh_races::started,
    ssh_scenarios::tool,
    support::{client::Mcp, process::Postern},
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::fs;

pub async fn run(fixture: &mut Fixture) -> Result<()> {
    let root = fixture.directory.path();
    let config: Value = serde_json::from_slice(&fs::read(root.join("execution.json"))?)?;
    let database = rusqlite::Connection::open(
        std::path::Path::new(config["audit_dir"].as_str().context("audit directory")?)
            .join("audit.sqlite3"),
    )?;
    let mcp = Mcp::new()?;
    let opened = tool(&mcp, "alice", "open_session", json!({"host_id":"lab"})).await?;
    let session = opened["session_id"]
        .as_str()
        .context("finalization session")?;
    let ready = root.join("finalization-started");
    let execute = tool(
        &mcp,
        "alice",
        "exec",
        json!({
            "session_id":session,
            "command":format!("touch {}; sleep 1; printf observed; exit 23", ready.display())
        }),
    );
    let fail = async {
        started(&ready).await?;
        database.execute_batch(
            "CREATE TRIGGER reject_final BEFORE INSERT ON events
            WHEN json_extract(NEW.record, '$.outcome') NOT IN ('accepted', 'running')
            BEGIN SELECT RAISE(ABORT,'fixture finalization failure'); END;",
        )?;
        Ok::<_, anyhow::Error>(())
    };
    let (result, ()) = tokio::try_join!(execute, fail)?;
    ensure!(result["audit_degraded"] == true);
    ensure!(result["record"]["exit_code"] == 23 && result["stdout"] == "observed");
    let denied = tool(
        &mcp,
        "alice",
        "exec",
        json!({"session_id":session,"command":"true"}),
    )
    .await?;
    ensure!(denied["denied"] == true);
    database.execute_batch("DROP TRIGGER reject_final;")?;
    fixture._server.stop()?;
    fixture._server = Postern::start(&fixture._env).await?;
    let fresh = Mcp::new()?;
    let audit = tool(
        &fresh,
        "alice",
        "audit",
        json!({
            "command_id":result["record"]["command_id"]
        }),
    )
    .await?;
    ensure!(audit["records"][0]["outcome"] == "unknown");
    // The observed response was valid; durable recovery cannot invent that missing result.
    ensure!(audit["records"][0]["exit_code"].is_null());
    Ok(())
}
