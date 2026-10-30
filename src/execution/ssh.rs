//! OpenSSH argv only. Ignore ambient config, agents, proxying and local commands.
use super::config::Profile;
use std::{path::Path, process::Stdio};
use tokio::process::Command;

pub(super) fn command(profile: &Profile, socket: &Path) -> Command {
    let mut command = Command::new("/usr/bin/ssh");
    command
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .args(["-F", "/dev/null", "-T", "-S"])
        .arg(socket)
        .args([
            "-o",
            "BatchMode=yes",
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            "UpdateHostKeys=no",
            "-o",
            "GlobalKnownHostsFile=/dev/null",
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "IdentityAgent=none",
            "-o",
            "ForwardAgent=no",
            "-o",
            "ForwardX11=no",
            "-o",
            "ClearAllForwardings=yes",
            "-o",
            "PermitLocalCommand=no",
            "-o",
            "KnownHostsCommand=none",
            "-o",
            "ConnectTimeout=5",
            "-o",
            "ConnectionAttempts=1",
            "-o",
            "ServerAliveInterval=5",
            "-o",
            "ServerAliveCountMax=2",
            "-o",
            "LogLevel=ERROR",
            "-o",
            "PasswordAuthentication=no",
            "-o",
            "KbdInteractiveAuthentication=no",
        ])
        .arg("-o")
        .arg(format!(
            "UserKnownHostsFile={}",
            profile.known_hosts_file.display()
        ))
        .arg("-i")
        .arg(&profile.identity_file)
        .arg("-l")
        .arg(&profile.username)
        .arg("-p")
        .arg(profile.port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    command
}

pub(super) fn destination(command: &mut Command, profile: &Profile) {
    command.arg("--").arg(&profile.hostname);
}
