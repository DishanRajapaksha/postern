//! Opt-in SSH execution. Sessions and audit storage are shared across MCP requests.
mod audit;
mod config;
mod directory;
mod output;
mod query;
mod recovery;
mod registry;
mod runner;
mod session;
mod ssh;
#[cfg(test)]
mod tests;
mod types;

use crate::config::{Config, Host};
use anyhow::{Result, ensure};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, Semaphore};
pub(crate) use types::{AuditQuery, Execute, OpenSession, SessionId};

pub(crate) struct Execution {
    inventory: PathBuf,
    profiles: HashMap<String, config::Profile>,
    sessions: Mutex<HashMap<String, Arc<session::Session>>>,
    capacity: Arc<Semaphore>,
    audit: Arc<audit::Audit>,
    runtime: PathBuf,
    issuer: String,
    _runtime_lock: std::fs::File,
}

impl Execution {
    pub(crate) async fn from_environment(inventory: PathBuf) -> Result<Option<Arc<Self>>> {
        let Some(path) = std::env::var_os("POSTERN_EXECUTION_CONFIG") else {
            return Ok(None);
        };
        let config = config::ExecutionConfig::load(&PathBuf::from(path))?;
        let audit = Arc::new(audit::Audit::open(&config.audit_dir)?);
        let runtime_lock = recovery::prepare(&config.runtime_dir).await?;
        let execution = Arc::new(Self {
            inventory,
            profiles: config
                .profiles
                .into_iter()
                .map(|p| (p.host_id.clone(), p))
                .collect(),
            sessions: Mutex::default(),
            capacity: Arc::new(Semaphore::new(32)),
            audit,
            runtime: config.runtime_dir,
            issuer: std::env::var("POSTERN_KEYCLOAK_ISSUER")?,
            _runtime_lock: runtime_lock,
        });
        registry::reap(Arc::downgrade(&execution));
        Ok(Some(execution))
    }

    pub(crate) fn inventory(&self) -> Result<Vec<Host>> {
        Ok(Config::load(&self.inventory)?.hosts)
    }

    fn authorised(&self, subject: &str, host_id: &str) -> Result<Host> {
        // Reload grants on every operation. A changed/unreadable inventory denies access.
        let config = Config::load(&self.inventory)?;
        let host = config.hosts.into_iter().find(|h| h.id == host_id);
        let host = host.ok_or_else(|| anyhow::anyhow!("session or host unavailable"))?;
        ensure!(
            host.allowed_subjects.iter().any(|s| s == subject),
            "session or host unavailable"
        );
        Ok(host)
    }
}
