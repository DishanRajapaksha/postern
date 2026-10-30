//! Target-owned systemd cgroups enforce deadlines even after SSH disconnects.
mod observe;
use crate::execution::{session::Session, ssh};
use anyhow::Result;
use std::process::Stdio;

pub(super) struct Observed {
    pub outcome: &'static str,
    pub exit_code: Option<i32>,
    pub result: String,
}

fn environment() -> &'static str {
    "export XDG_RUNTIME_DIR=/run/user/$(id -u); export DBUS_SESSION_BUS_ADDRESS=unix:path=$XDG_RUNTIME_DIR/bus; "
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub(super) fn launch(id: &str, command: &str, seconds: u64) -> String {
    // Never fall back to an unsupervised shell on unsupported targets.
    format!(
        "{}test -r /sys/fs/cgroup/cgroup.controllers || exit 125; shell=$(getent passwd \"$(id -u)\" | cut -d: -f7); test -x \"$shell\" || exit 125; exec systemd-run --user --quiet --pipe --wait --unit=postern-{id} \
         --service-type=exec --property=RuntimeMaxSec={seconds}s \
         --property=TimeoutStopSec=1s --property=KillMode=control-group \
         --property=SendSIGKILL=yes --property=ExitType=cgroup \
         --property=RemainAfterExit=yes \"$shell\" -c {}",
        environment(),
        // systemd expands dollars in ExecStart argv; preserve the shell text verbatim.
        quote(&command.replace('$', "$$"))
    )
}

pub(super) async fn observe(session: &Session, id: &str, seconds: u64) -> Result<Observed> {
    let mut command = ssh::command(&session.profile, &session.directory.path().join("c"));
    command.args(["-o", "ProxyCommand=false"]);
    ssh::destination(&mut command, &session.profile);
    let response = command
        .arg(format!("{}{}", environment(), observe::script(id, seconds)))
        .stdout(Stdio::piped())
        .output()
        .await?;
    anyhow::ensure!(response.status.success(), "remote termination unconfirmed");
    observe::parse(&response.stdout)
}
