//! Disposable real OpenSSH plus the actual authenticated MCP binary.
#[allow(dead_code)]
mod mock;
#[path = "ssh/deadlines.rs"]
mod ssh_deadlines;
#[path = "ssh/disconnect.rs"]
mod ssh_disconnect;
#[path = "ssh/fixture.rs"]
mod ssh_fixture;
#[path = "ssh/scenarios.rs"]
mod ssh_scenarios;
#[path = "ssh/security.rs"]
mod ssh_security;
mod support;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires Docker's root-owned disposable OpenSSH daemon"]
async fn audited_ssh_commands_are_isolated_and_fail_closed() -> anyhow::Result<()> {
    let fixture = ssh_fixture::Fixture::start().await?;
    ssh_scenarios::run(&fixture).await
}
