//! Wrap Axum in real rustls HTTPS so certificate validation remains part of the test.
use super::{Mock, State, handler};
use anyhow::Result;
use axum::{
    Router,
    routing::{get, post},
};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
    service::TowerToHyperService,
};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{
        self, ServerConfig,
        pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
    },
};

pub async fn start(directory: &Path) -> Result<Mock> {
    let certs = CertificateDer::pem_file_iter(directory.join("server.pem"))?
        .collect::<Result<Vec<_>, _>>()?;
    let key = PrivateKeyDer::from_pem_file(directory.join("server-key.pem"))?;
    let tls =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_single_cert(certs, key)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let state = Arc::new(State {
        issuer: format!(
            "https://localhost:{}/realms/test",
            listener.local_addr()?.port()
        ),
        counts: Mutex::default(),
    });
    let app = Router::new()
        .route(
            "/realms/test/protocol/openid-connect/token/introspect",
            post(handler::introspect),
        )
        .route("/redirect", get(handler::redirected))
        .with_state(Arc::clone(&state));
    let acceptor = TlsAcceptor::from(Arc::new(tls));
    let task = tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            let app = app.clone();
            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(socket).await {
                    let _ = Builder::new(TokioExecutor::new())
                        .serve_connection(TokioIo::new(tls), TowerToHyperService::new(app))
                        .await;
                }
            });
        }
    });
    Ok(Mock { state, task })
}
