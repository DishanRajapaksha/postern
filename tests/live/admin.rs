//! Master-realm admin credentials are read only on Gont and sent only over loopback.
use anyhow::{Context, Result, ensure};
use reqwest::{Client, Method};
use serde_json::{Value, json};
pub struct Admin {
    client: Client,
    token: String,
}
impl Admin {
    pub async fn new() -> Result<Self> {
        let private: Value = serde_json::from_slice(&std::fs::read(
            "/srv/services/keycloak/runtime.private.json",
        )?)?;
        let env = &private["services"]["keycloak"]["environment"];
        let client = Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(20))
            .build()?;
        let response = client
            .post("http://127.0.0.1:3095/keycloak/realms/master/protocol/openid-connect/token")
            .form(&[
                ("grant_type", "password"),
                ("client_id", "admin-cli"),
                (
                    "username",
                    env["KC_BOOTSTRAP_ADMIN_USERNAME"]
                        .as_str()
                        .context("missing admin user")?,
                ),
                (
                    "password",
                    env["KC_BOOTSTRAP_ADMIN_PASSWORD"]
                        .as_str()
                        .context("missing admin credential")?,
                ),
            ])
            .send()
            .await?;
        ensure!(
            response.status().is_success(),
            "local admin authentication failed"
        );
        let response: Value = response.json().await?;
        Ok(Self {
            client,
            token: response["access_token"]
                .as_str()
                .context("missing admin token")?
                .into(),
        })
    }

    pub async fn call(&self, method: Method, path: &str, data: Value) -> Result<Value> {
        let mut request = self
            .client
            .request(
                method,
                format!("http://127.0.0.1:3095/keycloak/admin/realms/gont{path}"),
            )
            .bearer_auth(&self.token);
        if !data.is_null() {
            request = request.json(&data);
        }
        let response = request.send().await?;
        ensure!(
            response.status().is_success(),
            "Keycloak admin operation failed: {}",
            response.status()
        );
        let body = response.bytes().await?;
        if body.is_empty() {
            return Ok(Value::Null);
        }
        Ok(serde_json::from_slice(&body)?)
    }

    pub async fn get(&self, path: &str) -> Result<Value> {
        self.call(Method::GET, path, json!(null)).await
    }
}
