//! Postern development preview.
//! It intentionally binds to 127.0.0.1 and has no auth or SSH operations.
//! Do not proxy, tunnel, or expose this experimental server externally.

mod config;
mod mcp;

use std::{env, net::{IpAddr, Ipv4Addr, SocketAddr}, path::Path, sync::Arc};

use anyhow::{Context, Result};
use axum::Router;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

use crate::{config::Config, mcp::Postern};

const DEVELOPMENT_PORT: u16 = 8765;

#[tokio::main]
async fn main() -> Result<()> {
    let config_path = env::var("POSTERN_CONFIG")
        .context("set POSTERN_CONFIG to an explicit JSON inventory file")?;
    let config = Config::load(Path::new(&config_path))?;
    let hosts = Arc::new(config.hosts);

    // No configurable bind address until OIDC request authentication exists.
    let listen = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), DEVELOPMENT_PORT);
    let service = StreamableHttpService::new(
        move || Ok(Postern::new(Arc::clone(&hosts))),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default()
            .with_legacy_session_mode(false)
            .with_json_response(true),
    );
    let router = Router::new().nest_service("/mcp", service);
    let listener = tokio::net::TcpListener::bind(listen).await?;
    eprintln!("Postern development preview on http://{listen}/mcp; read-only, NO AUTH, loopback ONLY; never expose or tunnel it.");
    axum::serve(listener, router)
        .with_graceful_shutdown(async { let _ = tokio::signal::ctrl_c().await; })
        .await?;
    Ok(())
}
