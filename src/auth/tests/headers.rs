//! Ambiguous or malformed credential headers must not reach the issuer.
use super::super::bearer;
use axum::http::HeaderMap;
#[test]
fn rejects_missing_duplicate_malformed_headers() {
    let mut h = HeaderMap::new();
    assert!(bearer::read(&h).is_err());
    h.insert("Authorization", "Basic something".parse().unwrap());
    assert!(bearer::read(&h).is_err());
    h.insert("Authorization", "Bearer abc".parse().unwrap());
    assert_eq!(bearer::read(&h).unwrap(), "abc");
    h.append("Authorization", "Bearer def".parse().unwrap());
    assert!(bearer::read(&h).is_err());
    h.clear();
    h.insert("Authorization", "Bearer abc,def".parse().unwrap());
    assert!(bearer::read(&h).is_err());
}

#[test]
fn accepts_case_insensitive_scheme_but_rejects_invalid_token_bytes() {
    let mut h = HeaderMap::new();
    h.insert("authorization", "bEaReR abc.def_123-~+/=".parse().unwrap());
    assert_eq!(bearer::read(&h).unwrap(), "abc.def_123-~+/=");
    for value in [
        "Bearer ",
        "Bearer  two-spaces",
        "Bearer abc:def",
        "Bearer abc def",
    ] {
        h.insert("authorization", value.parse().unwrap());
        assert!(bearer::read(&h).is_err());
    }
    h.insert(
        "authorization",
        format!("Bearer {}", "x".repeat(8193)).parse().unwrap(),
    );
    assert!(bearer::read(&h).is_err());
}
