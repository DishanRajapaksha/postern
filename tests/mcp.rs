//! Run the actual binary against a temporary HTTPS issuer. No Docker is required.
mod mock;
mod support;
use anyhow::Result;
use support::{
    client::Mcp,
    process::{Environment, Postern},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mcp_rechecks_tokens_and_never_leaks_another_subjects_hosts() -> Result<()> {
    let directory = tempfile::tempdir()?;
    support::tls::certificates(directory.path())?;
    let issuer = mock::start(directory.path()).await?;
    let mut env = Environment::new();
    for (name, value) in [
        (
            "POSTERN_CONFIG",
            format!("{}/config/postern.example.json", env!("CARGO_MANIFEST_DIR")),
        ),
        ("POSTERN_KEYCLOAK_ISSUER", issuer.state.issuer.clone()),
        ("POSTERN_KEYCLOAK_AUDIENCE", "postern".into()),
        ("POSTERN_KEYCLOAK_SCOPE", "postern:use".into()),
        ("POSTERN_KEYCLOAK_CLIENT_ID", "postern-test".into()),
        ("POSTERN_KEYCLOAK_CLIENT_SECRET", "fixture-secret".into()),
        (
            "POSTERN_KEYCLOAK_CA_FILE",
            directory.path().join("ca.pem").to_string_lossy().into(),
        ),
    ] {
        env.insert(name.into(), value);
    }
    let mut server = Postern::start(&env).await?;
    mock::scenarios::run(&Mcp::new()?, &issuer).await?;
    server.assert_private_logs(&["fixture-secret"])?;
    Ok(())
}
