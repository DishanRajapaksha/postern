//! Startup configuration fixes the trusted issuer; clients cannot choose one.
use anyhow::{Context, Result, ensure};
use reqwest::Url;
use std::env;
use zeroize::Zeroizing;

pub(super) struct Settings {
    pub(super) endpoint: Url,
    pub(super) issuer: String,
    pub(super) audience: String,
    pub(super) required_scope: String,
    pub(super) client_id: String,
    pub(super) client_secret: Zeroizing<String>,
}

fn required(name: &str) -> Result<String> {
    let value = env::var(name).with_context(|| format!("missing {name}"))?;
    ensure!(
        !value.trim().is_empty() && value == value.trim() && !value.chars().any(char::is_control),
        "{name} is empty or invalid"
    );
    Ok(value)
}

impl Settings {
    pub(super) fn from_environment() -> Result<Self> {
        let issuer = required("POSTERN_KEYCLOAK_ISSUER")?;
        let endpoint = introspection_url(&issuer)?;
        let required_scope = required("POSTERN_KEYCLOAK_SCOPE")?;
        ensure!(
            !required_scope.chars().any(char::is_whitespace),
            "scope must be one token"
        );
        Ok(Self {
            endpoint,
            issuer,
            required_scope,
            audience: required("POSTERN_KEYCLOAK_AUDIENCE")?,
            client_id: required("POSTERN_KEYCLOAK_CLIENT_ID")?,
            // Wipe our owned secret buffer on drop. HTTP/environment copies have their own lifetimes.
            client_secret: Zeroizing::new(required("POSTERN_KEYCLOAK_CLIENT_SECRET")?),
        })
    }
}

pub(super) fn introspection_url(issuer: &str) -> Result<Url> {
    let parsed = Url::parse(issuer).context("invalid Keycloak issuer URL")?;
    ensure!(parsed.scheme() == "https", "Keycloak issuer must use HTTPS");
    ensure!(
        parsed.host_str().is_some()
            && parsed.username().is_empty()
            && parsed.password().is_none()
            && parsed.query().is_none()
            && parsed.fragment().is_none()
            && !parsed.path().ends_with('/'),
        "invalid Keycloak issuer URL"
    );
    ensure!(
        parsed.path_segments().is_some_and(|parts| {
            let parts: Vec<_> = parts.collect();
            parts.len() >= 2
                && parts[parts.len() - 2] == "realms"
                && !parts[parts.len() - 1].is_empty()
        }),
        "issuer must end in /realms/<realm>"
    );
    Url::parse(&format!(
        "{issuer}/protocol/openid-connect/token/introspect"
    ))
    .context("invalid derived introspection URL")
}
