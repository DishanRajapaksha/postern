//! Postern authenticated read-only MCP development server.
//! Host discovery is gated on fresh Keycloak introspection on every request.
//! Until TLS/OAuth client discovery and SSH auditing are implemented, loopback only.

mod auth;
mod config;
mod mcp;

use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::Path,
    sync::Arc,
};

use anyhow::{Context, Result};
use axum::{Router, middleware};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

use crate::{auth::Authenticator, config::Config, mcp::Postern};

const DEVELOPMENT_PORT: u16 = 8765;

#[tokio::main]
async fn main() -> Result<()> {
    let config_path = env::var("POSTERN_CONFIG")
        .context("set POSTERN_CONFIG to an explicit JSON inventory file")?;
    let config = Config::load(Path::new(&config_path))?;
    // Fail startup unless the Keycloak issuer, audience, client and required
    // scope are configured. Never fall back to anonymous or 'dev' credentials.
    let auth = Arc::new(Authenticator::from_environment()?);
    let hosts = Arc::new(config.hosts);

    // No configurable bind address in this feature: restrict to loopback.
    let listen = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), DEVELOPMENT_PORT);
    let auth_for_service = Arc::clone(&auth);
    let service = StreamableHttpService::new(
        move || Ok(Postern::new(Arc::clone(&hosts), Arc::clone(&auth_for_service))),
        LocalSessionManager::default().into(),
        // Stateless MCP. No MCP session handles can be stolen or mixed between
        // valid users. Future SSH sessions are distinct, principal-bound objects.
        StreamableHttpServerConfig::default()
            .with_legacy_session_mode(false)
            .with_json_response(true),
    );
    let router = Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(Arc::clone(&auth), auth::require_auth));
    let listener = tokio::net::TcpListener::bind(listen).await?;
    eprintln!(
        "Postern Keycloak-authenticated MCP preview at http://{listen}/mcp; loopback ONLY, NO SSH; do not expose or tunnel it."
    );
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
