//! First MCP feature: a read-only, explicit, public-metadata host inventory.
//! There are intentionally no SSH, session or command execution tools yet.

use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig},
    tool, tool_handler, tool_router,
};
use serde::Serialize;

use crate::config::Host;

#[derive(Clone)]
pub struct Postern {
    hosts: Arc<Vec<Host>>,
    tool_router: ToolRouter<Self>,
}

#[derive(Serialize)]
struct HostsResponse<'a> {
    hosts: &'a [Host],
}

#[tool_router]
impl Postern {
    pub fn new(hosts: Arc<Vec<Host>>) -> Self {
        Self {
            hosts,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "List configured Postern host IDs and public descriptions. No connection credentials or SSH operations are available."
    )]
    async fn hosts(&self) -> Result<CallToolResult, McpError> {
        let response = HostsResponse {
            hosts: self.hosts.as_slice(),
        };
        let json =
            serde_json::to_string(&response).expect("Host consists only of serialisable strings");
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }
}

#[tool_handler]
impl ServerHandler for Postern {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_instructions("Postern development preview. Read-only host listing only. No SSH sessions or command execution. Not authenticated: loopback testing only.".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_only_the_hosts_tool() {
        let handler = Postern::new(Arc::new(vec![]));
        assert!(handler.tool_router.has_route("hosts"));
        assert_eq!(handler.tool_router.list_all().len(), 1);
    }

    #[test]
    fn response_contains_only_public_fields() {
        let hosts = vec![Host {
            id: "lab".into(),
            description: "Disposable test host".into(),
        }];
        let raw = serde_json::to_value(HostsResponse { hosts: &hosts }).unwrap();
        assert_eq!(
            raw,
            serde_json::json!({"hosts":[{"id":"lab","description":"Disposable test host"}]})
        );
        assert!(raw.to_string().find("password").is_none());
    }
}
