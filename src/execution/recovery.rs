//! A runtime lock excludes other gateways before reclaiming crash-left SSH masters.
use anyhow::{Result, ensure};
use std::{
    fs::{self, File, OpenOptions},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};
use tokio::{
    process::Command,
    time::{Duration, timeout},
};

pub(super) async fn prepare(root: &Path) -> Result<File> {
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(root.join("writer.lock"))?;
    lock.try_lock()?;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_name().to_string_lossy().starts_with("s-") {
            continue;
        }
        ensure!(
            entry.file_type()?.is_dir(),
            "unexpected runtime session entry"
        );
        let socket = entry.path().join("c");
        if socket.exists() {
            // The control protocol uses only the private socket; no network fallback.
            let mut command = Command::new("/usr/bin/ssh");
            command
                .args(["-F", "/dev/null", "-S"])
                .arg(&socket)
                .args(["-O", "exit", "127.0.0.1"])
                .kill_on_drop(true);
            let output = timeout(Duration::from_secs(2), command.output()).await??;
            // A stale socket is normal after a crash; refusal must not hide a live master.
            if !output.status.success() {
                let mut check = Command::new("/usr/bin/ssh");
                check
                    .args(["-F", "/dev/null", "-S"])
                    .arg(&socket)
                    .args(["-O", "check", "127.0.0.1"])
                    .kill_on_drop(true);
                let output = timeout(Duration::from_secs(2), check.output()).await??;
                ensure!(
                    !output.status.success(),
                    "orphan SSH master is still active"
                );
            }
        }
        fs::remove_dir_all(entry.path())?;
    }
    Ok(lock)
}
