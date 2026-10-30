//! Exclusive writer ownership, SQLite durability and recovery before serving.
use super::*;
use crate::execution::{config::private_dir, types::now};
use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt, path::Path};
impl Audit {
    pub fn open(directory: &Path) -> Result<Self> {
        private_dir(directory)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(directory.join("writer.lock"))?;
        lock.try_lock()?;
        let path = directory.join("audit.sqlite3");
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(&path)?;
        let connection = Connection::open(path)?;
        connection.busy_timeout(std::time::Duration::from_millis(500))?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
            CREATE TABLE IF NOT EXISTS events (
                seq INTEGER PRIMARY KEY, command_id TEXT NOT NULL, actor TEXT NOT NULL,
                session_id TEXT NOT NULL, record TEXT NOT NULL);
            CREATE INDEX IF NOT EXISTS command_events ON events(command_id, seq);",
        )?;
        let audit = Self {
            connection: Mutex::new(connection),
            degraded: AtomicBool::new(false),
            _lock: lock,
        };
        for (_, mut record) in audit.latest(
            None,
            &AuditQuery {
                command_id: None,
                session_id: None,
                limit: Some(u32::MAX),
                before: None,
            },
        )? {
            if matches!(record.outcome.as_str(), "accepted" | "running") {
                record.outcome = "unknown".into();
                record.finished_at = Some(now());
                audit.append(&record)?;
            }
        }
        Ok(audit)
    }
}
