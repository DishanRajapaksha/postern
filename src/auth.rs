//! Keycloak OAuth2 access-token introspection.
//! Every incoming MCP HTTP request is checked with Keycloak, without caching.
//! Any missing/invalid response or upstream outage fails closed.

use std::{
    env, fs,
    path::Path,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, ensure};
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use reqwest::{Client, Url, redirect::Policy};
use serde::Deserialize;

const MAX_BEARER_BYTES: usize = 8192;
const MAX_INTROSPECTION_BYTES: usize = 16 * 1024;
const REQUEST_TIMEOUT_SECONDS: u64 = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Principal {
    pub subject: String,
}

#[derive(Clone)]
pub struct Authenticator {
    client: Client,
    endpoint: Url,
    issuer: String,
    audience: String,
    required_scope: String,
    client_id: String,
    client_secret: String,
}

#[derive(Debug)]
pub enum AuthError {
    Unauthorised,
    KeycloakUnavailable,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Audience {
    One(String),
    Many(Vec<String>),
}

impl Audience {
    fn matches(&self, audience: &str) -> bool {
        match self {
            Self::One(value) => value == audience,
            Self::Many(values) => values.iter().any(|value| value == audience),
        }
    }
}

#[derive(Deserialize)]
struct Introspection {
    active: bool,
    sub: Option<String>,
    iss: Option<String>,
    aud: Option<Audience>,
    exp: Option<i64>,
    nbf: Option<i64>,
    iat: Option<i64>,
    scope: Option<String>,
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name).with_context(|| format!("missing {name}"))?;
    ensure!(
        !value.trim().is_empty() && value == value.trim(),
        "{name} is empty or invalid"
    );
    Ok(value)
}

fn introspection_url(issuer: &str) -> Result<Url> {
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
        parsed.path_segments().is_some_and(|mut segments| {
            let parts: Vec<_> = segments.by_ref().collect();
            parts.len() >= 2
                && parts[parts.len() - 2] == "realms"
                && !parts[parts.len() - 1].is_empty()
        }),
        "issuer must be a Keycloak realm URL ending in /realms/<realm>"
    );
    // Never follow a URL supplied by an MCP client: this endpoint is startup configuration.
    Url::parse(&format!(
        "{issuer}/protocol/openid-connect/token/introspect"
    ))
    .context("invalid derived introspection URL")
}

impl Authenticator {
    pub fn from_environment() -> Result<Self> {
        let issuer = required_env("POSTERN_KEYCLOAK_ISSUER")?;
        let endpoint = introspection_url(&issuer)?;
        let audience = required_env("POSTERN_KEYCLOAK_AUDIENCE")?;
        let required_scope = required_env("POSTERN_KEYCLOAK_SCOPE")?;
        ensure!(
            !required_scope.chars().any(char::is_whitespace),
            "required scope must be one token"
        );
        let client_id = required_env("POSTERN_KEYCLOAK_CLIENT_ID")?;
        let client_secret = required_env("POSTERN_KEYCLOAK_CLIENT_SECRET")?;
        let mut builder = Client::builder()
            .https_only(true)
            .redirect(Policy::none())
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
            .connect_timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS));
        if let Ok(path) = env::var("POSTERN_KEYCLOAK_CA_FILE") {
            ensure!(!path.is_empty(), "empty CA certificate path");
            let cert = fs::read(Path::new(&path)).context("cannot read Keycloak CA file")?;
            builder = builder.add_root_certificate(
                reqwest::Certificate::from_pem(&cert).context("invalid Keycloak CA PEM")?,
            );
        }
        let client = builder
            .build()
            .context("cannot build Keycloak HTTP client")?;
        Ok(Self {
            client,
            endpoint,
            issuer,
            audience,
            required_scope,
            client_id,
            client_secret,
        })
    }

    fn read_bearer<'a>(&self, headers: &'a HeaderMap) -> Result<&'a str, AuthError> {
        let mut auth_headers = headers.get_all(axum::http::header::AUTHORIZATION).iter();
        let first = auth_headers.next().ok_or(AuthError::Unauthorised)?;
        if auth_headers.next().is_some() {
            return Err(AuthError::Unauthorised);
        }
        let header = first.to_str().map_err(|_| AuthError::Unauthorised)?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or(AuthError::Unauthorised)?;
        if token.is_empty()
            || token.len() > MAX_BEARER_BYTES
            || !token.bytes().all(|c| c.is_ascii_graphic())
            || token.contains(',')
        {
            return Err(AuthError::Unauthorised);
        }
        Ok(token)
    }

    fn verify_claims(&self, claims: Introspection, now: i64) -> Result<Principal, AuthError> {
        if !claims.active
            || claims.iss.as_deref() != Some(self.issuer.as_str())
            || !claims
                .aud
                .as_ref()
                .is_some_and(|a| a.matches(&self.audience))
            || claims.exp.is_none_or(|exp| exp <= now)
            || claims.nbf.is_some_and(|nbf| nbf > now)
            || claims.iat.is_some_and(|iat| iat > now)
            || !claims.scope.as_deref().is_some_and(|scope| {
                scope
                    .split_ascii_whitespace()
                    .any(|s| s == self.required_scope)
            })
        {
            return Err(AuthError::Unauthorised);
        }
        let sub = claims.sub.ok_or(AuthError::Unauthorised)?;
        if sub.is_empty() || sub.len() > 256 || sub.chars().any(char::is_control) {
            return Err(AuthError::Unauthorised);
        }
        Ok(Principal { subject: sub })
    }

    pub async fn authenticate(&self, headers: &HeaderMap) -> Result<Principal, AuthError> {
        let token = self.read_bearer(headers)?;
        let mut response = self
            .client
            .post(self.endpoint.clone())
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(&[("token", token), ("token_type_hint", "access_token")])
            .send()
            .await
            .map_err(|err| {
                #[cfg(debug_assertions)]
                eprintln!("Keycloak HTTPS introspection transport failed: {err}");
                AuthError::KeycloakUnavailable
            })?;
        if !response.status().is_success() {
            return Err(AuthError::KeycloakUnavailable);
        }
        let mut payload = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| AuthError::KeycloakUnavailable)?
        {
            if payload.len().saturating_add(chunk.len()) > MAX_INTROSPECTION_BYTES {
                return Err(AuthError::KeycloakUnavailable);
            }
            payload.extend_from_slice(&chunk);
        }
        let claims: Introspection =
            serde_json::from_slice(&payload).map_err(|_| AuthError::KeycloakUnavailable)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| AuthError::KeycloakUnavailable)?
            .as_secs() as i64;
        self.verify_claims(claims, now)
    }
}

/// Protect the complete /mcp subtree, including initialisation, discovery,
/// tool calls, notifications, GET and DELETE requests. No unauthenticated
/// HTTP traffic may be passed to rmcp.
pub async fn require_auth(
    State(auth): State<Arc<Authenticator>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    match auth.authenticate(request.headers()).await {
        Ok(_) => next.run(request).await,
        Err(AuthError::Unauthorised) => StatusCode::UNAUTHORIZED.into_response(),
        Err(AuthError::KeycloakUnavailable) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn auth() -> Authenticator {
        Authenticator {
            client: Client::new(),
            endpoint: Url::parse(
                "https://keycloak.example/realms/postern/protocol/openid-connect/token/introspect",
            )
            .unwrap(),
            issuer: "https://keycloak.example/realms/postern".into(),
            audience: "postern".into(),
            required_scope: "postern:use".into(),
            client_id: "postern-resource".into(),
            client_secret: "fixture-only".into(),
        }
    }

    fn claims(now: i64) -> Introspection {
        Introspection {
            active: true,
            sub: Some("user-id".into()),
            iss: Some("https://keycloak.example/realms/postern".into()),
            aud: Some(Audience::Many(vec!["other".into(), "postern".into()])),
            exp: Some(now + 120),
            nbf: Some(now - 5),
            iat: Some(now - 10),
            scope: Some("profile postern:use".into()),
        }
    }

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

    #[test]
    fn accepts_valid_active_token_with_audience_array_and_exact_scope() {
        let p = auth().verify_claims(claims(1000), 1000).unwrap();
        assert_eq!(p.subject, "user-id");
    }

    #[test]
    fn rejects_inactive_expired_not_yet_valid_and_future_issued_tokens() {
        let a = auth();
        let mut c = claims(1000);
        c.active = false;
        assert!(a.verify_claims(c, 1000).is_err());
        let mut c = claims(1000);
        c.exp = Some(1000);
        assert!(a.verify_claims(c, 1000).is_err());
        let mut c = claims(1000);
        c.exp = None;
        assert!(a.verify_claims(c, 1000).is_err());
        let mut c = claims(1000);
        c.nbf = Some(1001);
        assert!(a.verify_claims(c, 1000).is_err());
        let mut c = claims(1000);
        c.iat = Some(1001);
        assert!(a.verify_claims(c, 1000).is_err());
    }

    #[test]
    fn rejects_wrong_audience_issuer_scope_or_identity() {
        let a = auth();
        let mut c = claims(1000);
        c.aud = Some(Audience::One("other".into()));
        assert!(a.verify_claims(c, 1000).is_err());
        let mut c = claims(1000);
        c.iss = Some("https://evil.example/realms/other".into());
        assert!(a.verify_claims(c, 1000).is_err());
        let mut c = claims(1000);
        c.scope = Some("postern:use-extra".into());
        assert!(a.verify_claims(c, 1000).is_err());
        let mut c = claims(1000);
        c.sub = Some("".into());
        assert!(a.verify_claims(c, 1000).is_err());
        let mut c = claims(1000);
        c.sub = None;
        assert!(a.verify_claims(c, 1000).is_err());
    }

    #[test]
    fn rejects_missing_duplicate_malformed_headers() {
        let a = auth();
        let mut h = HeaderMap::new();
        assert!(a.read_bearer(&h).is_err());
        h.insert("Authorization", "Basic something".parse().unwrap());
        assert!(a.read_bearer(&h).is_err());
        h.insert("Authorization", "Bearer abc".parse().unwrap());
        assert_eq!(a.read_bearer(&h).unwrap(), "abc");
        h.append("Authorization", "Bearer def".parse().unwrap());
        assert!(a.read_bearer(&h).is_err());
        h.clear();
        h.insert("Authorization", "Bearer abc,def".parse().unwrap());
        assert!(a.read_bearer(&h).is_err());
    }
}
