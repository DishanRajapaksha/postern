//! Disposable real OpenSSH plus the actual authenticated MCP binary.
#[allow(dead_code)]
mod mock;
#[path = "ssh/benchmark.rs"]
mod ssh_benchmark;
#[path = "ssh/deadlines.rs"]
mod ssh_deadlines;
#[path = "ssh/disconnect.rs"]
mod ssh_disconnect;
#[path = "ssh/finalization.rs"]
mod ssh_finalization;
#[path = "ssh/fixture.rs"]
mod ssh_fixture;
#[path = "ssh/races.rs"]
mod ssh_races;
#[path = "ssh/recovery.rs"]
mod ssh_recovery;
#[path = "ssh/runtime.rs"]
mod ssh_runtime;
#[path = "ssh/scenarios.rs"]
mod ssh_scenarios;
#[path = "ssh/security.rs"]
mod ssh_security;
#[path = "ssh/storage.rs"]
mod ssh_storage;
mod support;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires Docker's root-owned disposable OpenSSH daemon"]
async fn audited_ssh_commands_are_isolated_and_fail_closed() -> anyhow::Result<()> {
    let mut fixture = ssh_fixture::Fixture::start().await?;
    ssh_scenarios::run(&fixture).await?;
    fixture._server.stop()?;
    fixture._server = support::process::Postern::start(&fixture._env).await?;
    ssh_recovery::run(&mut fixture, &support::client::Mcp::new()?).await?;
    ssh_finalization::run(&mut fixture).await?;
    ssh_storage::run(&mut fixture).await
}
