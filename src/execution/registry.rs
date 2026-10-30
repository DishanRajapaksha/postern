//! Ownership and current host grants apply even while an SSH master remains alive.
use super::{
    Execution,
    session::Session,
    types::{OpenSession, SessionId},
};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::sync::{Arc, Weak, atomic::Ordering};

impl Execution {
    pub(crate) async fn open(&self, subject: &str, args: OpenSession) -> Result<Value> {
        self.authorised(subject, &args.host_id)?;
        ensure!(self.audit.healthy(), "audit unavailable");
        let profile = self
            .profiles
            .get(&args.host_id)
            .ok_or_else(|| anyhow::anyhow!("session or host unavailable"))?
            .clone();
        let permit = Arc::clone(&self.capacity).try_acquire_owned()?;
        let session = Arc::new(Session::open(subject, profile, &self.runtime, permit).await?);
        // Grants can change while connecting; never register a newly revoked session.
        self.authorised(subject, &args.host_id)?;
        let view = serde_json::to_value(session.view())?;
        self.sessions
            .lock()
            .await
            .insert(session.id.clone(), session);
        Ok(view)
    }

    pub(super) async fn owned(&self, subject: &str, id: &str) -> Result<Arc<Session>> {
        ensure!(id.len() == 36, "session unavailable");
        let session = self
            .sessions
            .lock()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("session unavailable"))?;
        ensure!(
            session.subject == subject && session.active(),
            "session unavailable"
        );
        self.authorised(subject, &session.profile.host_id)?;
        Ok(session)
    }

    pub(crate) async fn close(&self, subject: &str, args: SessionId) -> Result<Value> {
        let session = self.owned(subject, &args.session_id).await?;
        {
            let _gate = session
                .dispatch_gate
                .lock()
                .map_err(|_| anyhow::anyhow!("session unavailable"))?;
            session.closed.store(true, Ordering::SeqCst);
        }
        self.sessions.lock().await.remove(&args.session_id);
        // In-flight commands drain under their own timeout; no new dispatch is allowed.
        if let Ok(_permit) = session.busy.try_acquire() {
            let _ = session.master.lock().await.kill().await;
        }
        Ok(json!({"closed":true}))
    }
}

pub(super) fn reap(execution: Weak<Execution>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            let Some(execution) = execution.upgrade() else {
                break;
            };
            execution.sessions.lock().await.retain(|_, session| {
                let active = session.active()
                    && execution
                        .authorised(&session.subject, &session.profile.host_id)
                        .is_ok();
                if !active {
                    session.closed.store(true, Ordering::SeqCst);
                }
                active
            });
        }
    });
}
