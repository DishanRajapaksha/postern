//! Keep test configuration explicit and independent from the host shell environment.
use super::{ISSUER, SECRET};
use crate::support::process::Environment;

pub fn configured() -> Environment {
    [
        ("POSTERN_CONFIG", "/fixtures/hosts.json"),
        ("POSTERN_KEYCLOAK_ISSUER", ISSUER),
        ("POSTERN_KEYCLOAK_AUDIENCE", "postern"),
        ("POSTERN_OAUTH_RESOURCE", "https://postern.example/mcp"),
        ("POSTERN_KEYCLOAK_SCOPE", "postern:use"),
        ("POSTERN_KEYCLOAK_CLIENT_ID", "postern-test"),
        ("POSTERN_KEYCLOAK_CLIENT_SECRET", SECRET),
        ("POSTERN_KEYCLOAK_CA_FILE", "/fixtures/ca.pem"),
        ("HTTPS_PROXY", "http://127.0.0.1:1"),
        ("ALL_PROXY", "http://127.0.0.1:1"),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v.into()))
    .collect()
}
