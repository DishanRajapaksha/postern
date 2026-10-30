//! Postern authenticated MCP inventory and opt-in audited SSH gateway.
//! Host discovery is gated on fresh Keycloak introspection on every request.
//! TCP stays loopback-only; an explicit private socket connects a trusted TLS proxy.

mod auth;
mod config;
mod execution;
mod http;
mod listen;
mod mcp;
mod oauth;

use std::{env, path::Path, sync::Arc};

use anyhow::{Context, Result};
use axum::{Router, middleware};
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};

use crate::{auth::Authenticator, config::Config, mcp::Postern};

#[tokio::main]
async fn main() -> Result<()> {
    let config_path = env::var("POSTERN_CONFIG")
        .context("set POSTERN_CONFIG to an explicit JSON inventory file")?;
    let config = Config::load(Path::new(&config_path))?;
    // Fail startup unless the Keycloak issuer, audience, client and required
    // scope are configured. Never fall back to anonymous or 'dev' credentials.
    let auth = Arc::new(Authenticator::from_environment()?);
    let execution = execution::Execution::from_environment(config_path.clone().into()).await?;
    let discovery = Arc::clone(&auth.discovery);
    let hosts = Arc::new(config.hosts);

    // No configurable bind address in this feature: restrict to loopback.
    let auth_for_service = Arc::clone(&auth);
    let limits = Arc::new(http::Limits::from_environment(execution.is_some())?);
    let service = StreamableHttpService::new(
        move || {
            Ok(Postern::new(
                Arc::clone(&hosts),
                Arc::clone(&auth_for_service),
                execution.clone(),
            ))
        },
        LocalSessionManager::default().into(),
        // Stateless MCP. No MCP session handles can be stolen or mixed between
        // valid users. SSH sessions are distinct, principal-bound objects.
        limits.mcp_config(),
    );
    let protected =
        Router::new()
            .nest_service("/mcp", service)
            .layer(middleware::from_fn_with_state(
                Arc::clone(&auth),
                auth::require_auth,
            ));
    let router = Router::new()
        .merge(protected)
        // Anonymous access is allowed ONLY to static OAuth metadata.
        .merge(oauth::Discovery::router(discovery))
        // Last-added middleware runs first, before MCP parsing.
        .layer(middleware::from_fn_with_state(limits, http::protect));
    listen::serve(router).await?;
    Ok(())
}
