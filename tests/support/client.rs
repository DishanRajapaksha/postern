//! Send MCP over HTTP and assert status, headers and JSON before inspecting tools.
use anyhow::Result;
use reqwest::{
    Client, Method,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde_json::{Value, json};

pub struct Mcp {
    client: Client,
    challenge: String,
    endpoint: String,
}
impl Mcp {
    pub fn new() -> Result<Self> {
        Self::for_resource("https://postern.example/mcp")
    }

    pub fn for_resource(resource: &str) -> Result<Self> {
        Ok(Self {
            endpoint: "http://127.0.0.1:8765/mcp".into(),
            challenge: format!(
                "Bearer resource_metadata=\"{}/.well-known/oauth-protected-resource{}\", scope=\"postern:use\"",
                reqwest::Url::parse(resource)?
                    .origin()
                    .ascii_serialization(),
                reqwest::Url::parse(resource)?.path()
            ),
            client: Client::builder()
                .no_proxy()
                .timeout(std::time::Duration::from_secs(20))
                .build()?,
        })
    }

    pub fn public(resource: &str) -> Result<Self> {
        let mut client = Self::for_resource(resource)?;
        client.endpoint = resource.into();
        Ok(client)
    }

    pub async fn request(
        &self,
        token: Option<&str>,
        verb: Method,
        method: &str,
        params: Value,
        status: u16,
        extra: &[(&str, &str)],
    ) -> Result<Value> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "accept",
            HeaderValue::from_static("application/json, text/event-stream"),
        );
        headers.insert(
            "mcp-protocol-version",
            HeaderValue::from_static("2025-11-25"),
        );
        if let Some(token) = token {
            headers.insert("authorization", format!("Bearer {token}").parse()?);
        }
        for (name, value) in extra {
            headers.append(HeaderName::from_bytes(name.as_bytes())?, value.parse()?);
        }
        let response = self
            .client
            .request(verb, &self.endpoint)
            .headers(headers)
            .json(&json!({"jsonrpc":"2.0", "id":1, "method":method, "params":params}))
            .send()
            .await?;
        super::responses::check(response, status, method, &self.challenge).await
    }

    pub async fn call(
        &self,
        token: &str,
        method: &str,
        params: Value,
        status: u16,
    ) -> Result<Value> {
        self.request(Some(token), Method::POST, method, params, status, &[])
            .await
    }
}
