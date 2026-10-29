//! The only exposed tool lists authorised hosts. No SSH operation is registered.
mod inventory;
#[cfg(test)]
mod tests;

use crate::{
    auth::{Authenticator, Principal},
    config::Host,
};
use inventory::{HostsResponse, allowed_hosts};
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{CallToolResult, ContentBlock},
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct Postern {
    hosts: Arc<Vec<Host>>,
    auth: Arc<Authenticator>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl Postern {
    pub(crate) fn new(hosts: Arc<Vec<Host>>, auth: Arc<Authenticator>) -> Self {
        Self {
            hosts,
            auth,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "List host IDs and descriptions authorised for the current user. No SSH operations."
    )]
    async fn hosts(&self, ctx: RequestContext<RoleServer>) -> Result<CallToolResult, McpError> {
        let denied = || McpError::internal_error("request authentication failed", None);
        let parts = ctx
            .extensions
            .get::<axum::http::request::Parts>()
            .ok_or_else(denied)?;
        // Ask Keycloak again: the token may have been revoked after the HTTP gate.
        let principal = self
            .auth
            .authenticate(&parts.headers)
            .await
            .map_err(|_| denied())?;
        let original = parts.extensions.get::<Principal>().ok_or_else(denied)?;
        // An issuer must not switch subjects between the two checks in one request.
        if original != &principal {
            return Err(denied());
        }
        let response = HostsResponse {
            hosts: allowed_hosts(&self.hosts, &principal.subject),
        };
        let json = serde_json::to_string(&response)
            .map_err(|_| McpError::internal_error("host response unavailable", None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }
}

#[tool_handler(router = self.tool_router, name = "postern", version = "0.1.0",
    instructions = "Read-only host inventory. Fresh Keycloak checks on every request and tool call.")]
impl ServerHandler for Postern {}
