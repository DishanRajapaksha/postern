//! Private per-session control sockets; dropping a session kills its SSH master.
mod lifecycle;
use super::{
    config::Profile,
    ssh,
    types::{id, now},
};
use anyhow::{Result, ensure};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, AtomicU64},
    time::{Duration, Instant},
};
use tokio::{
    process::Child,
    sync::{Mutex, OwnedSemaphorePermit},
};

pub(super) struct Session {
    pub id: String,
    pub subject: String,
    pub profile: Profile,
    pub created: Instant,
    pub created_at: u64,
    pub touched: AtomicU64,
    pub closed: AtomicBool,
    pub dispatch_gate: std::sync::Mutex<()>,
    pub busy: std::sync::Arc<tokio::sync::Semaphore>,
    pub master: Mutex<Child>,
    pub directory: tempfile::TempDir,
    pub _permit: OwnedSemaphorePermit,
}

impl Session {
    pub async fn open(
        subject: &str,
        profile: Profile,
        root: &Path,
        permit: OwnedSemaphorePermit,
    ) -> Result<Self> {
        let directory = tempfile::Builder::new().prefix("s-").tempdir_in(root)?;
        let socket = directory.path().join("c");
        ensure!(
            socket.as_os_str().len() < 100,
            "runtime path too long for SSH socket"
        );
        let mut command = ssh::command(&profile, &socket);
        command.args(["-M", "-N", "-o", "ControlPersist=no"]);
        ssh::destination(&mut command, &profile);
        let mut master = command.spawn()?;
        let ready = tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                ensure!(master.try_wait()?.is_none(), "SSH connection unavailable");
                if socket.exists() {
                    return Ok::<(), anyhow::Error>(());
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await;
        if !matches!(ready, Ok(Ok(()))) {
            let _ = master.kill().await;
            anyhow::bail!("SSH connection unavailable");
        }
        Ok(Self {
            id: id(),
            subject: subject.into(),
            profile,
            created: Instant::now(),
            created_at: now(),
            touched: AtomicU64::new(0),
            closed: AtomicBool::new(false),
            dispatch_gate: std::sync::Mutex::new(()),
            busy: std::sync::Arc::new(tokio::sync::Semaphore::new(1)),
            master: Mutex::new(master),
            directory,
            _permit: permit,
        })
    }
}
