//! RFC 9728 resource discovery. Metadata is public; MCP calls are not.
#[cfg(test)]
mod tests;
use anyhow::{Context, Result, ensure};
use axum::{Json, Router, extract::State, http::HeaderValue, routing::get};
use reqwest::Url;
use serde::Serialize;
use std::{env, sync::Arc};

#[derive(Clone, Serialize)]
pub(crate) struct Metadata {
    resource: String,
    authorization_servers: Vec<String>,
    scopes_supported: Vec<String>,
    bearer_methods_supported: Vec<String>,
}

pub(crate) struct Discovery {
    pub(crate) challenge: HeaderValue,
    metadata: Metadata,
}
impl Discovery {
    pub(crate) fn from_environment(issuer: &str, scope: &str) -> Result<Self> {
        let resource =
            env::var("POSTERN_OAUTH_RESOURCE").context("missing POSTERN_OAUTH_RESOURCE")?;
        Self::new(&resource, issuer, scope)
    }

    pub(crate) fn new(resource: &str, issuer: &str, scope: &str) -> Result<Self> {
        let url = Url::parse(resource).context("invalid OAuth resource URL")?;
        ensure!(
            url.scheme() == "https"
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
                && url.path() == "/mcp"
                && url.query().is_none()
                && url.fragment().is_none()
                && url.as_str() == resource,
            "OAuth resource must be a canonical HTTPS URL ending in /mcp"
        );
        ensure!(
            !scope.is_empty()
                && scope
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b)),
            "OAuth scope must be a safe single token"
        );
        let metadata_url = format!(
            "{}/.well-known/oauth-protected-resource",
            url.origin().ascii_serialization()
        );
        let challenge = HeaderValue::from_str(&format!(
            "Bearer resource_metadata=\"{metadata_url}\", scope=\"{scope}\""
        ))?;
        Ok(Self {
            challenge,
            metadata: Metadata {
                resource: resource.to_owned(),
                authorization_servers: vec![issuer.to_owned()],
                scopes_supported: vec![scope.to_owned()],
                bearer_methods_supported: vec!["header".into()],
            },
        })
    }

    pub(crate) fn router(state: Arc<Self>) -> Router {
        Router::new()
            .route(
                "/.well-known/oauth-protected-resource",
                get(resource_metadata),
            )
            .route(
                "/.well-known/oauth-protected-resource/mcp",
                get(resource_metadata),
            )
            .with_state(state)
    }
}

async fn resource_metadata(State(state): State<Arc<Discovery>>) -> Json<Metadata> {
    Json(state.metadata.clone())
}
