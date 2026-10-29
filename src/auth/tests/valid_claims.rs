//! Accept the legitimate baseline, including Keycloak's audience-array representation.
use super::fixtures::{auth, claims};
#[test]
fn accepts_valid_active_token_with_audience_array_and_exact_scope() {
    let p = auth().verify_test_claims(claims(1000), 1000).unwrap();
    assert_eq!(p.subject, "user-id");
}
