//! Keep the real server alive across identity switches and revocation checks.
use super::{
    ALICE, SECRET, boundary, discovery, environment, isolation, issuer::Issuer, lifecycle,
};
use crate::support::{client::Mcp, process::Postern};
use anyhow::{Result, ensure};

pub async fn run(issuer: &Issuer) -> Result<()> {
    let alice = issuer.token("alice", "postern-test", true).await?;
    let bob = issuer.token("bob", "postern-test", true).await?;
    let charlie = issuer.token("charlie", "postern-test", true).await?;
    let claims = issuer
        .post(
            "token/introspect",
            "postern-test",
            &[("token", &alice.access_token)],
        )
        .await?;
    ensure!(claims["active"] == true && claims["sub"] == ALICE && claims["token_type"] == "Bearer");
    let mut server = Postern::start(&environment::configured()).await?;
    let mcp = Mcp::new()?;
    // Real deployments issue tokens through a different client from the introspector.
    let independent = issuer.token("alice", "postern-caller", true).await?;
    mcp.hosts(&independent.access_token, isolation::inventory("alice"))
        .await?;
    boundary::run(&mcp, &bob.access_token).await?;
    discovery::run(&mcp, &alice.access_token, &bob.access_token).await?;
    isolation::run(
        &mcp,
        &alice.access_token,
        &bob.access_token,
        &charlie.access_token,
    )
    .await?;
    lifecycle::run(issuer, &mcp, &alice, &bob).await?;
    server.assert_private_logs(&[SECRET, &alice.access_token, &bob.access_token])?;
    Ok(())
}
