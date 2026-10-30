//! Closure and the last authorization check serialize with the actual process spawn.
use crate::execution::{
    Execution,
    session::Session,
    ssh,
    types::{Record, now},
};
use anyhow::Result;
use std::process::Stdio;

pub(super) fn start(
    execution: &Execution,
    session: &Session,
    record: &mut Record,
) -> Result<Option<tokio::process::Child>> {
    let _gate = session
        .dispatch_gate
        .lock()
        .map_err(|_| anyhow::anyhow!("session unavailable"))?;
    if !session.active()
        || execution
            .authorised(&session.subject, &record.host_id)
            .is_err()
    {
        record.outcome = "cancelled".into();
        record.finished_at = Some(now());
        execution.audit.append(record)?;
        return Ok(None);
    }
    record.started_at = Some(now());
    record.outcome = "running".into();
    execution.audit.append(record)?;
    let mut command = ssh::command(&session.profile, &session.directory.path().join("c"));
    // A lost master must never fall back to a new transport.
    command.args(["-o", "ProxyCommand=false"]);
    ssh::destination(&mut command, &session.profile);
    command
        .arg(&record.command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    match command.spawn() {
        Ok(child) => Ok(Some(child)),
        Err(_) => {
            record.outcome = "transport_error".into();
            record.finished_at = Some(now());
            execution.audit.append(record)?;
            Ok(None)
        }
    }
}
