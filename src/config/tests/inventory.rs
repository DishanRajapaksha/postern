//! Inventory regression tests use the same parser as startup.
use super::super::*;
fn config(value: &str) -> anyhow::Result<Config> {
    let c: Config = serde_json::from_str(value)?;
    c.validate()
}
#[test]
fn accepts_and_sorts_explicit_host_ids() {
    let c = config(r#"{"hosts":[{"id":"z_lab","description":"Secondary","allowed_subjects":[]},{"id":"a-lab","description":"Primary","allowed_subjects":[]}]}"#).unwrap();
    assert_eq!(
        c.hosts.iter().map(|h| h.id.as_str()).collect::<Vec<_>>(),
        vec!["a-lab", "z_lab"]
    );
}

#[test]
fn empty_inventory_is_safe() {
    assert!(config(r#"{"hosts":[]}"#).unwrap().hosts.is_empty());
}

#[test]
fn rejects_duplicate_host_ids() {
    assert!(
            config(
                r#"{"hosts":[{"id":"lab","description":"One","allowed_subjects":[]},{"id":"lab","description":"Two","allowed_subjects":[]}]}"#
            )
            .is_err()
        );
}

#[test]
fn rejects_more_than_64_hosts() {
    let hosts = (0..65)
        .map(|n| Host {
            id: format!("h{n}"),
            description: "Test".into(),
            allowed_subjects: vec![],
        })
        .collect();
    assert!(Config { hosts }.validate().is_err());
}
