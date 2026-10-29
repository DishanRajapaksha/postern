//! Authentication has three steps: read the token, ask Keycloak, check its answer.
//! Each request repeats all three steps. An earlier login never grants access.

mod bearer;
mod claims;
mod middleware;
mod settings;
#[cfg(test)]
mod tests;
mod transport;

use axum::http::HeaderMap;
pub(crate) use middleware::require_auth;
use reqwest::Client;
use settings::Settings;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// A verified identity. Only Keycloak's checked response can create this value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Principal {
    pub(crate) subject: String,
}

/// Keep credentials private: this type deliberately does not implement Debug.
pub(crate) struct Authenticator {
    client: Client,
    settings: Settings,
    permits: Arc<Semaphore>,
}

#[derive(Debug)]
pub(crate) enum AuthError {
    Unauthorised,
    KeycloakUnavailable,
}

impl Authenticator {
    pub(crate) fn from_environment() -> anyhow::Result<Self> {
        let settings = Settings::from_environment()?;
        let client = transport::client()?;
        Ok(Self {
            client,
            settings,
            permits: Arc::new(Semaphore::new(32)),
        })
    }

    pub(crate) async fn authenticate(&self, headers: &HeaderMap) -> Result<Principal, AuthError> {
        let token = bearer::read(headers)?;
        // Refuse overload immediately. Do not create an unbounded queue at the issuer.
        let _permit = self
            .permits
            .try_acquire()
            .map_err(|_| AuthError::KeycloakUnavailable)?;
        let claims = transport::introspect(&self.client, &self.settings, token).await?;
        claims.verify(&self.settings, claims::now()?)
    }

    #[cfg(test)]
    fn verify_test_claims(
        &self,
        claims: claims::Introspection,
        now: i64,
    ) -> Result<Principal, AuthError> {
        claims.verify(&self.settings, now)
    }
}
