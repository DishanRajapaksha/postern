//! A second gateway must fail before reclaiming a live gateway's SSH masters.
use crate::{
    ssh_fixture::Fixture,
    ssh_scenarios::tool,
    support::{client::Mcp, process},
};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::{fs, time::Duration};

pub async fn run(fixture: &Fixture, mcp: &Mcp, session: &str) -> Result<()> {
    let root = fixture.directory.path();
    let mut config: Value = serde_json::from_slice(&fs::read(root.join("execution.json"))?)?;
    // A different audit directory ensures only the shared runtime lock protects us.
    config["audit_dir"] = json!(root.join("second-audit"));
    let path = root.join("second-execution.json");
    fs::write(&path, serde_json::to_vec(&config)?)?;
    let mut environment = fixture._env.clone();
    environment.insert(
        "POSTERN_EXECUTION_CONFIG".into(),
        path.display().to_string(),
    );
    let mut command = tokio::process::Command::from(process::command(&environment));
    command.kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(3), command.output()).await??;
    ensure!(!output.status.success(), "second runtime owner started");
    ensure!(
        fs::read_dir(root.join("run"))?
            .filter_map(Result::ok)
            .any(|entry| entry.path().join("c").exists()),
        "second gateway reclaimed a live SSH master"
    );
    let result = tool(
        mcp,
        "alice",
        "exec",
        json!({
            "session_id":session,"command":"printf runtime-still-owned"
        }),
    )
    .await?;
    ensure!(result["record"]["exit_code"] == 0);
    ensure!(result["stdout"] == "runtime-still-owned");
    Ok(())
}
