//! Send MCP over HTTP and assert status, headers and JSON before inspecting tools.
use anyhow::{Result, ensure};
use reqwest::{
    Client, Method,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde_json::{Value, json};

pub struct Mcp {
    client: Client,
}
impl Mcp {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .no_proxy()
                .timeout(std::time::Duration::from_secs(20))
                .build()?,
        })
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
            .request(verb, "http://127.0.0.1:8765/mcp")
            .headers(headers)
            .json(&json!({"jsonrpc":"2.0", "id":1, "method":method, "params":params}))
            .send()
            .await?;
        super::responses::check(response, status, method).await
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

    pub async fn hosts(&self, token: &str, expected: Value) -> Result<()> {
        let result = self
            .call(
                token,
                "tools/call",
                json!({"name":"hosts","arguments":{}}),
                200,
            )
            .await?;
        ensure!(
            result.get("error").is_none() && result["result"]["isError"] != true,
            "hosts failed"
        );
        let text = result["result"]["content"][0]["text"]
            .as_str()
            .context("missing hosts text")?;
        ensure!(
            serde_json::from_str::<Value>(text)? == expected,
            "unexpected host inventory"
        );
        Ok(())
    }
}
use anyhow::Context;
