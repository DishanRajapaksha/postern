//! Audit recovery and bounded draining are independent of a live SSH server.
#![allow(clippy::unwrap_used)]
use super::{
    audit::Audit,
    types::{AuditQuery, Record},
};

#[test]
fn durable_intent_recovers_as_unknown_and_is_private() {
    let directory = tempfile::tempdir().unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let audit = Audit::open(directory.path()).unwrap();
    let record = Record {
        command_id: "command".into(),
        session_id: "session".into(),
        actor_id: "alice".into(),
        host_id: "lab".into(),
        command: "arbitrary; text".into(),
        accepted_at: 1,
        started_at: None,
        finished_at: None,
        outcome: "accepted".into(),
        exit_code: None,
        timeout_seconds: None,
        termination_confirmed: None,
        supervisor_result: None,
        stdout_bytes: 0,
        stderr_bytes: 0,
        stdout_truncated: false,
        stderr_truncated: false,
    };
    audit.append(&record).unwrap();
    assert!(
        Audit::open(directory.path()).is_err(),
        "only one writer may recover a store"
    );
    drop(audit);
    let recovered = Audit::open(directory.path()).unwrap();
    let query = AuditQuery {
        command_id: None,
        session_id: None,
        limit: None,
        before: None,
    };
    let rows = recovered.latest(Some("alice"), &query).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1.outcome, "unknown");
    assert!(recovered.latest(Some("bob"), &query).unwrap().is_empty());
}

#[tokio::test]
async fn drains_large_output_without_unbounded_capture() {
    let data = vec![b'x'; 200000];
    let captured = super::output::drain(data.as_slice()).await.unwrap();
    assert_eq!(captured.bytes, 200000);
    assert_eq!(captured.text.len(), 65536);
    assert!(captured.truncated);
}
