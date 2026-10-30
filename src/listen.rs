//! Loopback TCP plus an optional private Unix socket for the trusted TLS gateway.
mod permissions;
use anyhow::{Result, ensure};
use axum::Router;
use std::{env, os::unix::fs::FileTypeExt, path::PathBuf};

pub(crate) async fn serve(router: Router) -> Result<()> {
    let tcp = tokio::net::TcpListener::bind("127.0.0.1:8765").await?;
    let Some(socket) = env::var_os("POSTERN_PROXY_SOCKET") else {
        axum::serve(tcp, router)
            .with_graceful_shutdown(shutdown())
            .await?;
        return Ok(());
    };
    ensure!(
        env::var("POSTERN_PUBLIC_PROXY").is_ok_and(|v| v == "true"),
        "proxy socket requires public proxy mode"
    );
    let path = PathBuf::from(socket);
    ensure!(path.is_absolute(), "proxy socket must be absolute");
    if let Ok(metadata) = std::fs::symlink_metadata(&path) {
        ensure!(
            metadata.file_type().is_socket(),
            "proxy path is not a socket"
        );
        std::fs::remove_file(&path)?;
    }
    let unix = tokio::net::UnixListener::bind(&path)?;
    let tcp_router = router.clone();
    permissions::apply(
        &path,
        env::var("POSTERN_PROXY_SOCKET_GROUP").ok().as_deref(),
    )?;
    let result = tokio::try_join!(
        async {
            axum::serve(tcp, tcp_router)
                .with_graceful_shutdown(shutdown())
                .await
        },
        async {
            axum::serve(unix, router)
                .with_graceful_shutdown(shutdown())
                .await
        }
    );
    let _ = std::fs::remove_file(path);
    result?;
    Ok(())
}

async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
}
