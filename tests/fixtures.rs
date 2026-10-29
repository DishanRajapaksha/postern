//! Docker's init container generates the certificates and realm before Keycloak starts.
#[path = "realm/claims.rs"]
mod claims;
#[path = "support/tls.rs"]
mod tls;
const ALICE: &str = "00000000-0000-0000-0000-000000000001";
const BOB: &str = "00000000-0000-0000-0000-000000000002";
const CHARLIE: &str = "00000000-0000-0000-0000-000000000003";
const SECRET: &str = "disposable-fixture-secret";

#[test]
#[ignore = "writes only to Docker fixture volumes"]
fn prepare_disposable_keycloak() -> anyhow::Result<()> {
    use std::{fs, path::Path};
    tls::certificates(Path::new("/fixtures"))?;
    fs::write(
        "/realms/postern-test-realm.json",
        serde_json::to_vec(&claims::realm())?,
    )?;
    fs::write(
        "/fixtures/hosts.json",
        serde_json::to_vec(&claims::hosts())?,
    )?;
    Ok(())
}
