//! A detached bounded task finalises audit even if the HTTP caller disconnects.
mod dispatch;
mod process;
mod supervisor;
use super::{
    Execution,
    types::{Execute, Record, id, now},
};
use anyhow::{Result, ensure};
use process::run;
use serde_json::{Value, json};
use std::sync::Arc;

impl Execution {
    pub(crate) async fn execute(self: &Arc<Self>, subject: &str, args: Execute) -> Result<Value> {
        ensure!(
            !args.command.is_empty() && args.command.len() <= 8192 && !args.command.contains('\0'),
            "command must contain 1..8192 bytes without NUL"
        );
        let seconds = args.timeout_seconds.unwrap_or(10);
        ensure!((1..=60).contains(&seconds), "timeout must be 1..60 seconds");
        let session = self.owned(subject, &args.session_id).await?;
        let permit = Arc::clone(&session.busy).try_acquire_owned()?;
        ensure!(session.active(), "session unavailable");
        self.authorised(subject, &session.profile.host_id)?;
        session.touch();
        let mut record = Record {
            command_id: id(),
            session_id: session.id.clone(),
            actor_id: format!("{}#{subject}", self.issuer),
            host_id: session.profile.host_id.clone(),
            command: args.command,
            accepted_at: now(),
            started_at: None,
            finished_at: None,
            outcome: "accepted".into(),
            exit_code: None,
            timeout_seconds: Some(seconds),
            termination_confirmed: Some(false),
            supervisor_result: None,
            stdout_bytes: 0,
            stderr_bytes: 0,
            stdout_truncated: false,
            stderr_truncated: false,
        };
        self.audit.append(&record)?;
        let execution = Arc::clone(self);
        tokio::spawn(async move {
            let _permit = permit;
            let child = dispatch::start(&execution, &session, &mut record, seconds)?;
            let observed = match child {
                Some(child) => run(child, &session, &record.command_id, seconds).await,
                None => {
                    return Ok(
                        json!({"record":record,"stdout":"","stderr":"","audit_degraded":false}),
                    );
                }
            };
            record.finished_at = Some(now());
            let (stdout, stderr) = match observed {
                Ok((observed, out, err)) => {
                    record.exit_code = observed.exit_code;
                    record.outcome = observed.outcome.into();
                    record.termination_confirmed = Some(true);
                    record.supervisor_result = Some(observed.result);
                    record.stdout_bytes = out.bytes;
                    record.stderr_bytes = err.bytes;
                    record.stdout_truncated = out.truncated;
                    record.stderr_truncated = err.truncated;
                    (out.text, err.text)
                }
                Err(_) => {
                    record.outcome = "unknown".into();
                    (String::new(), String::new())
                }
            };
            let durable = execution.audit.append(&record).is_ok();
            Ok::<_, anyhow::Error>(json!({"record":record,"stdout":stdout,"stderr":stderr,
                "audit_degraded":!durable}))
        })
        .await?
    }
}
