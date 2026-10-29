//! A user must never see another user's private host or grant list.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::inventory::{HostsResponse, allowed_hosts};
use crate::config::Host;

#[test]
fn allowlist_is_exact_and_denies_everything_else() {
    let hosts = vec![
        Host {
            id: "a".into(),
            description: "one".into(),
            allowed_subjects: vec!["alice".into()],
        },
        Host {
            id: "b".into(),
            description: "two".into(),
            allowed_subjects: vec!["bob".into()],
        },
        Host {
            id: "c".into(),
            description: "three".into(),
            allowed_subjects: vec![],
        },
    ];
    assert_eq!(
        allowed_hosts(&hosts, "alice")
            .iter()
            .map(|h| h.id)
            .collect::<Vec<_>>(),
        vec!["a"]
    );
    assert_eq!(
        allowed_hosts(&hosts, "bob")
            .iter()
            .map(|h| h.id)
            .collect::<Vec<_>>(),
        vec!["b"]
    );
    assert!(allowed_hosts(&hosts, "charlie").is_empty());
    assert!(allowed_hosts(&hosts, "ali").is_empty());
    let json = serde_json::to_value(HostsResponse {
        hosts: allowed_hosts(&hosts, "alice"),
    })
    .unwrap();
    assert_eq!(
        json,
        serde_json::json!({"hosts":[{"id":"a","description":"one"}]})
    );
    assert!(!json.to_string().contains("allowed_subjects"));
}
