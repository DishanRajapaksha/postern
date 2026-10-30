//! Actor-only audit queries with bounded pagination.
use super::{Execution, types::AuditQuery};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
impl Execution {
    pub(crate) fn audit(&self, subject: &str, query: AuditQuery) -> Result<Value> {
        ensure!(
            (1..=100).contains(&query.limit.unwrap_or(20)),
            "audit limit must be 1..100"
        );
        for id in [&query.command_id, &query.session_id].into_iter().flatten() {
            ensure!(id.len() == 36, "invalid audit identifier");
        }
        let actor = format!("{}#{subject}", self.issuer);
        let rows = self.audit.latest(Some(&actor), &query)?;
        Ok(
            json!({"records":rows.iter().map(|(_, r)| r).collect::<Vec<_>>(),
            "next_cursor":rows.last().map(|(seq, _)| seq)}),
        )
    }
}
