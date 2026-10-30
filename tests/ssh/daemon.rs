//! Root-only disposable daemon with freshly generated out-of-band host-key pins.
use anyhow::{Result, ensure};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};
pub async fn start(root: &Path) -> Result<tokio::process::Child> {
    for name in ["identity", "host"] {
        ensure!(
            Command::new("ssh-keygen")
                .args(["-q", "-t", "ed25519", "-N", "", "-f"])
                .arg(root.join(name))
                .status()?
                .success()
        );
    }
    let host = fs::read_to_string(root.join("host.pub"))?;
    fs::write(root.join("known_hosts"), format!("[127.0.0.1]:2222 {host}"))?;
    fs::write(
        root.join("sshd_config"),
        format!(
            "Port 2222\nListenAddress 127.0.0.1\nHostKey {}/host\nPidFile {}/pid\nAuthorizedKeysFile {}/identity.pub\nPasswordAuthentication no\nPermitRootLogin prohibit-password\nUsePAM yes\nStrictModes no\n",
            root.display(),
            root.display(),
            root.display()
        ),
    )?;
    fs::create_dir_all("/run/sshd")?;
    let sshd = tokio::process::Command::new("/usr/sbin/sshd")
        .args(["-D", "-e", "-f"])
        .arg(root.join("sshd_config"))
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(root.join("sshd.log"))?)
        .kill_on_drop(true)
        .spawn()?;
    for _ in 0..100 {
        if tokio::net::TcpStream::connect("127.0.0.1:2222")
            .await
            .is_ok()
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    let check = Command::new("/usr/bin/ssh")
        .args([
            "-F",
            "/dev/null",
            "-o",
            "BatchMode=yes",
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
        ])
        .arg(format!(
            "UserKnownHostsFile={}",
            root.join("known_hosts").display()
        ))
        .arg("-i")
        .arg(root.join("identity"))
        .args(["-p", "2222", "root@127.0.0.1", "true"])
        .output()?;
    ensure!(
        check.status.success(),
        "fixture SSH unavailable: {} {}",
        String::from_utf8_lossy(&check.stderr),
        fs::read_to_string(root.join("sshd.log"))?
    );
    Ok(sshd)
}
