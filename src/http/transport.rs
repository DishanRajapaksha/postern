//! Keep SDK admission aligned with the outer explicit Host/Origin boundary.
use super::Limits;
use rmcp::transport::streamable_http_server::StreamableHttpServerConfig;

impl Limits {
    pub(crate) fn mcp_config(&self) -> StreamableHttpServerConfig {
        let mut hosts = vec!["localhost:8765".to_owned(), "127.0.0.1:8765".to_owned()];
        let mut origins = vec![
            "http://localhost:8765".to_owned(),
            "http://127.0.0.1:8765".to_owned(),
        ];
        if let Some(origin) = &self.public_origin {
            hosts.push(origin.trim_start_matches("https://").to_owned());
            origins.push(origin.clone());
        }
        StreamableHttpServerConfig::default()
            .with_legacy_session_mode(false)
            .with_json_response(true)
            .with_allowed_hosts(hosts)
            .with_allowed_origins(origins)
    }
}
