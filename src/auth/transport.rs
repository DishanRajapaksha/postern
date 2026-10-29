//! Contact only the configured HTTPS issuer. Bound time and memory, then fail closed.
use super::{AuthError, claims::Introspection, settings::Settings};
use anyhow::{Context, Result};
use reqwest::{Client, StatusCode, redirect::Policy};
use std::{env, time::Duration};

pub(super) fn client() -> Result<Client> {
    let mut builder = Client::builder()
        .no_proxy()
        .https_only(true)
        .min_tls_version(reqwest::tls::Version::TLS_1_2)
        .redirect(Policy::none())
        .timeout(Duration::from_secs(5))
        .connect_timeout(Duration::from_secs(5));
    if let Some(path) = env::var_os("POSTERN_KEYCLOAK_CA_FILE") {
        let cert = crate::config::read_bounded(std::path::Path::new(&path), 64 * 1024)?;
        builder = builder.add_root_certificate(
            reqwest::Certificate::from_pem(&cert).context("invalid Keycloak CA PEM")?,
        );
    }
    builder.build().context("cannot build Keycloak HTTP client")
}

pub(super) async fn introspect(
    client: &Client,
    settings: &Settings,
    token: &str,
) -> Result<Introspection, AuthError> {
    let mut response = client
        .post(settings.endpoint.clone())
        .basic_auth(&settings.client_id, Some(settings.client_secret.as_str()))
        .form(&[("token", token), ("token_type_hint", "access_token")])
        .send()
        .await
        .map_err(|_| AuthError::KeycloakUnavailable)?;
    // Do not log upstream errors, token bytes or credentials, even in debug builds.
    if response.status() != StatusCode::OK {
        return Err(AuthError::KeycloakUnavailable);
    }
    let mut payload = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| AuthError::KeycloakUnavailable)?
    {
        if payload.len().saturating_add(chunk.len()) > 16 * 1024 {
            return Err(AuthError::KeycloakUnavailable);
        }
        payload.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&payload).map_err(|_| AuthError::KeycloakUnavailable)
}
