//! Bounded tool inputs and explicit observed outcomes; commands are never parsed.
use rmcp::schemars;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct OpenSession {
    pub host_id: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SessionId {
    pub session_id: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Execute {
    pub session_id: String,
    pub command: String,
    pub timeout_seconds: Option<u64>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuditQuery {
    pub command_id: Option<String>,
    pub session_id: Option<String>,
    pub limit: Option<u32>,
    pub before: Option<i64>,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Record {
    pub command_id: String,
    pub session_id: String,
    pub actor_id: String,
    pub host_id: String,
    pub command: String,
    pub accepted_at: u64,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub outcome: String,
    pub exit_code: Option<i32>,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

pub(super) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|t| t.as_secs())
        .unwrap_or(0)
}

pub(super) fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
