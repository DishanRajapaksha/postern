//! Start with valid claims so each test can isolate one reason to reject a token.
use super::super::{
    Authenticator,
    claims::{Audience, Introspection},
    settings::Settings,
};
use reqwest::{Client, Url};
pub(super) fn auth() -> Authenticator {
    Authenticator {
        client: Client::new(),
        settings: Settings {
            endpoint: Url::parse(
                "https://keycloak.example/realms/postern/protocol/openid-connect/token/introspect",
            )
            .unwrap(),
            issuer: "https://keycloak.example/realms/postern".into(),
            audience: "postern".into(),
            required_scope: "postern:use".into(),
            client_id: "postern-resource".into(),
            client_secret: zeroize::Zeroizing::new("fixture-only".into()),
        },
        permits: std::sync::Arc::new(tokio::sync::Semaphore::new(32)),
    }
}

pub(super) fn claims(now: i64) -> Introspection {
    Introspection {
        active: true,
        token_type: Some("Bearer".into()),
        sub: Some("user-id".into()),
        iss: Some("https://keycloak.example/realms/postern".into()),
        aud: Some(Audience::Many(vec!["other".into(), "postern".into()])),
        exp: Some(now + 120),
        nbf: Some(now - 5),
        iat: Some(now - 10),
        scope: Some("profile postern:use".into()),
    }
}
