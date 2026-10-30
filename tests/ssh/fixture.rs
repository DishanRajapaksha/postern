//! Generate fixture keys independently, pin the server key, never trust a network scan.
#[path = "daemon.rs"]
mod daemon;
#[path = "environment.rs"]
mod environment;
use crate::{
    mock,
    support::{
        process::{Environment, Postern},
        tls,
    },
};
use anyhow::Result;
use environment::environment;
use serde_json::json;
use std::fs;

pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub _env: Environment,
    pub _server: Postern,
    pub _issuer: mock::Mock,
    _sshd: tokio::process::Child,
}

impl Fixture {
    pub async fn start() -> Result<Self> {
        let directory = tempfile::tempdir()?;
        let root = directory.path();
        tls::certificates(root)?;
        let issuer = mock::start(root).await?;
        let sshd = daemon::start(root).await?;
        fs::write(
            root.join("hosts.json"),
            serde_json::to_vec(&json!({"hosts":[{
            "id":"lab", "description":"SSH fixture", "allowed_subjects":[
            "00000000-0000-0000-0000-000000000001", "other-user"]}]}))?,
        )?;
        fs::write(
            root.join("execution.json"),
            serde_json::to_vec(&json!({
            "audit_dir":root.join("audit"), "runtime_dir":root.join("run"), "profiles":[{
                "host_id":"lab", "hostname":"127.0.0.1", "username":"root", "port":2222,
                "identity_file":root.join("identity"), "known_hosts_file":root.join("known_hosts")}]}))?,
        )?;
        let env = environment(root, &issuer.state.issuer);
        let server = Postern::start(&env).await?;
        Ok(Self {
            directory,
            _env: env,
            _server: server,
            _issuer: issuer,
            _sshd: sshd,
        })
    }
}
