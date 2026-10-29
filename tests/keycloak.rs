//! A real issuer integration; Docker supplies the isolated realm and HTTPS trust root.
mod realm;
mod support;

#[tokio::test]
#[ignore = "requires the disposable Docker Keycloak realm"]
async fn genuine_keycloak_tokens_obey_the_mcp_security_boundary() -> anyhow::Result<()> {
    let issuer = realm::issuer::Issuer::new()?;
    issuer.ready().await?;
    realm::scenarios::run(&issuer).await?;
    realm::failures::run(&issuer).await?;
    Ok(())
}
