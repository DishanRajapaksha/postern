//! Inventory regression tests use the same parser as startup.
use super::super::*;
fn config(value: &str) -> anyhow::Result<Config> {
    let c: Config = serde_json::from_str(value)?;
    c.validate()
}
#[test]
fn rejects_host_option_injection_and_unknown_fields() {
    for id in [
        "-oProxyCommand",
        "evil.host",
        "../root",
        "bad host",
        "",
        "UPPER",
        "a/b",
    ] {
        let s = serde_json::json!({"hosts":[{"id":id,"description":"Test","allowed_subjects":[]}]})
            .to_string();
        assert!(config(&s).is_err(), "accepted invalid id: {id}");
    }
    assert!(
            config(r#"{"hosts":[{"id":"test","description":"Test","allowed_subjects":[],"private_key":"/tmp/id"}]}"#)
                .is_err()
        );
    assert!(config(r#"{"hosts":[],"ssh_password":"oops"}"#).is_err());
}

#[test]
fn rejects_unbounded_or_control_character_descriptions() {
    let long = "x".repeat(257);
    for description in [long.as_str(), "\n", "bad\rtext", " spaces "] {
        let s =
                serde_json::json!({"hosts":[{"id":"test","description":description,"allowed_subjects":[]}]}).to_string();
        assert!(config(&s).is_err());
    }
}

#[test]
fn refuses_unrecognised_or_duplicate_grants() {
    assert!(
        config(
            r#"{"hosts":[{"id":"lab","description":"Lab","allowed_subjects":["same","same"]}]}"#
        )
        .is_err()
    );
    assert!(
        config(r#"{"hosts":[{"id":"lab","description":"Lab","allowed_subjects":[""]}]}"#).is_err()
    );
    assert!(config(r#"{"hosts":[{"id":"lab","description":"Lab"}]}"#).is_err());
}
