//! Real tmpfs exhaustion and read-only remounts, confined to a disposable mount.
#[path = "storage/mount.rs"]
mod mount;
use crate::{
    ssh_fixture::Fixture,
    ssh_scenarios::tool,
    support::{client::Mcp, process::Postern},
};
use anyhow::{Context, Result, ensure};
use mount::{Mount, mount};
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    io::Write,
};

pub async fn run(fixture: &mut Fixture) -> Result<()> {
    fixture._server.stop()?;
    let root = fixture.directory.path();
    let storage = root.join("storage");
    fs::create_dir(&storage)?;
    mount(
        &["-t", "tmpfs", "-o", "size=1m,mode=0700", "tmpfs"],
        &storage,
    )?;
    let _mount = Mount(storage.clone());
    let path = root.join("execution.json");
    let mut config: Value = serde_json::from_slice(&fs::read(&path)?)?;
    config["audit_dir"] = json!(storage);
    fs::write(&path, serde_json::to_vec(&config)?)?;
    fixture._server = Postern::start(&fixture._env).await?;
    let mcp = Mcp::new()?;
    let opened = tool(&mcp, "alice", "open_session", json!({"host_id":"lab"})).await?;
    let session = opened["session_id"].as_str().context("storage session")?;
    let mut filler = File::create(storage.join("filler"))?;
    // A bounded 2 MiB attempt must exhaust the isolated 1 MiB filesystem.
    let error = filler
        .write_all(&vec![0u8; 2 * 1024 * 1024])
        .err()
        .context("tmpfs did not fill")?;
    ensure!(
        error.raw_os_error() == Some(28),
        "expected ENOSPC, got {error}"
    );
    let marker = root.join("storage-must-not-run");
    let result = tool(
        &mcp,
        "alice",
        "exec",
        json!({
            "session_id":session,"command":format!("touch {}", marker.display())
        }),
    )
    .await?;
    ensure!(
        result["denied"] == true && !marker.exists(),
        "disk-full audit launched SSH"
    );
    drop(filler);
    fs::remove_file(storage.join("filler"))?;
    // Removing the cause does not silently re-enable a degraded writer.
    let denied = tool(
        &mcp,
        "alice",
        "exec",
        json!({"session_id":session,"command":"true"}),
    )
    .await?;
    ensure!(denied["denied"] == true);
    fixture._server.stop()?;
    mount(&["-o", "remount,ro"], &storage)?;
    ensure!(
        Postern::start(&fixture._env).await.is_err(),
        "read-only audit startup succeeded"
    );
    mount(&["-o", "remount,rw"], &storage)?;
    fixture._server = Postern::start(&fixture._env).await?;
    let fresh = Mcp::new()?;
    let opened = tool(&fresh, "alice", "open_session", json!({"host_id":"lab"})).await?;
    let result = tool(
        &fresh,
        "alice",
        "exec",
        json!({
            "session_id":opened["session_id"],"command":"exit 23"
        }),
    )
    .await?;
    ensure!(
        result["record"]["exit_code"] == 23,
        "repaired audit did not recover"
    );
    fixture._server.stop()?;
    Ok(())
}
