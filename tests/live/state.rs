//! Private provisioning state lets verification clean up exactly the identities it created.
use serde::{Deserialize, Serialize};
pub const ROOT: &str = "/srv/services/postern";
pub const ISSUER: &str = "https://heimdall.talesfrom.earth/keycloak/realms/gont";
#[derive(Serialize, Deserialize)]
pub struct Diagnostic {
    pub id: String,
    pub client: String,
    pub secret: String,
    pub subject: String,
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub owner: String,
    pub browser_flow: String,
    pub diagnostics: Vec<Diagnostic>,
}
