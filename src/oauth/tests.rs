//! Discovery identifiers cannot be controlled by HTTP callers.
#![allow(clippy::unwrap_used)]
use super::Discovery;

#[test]
fn advertises_keycloak_with_a_correct_bearer_challenge() {
    let oauth = Discovery::new(
        "https://postern.example.org/mcp",
        "https://keycloak.example.org/realms/home",
        "postern:use",
    )
    .unwrap();
    let json = serde_json::to_value(&oauth.metadata).unwrap();
    assert_eq!(json["resource"], "https://postern.example.org/mcp");
    assert_eq!(
        json["authorization_servers"][0],
        "https://keycloak.example.org/realms/home"
    );
    assert_eq!(json["scopes_supported"][0], "postern:use");
    assert_eq!(json["bearer_methods_supported"][0], "header");
    assert_eq!(
        oauth.challenge.to_str().unwrap(),
        "Bearer resource_metadata=\"https://postern.example.org/.well-known/oauth-protected-resource\", scope=\"postern:use\""
    );
}

#[test]
fn rejects_untrusted_or_noncanonical_resource_urls_and_scopes() {
    for url in [
        "http://postern.example/mcp",
        "https://postern.example/",
        "https://postern.example/mcp?secret=x",
        "https://postern.example/mcp#x",
        "https://u:p@postern.example/mcp",
        "https://postern.example/mcp/",
        "file:///mcp",
        "https://postern.example/mcp/../mcp",
    ] {
        assert!(
            Discovery::new(url, "https://sso.example/realms/home", "postern:use").is_err(),
            "accepted {url}"
        );
    }
    for scope in [
        "",
        "postern:use\";evil",
        "postern:use other",
        "postern:use\n",
    ] {
        assert!(
            Discovery::new(
                "https://postern.example/mcp",
                "https://sso.example/realms/home",
                scope
            )
            .is_err()
        );
    }
}
