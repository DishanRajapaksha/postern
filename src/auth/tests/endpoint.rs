//! Startup must reject issuer URLs that could misdirect confidential credentials.
use super::super::settings::introspection_url;
#[test]
fn accepts_only_correct_keycloak_issuer_url() {
    assert!(introspection_url("https://example.com/realms/demo").is_ok());
    for url in [
        "http://example.com/realms/demo",
        "https://example.com/realms/demo/",
        "https://example.com/realms/demo?x=1",
        "https://user:pass@example.com/realms/demo",
        "https://example.com/other/demo",
        "file:///realms/demo",
    ] {
        assert!(introspection_url(url).is_err(), "accepted bad issuer {url}");
    }
}
