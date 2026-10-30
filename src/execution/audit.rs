//! Append-only SQLite transitions. FULL commits precede dispatch; restart means unknown.
mod store;
use super::types::{AuditQuery, Record};
use anyhow::{Result, ensure};
use rusqlite::{Connection, params};
use std::{
    fs::File,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

pub(super) struct Audit {
    connection: Mutex<Connection>,
    degraded: AtomicBool,
    _lock: File,
}

impl Audit {
    pub fn healthy(&self) -> bool {
        !self.degraded.load(Ordering::SeqCst)
    }

    pub fn append(&self, record: &Record) -> Result<()> {
        ensure!(self.healthy(), "audit storage degraded");
        let result = (|| -> Result<()> {
            let connection = self
                .connection
                .lock()
                .map_err(|_| anyhow::anyhow!("audit lock unavailable"))?;
            connection.execute(
                "INSERT INTO events(command_id,actor,session_id,record) VALUES (?1,?2,?3,?4)",
                params![
                    record.command_id,
                    record.actor_id,
                    record.session_id,
                    serde_json::to_string(record)?
                ],
            )?;
            Ok(())
        })();
        if result.is_err() {
            self.degraded.store(true, Ordering::SeqCst);
        }
        result
    }

    pub fn latest(&self, actor: Option<&str>, query: &AuditQuery) -> Result<Vec<(i64, Record)>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("audit lock unavailable"))?;
        let mut statement = connection.prepare(
            "SELECT seq, record FROM events e
            WHERE seq=(SELECT MAX(seq) FROM events WHERE command_id=e.command_id)
            AND (?1 IS NULL OR actor=?1) AND (?2 IS NULL OR command_id=?2)
            AND (?3 IS NULL OR session_id=?3) AND (?4 IS NULL OR seq<?4)
            ORDER BY seq DESC LIMIT ?5",
        )?;
        let rows = statement.query_map(
            params![
                actor,
                query.command_id,
                query.session_id,
                query.before,
                query.limit.unwrap_or(20)
            ],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
        )?;
        rows.map(|r| {
            let (seq, text) = r?;
            Ok((seq, serde_json::from_str(&text)?))
        })
        .collect()
    }
}
