//! Only the configured issuer, audience, scope and a valid subject may grant access.
use super::super::claims::Audience;
use super::fixtures::{auth, claims};
#[test]
fn rejects_wrong_audience_issuer_scope_or_identity() {
    let a = auth();
    let mut c = claims(1000);
    c.aud = Some(Audience::One("other".into()));
    assert!(a.verify_test_claims(c, 1000).is_err());
    let mut c = claims(1000);
    c.iss = Some("https://evil.example/realms/other".into());
    assert!(a.verify_test_claims(c, 1000).is_err());
    let mut c = claims(1000);
    c.scope = Some("postern:use-extra".into());
    assert!(a.verify_test_claims(c, 1000).is_err());
    let mut c = claims(1000);
    c.sub = Some("".into());
    assert!(a.verify_test_claims(c, 1000).is_err());
    let mut c = claims(1000);
    c.sub = None;
    assert!(a.verify_test_claims(c, 1000).is_err());
}

#[test]
fn whitespace_and_control_characters_cannot_become_an_identity() {
    for subject in [" ", " user-id", "user-id ", "user\nid"] {
        let mut c = claims(1000);
        c.sub = Some(subject.into());
        assert!(auth().verify_test_claims(c, 1000).is_err());
    }
}
