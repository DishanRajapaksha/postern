//! Create secret files with their final permissions before writing any secret bytes.
use super::state::ROOT;
use anyhow::{Result, ensure};
use serde_json::json;
use std::os::unix::fs::PermissionsExt;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
};

pub fn private(name: &str, bytes: &[u8]) -> Result<()> {
    let path = format!("{ROOT}/private/{name}");
    let temporary = format!("{path}.new");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(temporary, path)?;
    Ok(())
}

pub fn inventory(owner: &str, diagnostics: Option<(&str, &str)>) -> Result<()> {
    let mut hosts = vec![
        json!({"id":"gont", "description":"Gont inventory preview", "allowed_subjects":[owner]}),
    ];
    if let Some((alice, bob)) = diagnostics {
        hosts.push(json!({"id":"diagnostic-private", "description":"Temporary private test", "allowed_subjects":[alice]}));
        hosts.push(json!({"id":"diagnostic-shared", "description":"Temporary shared test", "allowed_subjects":[alice,bob]}));
    }
    fs::write(
        format!("{ROOT}/config/hosts.json"),
        serde_json::to_vec_pretty(&json!({"hosts":hosts}))?,
    )?;
    let status = std::process::Command::new("chown")
        .args(["root:postern", &format!("{ROOT}/config/hosts.json")])
        .status()?;
    ensure!(status.success(), "inventory ownership failed");
    fs::set_permissions(
        format!("{ROOT}/config/hosts.json"),
        fs::Permissions::from_mode(0o640),
    )?;
    Ok(())
}

pub fn restart() -> Result<()> {
    ensure!(
        std::process::Command::new("systemctl")
            .args(["restart", "postern.service"])
            .status()?
            .success()
    );
    for _ in 0..100 {
        if std::net::TcpStream::connect("127.0.0.1:8765").is_ok() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    anyhow::bail!("installed service did not become ready")
}
