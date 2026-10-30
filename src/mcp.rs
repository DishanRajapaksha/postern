//! Authenticated inventory plus explicitly enabled audited SSH tools.
mod access;
mod execution;
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
    execution: Option<Arc<crate::execution::Execution>>,
}

#[tool_router]
impl Postern {
    pub(crate) fn new(
        hosts: Arc<Vec<Host>>,
        auth: Arc<Authenticator>,
        execution: Option<Arc<crate::execution::Execution>>,
    ) -> Self {
        let mut tool_router = Self::tool_router() + Self::execution_router();
        if execution.is_none() {
            for name in ["open_session", "exec", "close_session", "audit"] {
                tool_router.remove_route(name);
            }
        }
        for route in tool_router.map.values_mut() {
            let mut meta = rmcp::model::MetaObject::new();
            meta.0.insert(
                "securitySchemes".into(),
                serde_json::json!([
                {"type":"oauth2","scopes":[auth.discovery.scope()]}]),
            );
            route.attr.meta = Some(meta);
        }
        Self {
            hosts,
            auth,
            tool_router,
            execution,
        }
    }

    #[tool(
        description = "List host IDs and descriptions authorised for the current user.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true
        )
    )]
    async fn hosts(&self, ctx: RequestContext<RoleServer>) -> Result<CallToolResult, McpError> {
        let principal = self.principal(ctx).await?;
        let refreshed = self
            .execution
            .as_ref()
            .map(|e| e.inventory())
            .transpose()
            .map_err(|_| McpError::internal_error("inventory unavailable", None))?;
        let hosts = refreshed.as_ref().unwrap_or(&self.hosts);
        let response = HostsResponse {
            hosts: allowed_hosts(hosts, &principal.subject),
        };
        let json = serde_json::to_string(&response)
            .map_err(|_| McpError::internal_error("host response unavailable", None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }
}

#[tool_handler(router = self.tool_router, name = "postern", version = "0.1.0",
    instructions = "Fresh Keycloak checks on every request and tool. SSH commands are arbitrary and audited. Never automatically retry execution after a lost response.")]
impl ServerHandler for Postern {}
