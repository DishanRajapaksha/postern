//! A successful HTTPS request is not proof of authorisation. Check every claim.
use super::{AuthError, Principal, settings::Settings};
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum Audience {
    One(String),
    Many(Vec<String>),
}

impl Audience {
    fn matches(&self, expected: &str) -> bool {
        match self {
            Self::One(v) => v == expected,
            Self::Many(v) => v.iter().any(|a| a == expected),
        }
    }
}

#[derive(Deserialize)]
pub(super) struct Introspection {
    pub(super) active: bool,
    pub(super) token_type: Option<String>,
    pub(super) sub: Option<String>,
    pub(super) iss: Option<String>,
    pub(super) aud: Option<Audience>,
    pub(super) exp: Option<i64>,
    pub(super) nbf: Option<i64>,
    pub(super) iat: Option<i64>,
    pub(super) scope: Option<String>,
}

pub(super) fn now() -> Result<i64, AuthError> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AuthError::KeycloakUnavailable)?
        .as_secs();
    i64::try_from(seconds).map_err(|_| AuthError::KeycloakUnavailable)
}

impl Introspection {
    pub(super) fn verify(self, settings: &Settings, now: i64) -> Result<Principal, AuthError> {
        // Missing security claims deny access. A refresh token is never an access token.
        if !self.active
            || self.token_type.as_deref() != Some("Bearer")
            || self.iss.as_deref() != Some(settings.issuer.as_str())
            || !self
                .aud
                .as_ref()
                .is_some_and(|a| a.matches(&settings.audience))
            || self.exp.is_none_or(|exp| exp <= now)
            || self.nbf.is_some_and(|nbf| nbf > now)
            || self.iat.is_some_and(|iat| iat > now)
            || !self.scope.as_deref().is_some_and(|scopes| {
                scopes
                    .split_ascii_whitespace()
                    .any(|s| s == settings.required_scope)
            })
        {
            return Err(AuthError::Unauthorised);
        }
        let subject = self.sub.ok_or(AuthError::Unauthorised)?;
        if subject.trim().is_empty()
            || subject != subject.trim()
            || subject.len() > 256
            || subject.chars().any(char::is_control)
        {
            return Err(AuthError::Unauthorised);
        }
        Ok(Principal { subject })
    }
}
