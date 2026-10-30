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
        (
            "POSTERN_OAUTH_RESOURCE",
            "https://postern.example/mcp".into(),
        ),
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
    support::startup::check(&env).await?;
    let mut server = Postern::start(&env).await?;
    support::discovery::check(&issuer.state.issuer).await?;
    anyhow::ensure!(issuer.count("outage")? == 0);
    // Only static discovery is anonymous. All MCP methods still require a token.
    let client = reqwest::Client::builder().no_proxy().build()?;
    for path in [
        "/.well-known/oauth-protected-resource",
        "/.well-known/oauth-protected-resource/mcp",
    ] {
        let response = client
            .get(format!("http://127.0.0.1:8765{path}"))
            .send()
            .await?;
        anyhow::ensure!(response.status() == 200);
        anyhow::ensure!(response.headers()["cache-control"] == "no-store");
        let meta: serde_json::Value = response.json().await?;
        anyhow::ensure!(meta["resource"] == "https://postern.example/mcp");
        anyhow::ensure!(meta["authorization_servers"][0] == issuer.state.issuer);
        anyhow::ensure!(meta["scopes_supported"][0] == "postern:use");
    }
    mock::scenarios::run(&Mcp::new()?, &issuer).await?;
    server.assert_private_logs(&["fixture-secret"])?;
    Ok(())
}
