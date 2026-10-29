//! Revoked, expired and future-dated tokens must fail even if other claims match.
use super::fixtures::{auth, claims};
#[test]
fn rejects_inactive_expired_not_yet_valid_and_future_issued_tokens() {
    let a = auth();
    let mut c = claims(1000);
    c.active = false;
    assert!(a.verify_test_claims(c, 1000).is_err());
    let mut c = claims(1000);
    c.token_type = Some("Refresh".into());
    assert!(a.verify_test_claims(c, 1000).is_err());
    let mut c = claims(1000);
    c.exp = Some(1000);
    assert!(a.verify_test_claims(c, 1000).is_err());
    let mut c = claims(1000);
    c.exp = None;
    assert!(a.verify_test_claims(c, 1000).is_err());
    let mut c = claims(1000);
    c.nbf = Some(1001);
    assert!(a.verify_test_claims(c, 1000).is_err());
    let mut c = claims(1000);
    c.iat = Some(1001);
    assert!(a.verify_test_claims(c, 1000).is_err());
}
