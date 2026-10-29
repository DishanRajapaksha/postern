//! All real issuer requests verify the generated CA and bypass ambient proxies.
use super::{ISSUER, SECRET};
use anyhow::{Context, Result, ensure};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}
pub struct Issuer {
    client: Client,
}
impl Issuer {
    pub fn new() -> Result<Self> {
        let cert = std::fs::read("/fixtures/ca.pem")?;
        Ok(Self {
            client: Client::builder()
                .no_proxy()
                .timeout(std::time::Duration::from_secs(5))
                .add_root_certificate(reqwest::Certificate::from_pem(&cert)?)
                .build()?,
        })
    }

    pub async fn ready(&self) -> Result<()> {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(180);
        loop {
            if let Ok(response) = self
                .client
                .get(format!("{ISSUER}/.well-known/openid-configuration"))
                .send()
                .await
                && response.status().is_success()
            {
                let discovery: Value = response.json().await?;
                ensure!(discovery["issuer"] == ISSUER);
                return Ok(());
            }
            ensure!(
                tokio::time::Instant::now() < deadline,
                "Keycloak HTTPS readiness timed out"
            );
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    pub async fn post(
        &self,
        endpoint: &str,
        client: &str,
        fields: &[(&str, &str)],
    ) -> Result<Value> {
        let response = self
            .client
            .post(format!("{ISSUER}/protocol/openid-connect/{endpoint}"))
            .basic_auth(client, Some(SECRET))
            .form(fields)
            .send()
            .await?;
        ensure!(
            response.status().is_success(),
            "Keycloak {endpoint} returned {}",
            response.status()
        );
        let bytes = response.bytes().await?;
        if bytes.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_slice(&bytes).context("invalid issuer JSON")
    }

    pub async fn token(&self, user: &str, client: &str, scope: bool) -> Result<Tokens> {
        let mut fields = vec![
            ("grant_type", "password"),
            ("username", user),
            ("password", "fixture-password"),
        ];
        if scope {
            fields.push(("scope", "postern:use"));
        }
        Ok(serde_json::from_value(
            self.post("token", client, &fields).await?,
        )?)
    }
}
