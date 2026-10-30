//! Explicit test process environment, independent of shell state.
use crate::support::process::Environment;
use std::path::Path;
pub fn environment(root: &Path, issuer: &str) -> Environment {
    [
        ("POSTERN_PUBLIC_PROXY", "true".into()),
        (
            "POSTERN_PROXY_SOCKET",
            root.join("http.sock").to_string_lossy().into_owned(),
        ),
        (
            "POSTERN_CONFIG",
            root.join("hosts.json").to_string_lossy().into_owned(),
        ),
        (
            "POSTERN_EXECUTION_CONFIG",
            root.join("execution.json").to_string_lossy().into_owned(),
        ),
        ("POSTERN_KEYCLOAK_ISSUER", issuer.into()),
        (
            "POSTERN_OAUTH_RESOURCE",
            "https://postern.example/mcp".into(),
        ),
        ("POSTERN_KEYCLOAK_AUDIENCE", "postern".into()),
        ("POSTERN_KEYCLOAK_SCOPE", "postern:use".into()),
        ("POSTERN_KEYCLOAK_CLIENT_ID", "postern-test".into()),
        ("POSTERN_KEYCLOAK_CLIENT_SECRET", "fixture-secret".into()),
        (
            "POSTERN_KEYCLOAK_CA_FILE",
            root.join("ca.pem").to_string_lossy().into_owned(),
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v))
    .collect()
}
